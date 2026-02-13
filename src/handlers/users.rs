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
