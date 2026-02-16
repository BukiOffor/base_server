use super::*;
use crate::auth::jwt::Claims;
use crate::dto::organisations::*;

pub fn routes(state: Arc<AppState>) -> Router {
    let routes = org_routes(state.clone());
    let api = Router::new().nest("/organisations", routes);
    Router::new().merge(api)
}

pub fn org_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", post(create_organisation).get(list_organisations))
        .route("/{id}", get(get_organisation))
        .with_state(state)
}

pub async fn create_organisation(
    claims: Claims,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<NewOrganisation>,
) -> Result<Json<Message>, ModuleError> {
    // Only Root can create organisations
    if claims.role != crate::models::users::RoleType::Root {
        return Err(ModuleError::NotAllowed(
            "Only Root can create organisations",
        ));
    }
    let response =
        services::organisations::create_organisation(state.pool.clone(), payload).await?;
    Ok(Json(response))
}

pub async fn list_organisations(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<OrganisationDto>>, ModuleError> {
    let orgs = services::organisations::list_organisations(state.pool.clone()).await?;
    Ok(Json(orgs))
}

pub async fn get_organisation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<OrganisationDto>, ModuleError> {
    let org = services::organisations::get_organisation_by_id(state.pool.clone(), id).await?;
    Ok(Json(org))
}
