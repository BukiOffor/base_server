pub mod auth;
pub mod organisations;
pub mod tours;
pub mod users;
pub use super::*;

pub use axum::{
    extract::{Json, Path, Query, State},
    response::{IntoResponse, Redirect},
    routing::*,
};

pub fn get_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .merge(auth::routes(state.clone()))
        .merge(users::routes(state.clone()))
        .merge(organisations::routes(state.clone()))
        .merge(tours::routes(state.clone()))
}
