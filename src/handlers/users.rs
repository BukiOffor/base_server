use super::*;
use crate::dto::users::*;
use crate::errors::ErrorMessage;

pub fn routes(state: Arc<AppState>) -> Router {
    let routes = user_routes(state.clone());
    let api = Router::new().nest("/users", routes);
    Router::new().merge(api)
}

pub fn user_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/register", post(register_user))
        .route("/org-admin", post(create_org_admin))
        .route("/manager", post(create_manager))
        .with_state(state)
}

#[utoipa::path(
    post,
    path = "/api/v1/users/register",
    request_body = NewUser,
    responses(
        (status = 200, description = "User registered successfully", body = Message),
        (status = 400, description = "Bad request", body = ErrorMessage),
        (status = 401, description = "Unauthorized", body = ErrorMessage),
        (status = 403, description = "Forbidden", body = ErrorMessage),
        (status = 500, description = "Internal server error", body = ErrorMessage)
    ),
    tag = "Users"
)]
pub async fn register_user(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<NewUser>,
) -> Result<Json<Message>, ModuleError> {
    // Placeholder implementation
    let response = services::users::register_user(state.pool.clone(), payload).await?;
    Ok(Json(response))
}

pub async fn create_org_admin(
    claims: crate::auth::jwt::Claims,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<NewUser>,
) -> Result<Json<Message>, ModuleError> {
    // Only Root can create Org Admins
    if claims.role != crate::models::users::RoleType::Root {
        return Err(ModuleError::NotAllowed("Only Root can create Org Admins"));
    }

    let org_id = payload
        .organisation_id
        .ok_or(ModuleError::BadRequest("Organisation ID is required"))?;

    let response = services::users::register_user_with_role(
        state.pool.clone(),
        payload,
        crate::models::users::RoleType::Admin,
        Some(org_id),
    )
    .await?;
    Ok(Json(response))
}

pub async fn create_manager(
    claims: crate::auth::jwt::Claims,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<NewUser>,
) -> Result<Json<Message>, ModuleError> {
    // Admins can create managers for their own organisation
    if claims.role != crate::models::users::RoleType::Admin {
        return Err(ModuleError::NotAllowed("Only Admins can create Managers"));
    }

    let org_id = claims.organisation_id.ok_or(ModuleError::InternalError(
        "Admin without organisation ID".into(),
    ))?;

    let response = services::users::register_user_with_role(
        state.pool.clone(),
        payload,
        crate::models::users::RoleType::Manager,
        Some(org_id),
    )
    .await?;
    Ok(Json(response))
}
