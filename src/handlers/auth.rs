use axum_extra::extract::CookieJar;

use super::*;
use crate::{auth::*, dto::users::UserDto};

pub fn routes(state: Arc<AppState>) -> Router {
    let routes = auth_routes(state.clone());
    let api = Router::new().nest("/auth", routes);
    Router::new().merge(api)
}

pub fn auth_routes(state: Arc<AppState>) -> Router {
    Router::new().route("/login", post(auth)).with_state(state)
}

pub async fn auth(
    jar: CookieJar,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginPayload>,
) -> Result<(CookieJar, Json<UserDto>), ModuleError> {
    let response = auth::service::login(jar, state.pool.clone(), payload).await?;
    Ok((response.0, Json(response.1)))
}
