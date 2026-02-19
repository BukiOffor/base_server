use super::*;
use crate::auth::jwt::Claims;
use crate::dto::projects::*;

#[derive(Deserialize)]
pub struct UpdateVisibilityPayload {
    pub is_public: bool,
}

pub fn routes(state: Arc<AppState>) -> Router {
    let routes = project_routes(state.clone());
    let api = Router::new().nest("/projects", routes);
    Router::new().merge(api)
}

pub fn project_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", post(create_project))
        .route("/org/{org_id}", get(list_projects_by_org))
        .route("/{id}", get(get_project).delete(delete_project))
        .route("/{id}/visibility", patch(update_visibility))
        .with_state(state)
}

pub async fn create_project(
    claims: Claims,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<NewProject>,
) -> Result<Json<ProjectDto>, ModuleError> {
    // Root and Admin can create projects
    if claims.role != crate::models::users::RoleType::Root
        && claims.role != crate::models::users::RoleType::Admin
    {
        return Err(ModuleError::NotAllowed(
            "Only Root and Admins can create projects",
        ));
    }
    let project = services::projects::create_project(state.pool.clone(), payload).await?;
    Ok(Json(project))
}

pub async fn list_projects_by_org(
    State(state): State<Arc<AppState>>,
    Path(org_id): Path<uuid::Uuid>,
) -> Result<Json<Vec<ProjectDto>>, ModuleError> {
    let projects = services::projects::list_projects_by_org(state.pool.clone(), org_id).await?;
    Ok(Json(projects))
}

pub async fn get_project(
    State(state): State<Arc<AppState>>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<ProjectDto>, ModuleError> {
    let project = services::projects::get_project_by_id(state.pool.clone(), id).await?;
    Ok(Json(project))
}

pub async fn update_visibility(
    claims: Claims,
    State(state): State<Arc<AppState>>,
    Path(id): Path<uuid::Uuid>,
    Json(payload): Json<UpdateVisibilityPayload>,
) -> Result<Json<ProjectDto>, ModuleError> {
    // Root and Admin can update projects
    if claims.role != crate::models::users::RoleType::Root
        && claims.role != crate::models::users::RoleType::Admin
    {
        return Err(ModuleError::NotAllowed(
            "Only Root and Admins can update project visibility",
        ));
    }
    let project =
        services::projects::update_project_visibility(state.pool.clone(), id, payload.is_public)
            .await?;
    Ok(Json(project))
}

pub async fn delete_project(
    _claims: Claims,
    State(state): State<Arc<AppState>>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<Message>, ModuleError> {
    let response = services::projects::delete_project(state.pool.clone(), id).await?;
    Ok(Json(response))
}
