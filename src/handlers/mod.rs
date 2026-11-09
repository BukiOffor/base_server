pub mod auth;
pub mod organisations;
pub mod projects;
pub mod roster;
pub mod tours;
pub mod users;
pub use super::*;

pub use axum::{
    extract::{Json, Path, Query, State},
    response::{IntoResponse, Redirect},
    routing::*,
};

pub fn get_routes(state: Arc<AppState>) -> Router {
    roster::routes(state)
}
