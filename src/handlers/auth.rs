//! Legacy template login is no longer mounted. Roster authentication lives in auth::admin.
use super::*;
use crate::auth::{dto::LoginPayload, jwt::AuthBodyDto};
pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/auth/login", post(auth))
        .with_state(state)
}
pub async fn auth(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginPayload>,
) -> Result<Json<AuthBodyDto>, ModuleError> {
    Ok(Json(
        crate::auth::service::login(state.pool.clone(), payload).await?,
    ))
}
