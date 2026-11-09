use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};
#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
    pub fields: Option<Value>,
    pub details: Option<Value>,
}
pub type Result<T> = std::result::Result<T, ApiError>;
impl ApiError {
    pub fn new(status: u16, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            code,
            message: message.into(),
            fields: None,
            details: None,
        }
    }
    pub fn bad(message: impl Into<String>) -> Self {
        Self::new(400, "VALIDATION_ERROR", message)
    }
    pub fn missing() -> Self {
        Self::new(404, "NOT_FOUND", "Resource not found")
    }
    pub fn unauthorized() -> Self {
        Self::new(401, "UNAUTHORIZED", "A valid bearer token is required")
    }
    pub fn forbidden() -> Self {
        Self::new(403, "FORBIDDEN", "You cannot perform this action")
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut e = json!({"code":self.code,"message":self.message});
        if let Some(details) = self.details {
            e["details"] = details;
        }
        if let Some(f) = self.fields {
            e["fields"] = f;
        }
        (self.status, Json(json!({"error":e}))).into_response()
    }
}
impl From<diesel::result::Error> for ApiError {
    fn from(e: diesel::result::Error) -> Self {
        use diesel::result::{DatabaseErrorKind, Error};
        match e {
            Error::NotFound => Self::missing(),
            Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => Self::new(
                409,
                "CONFLICT",
                "This change conflicts with an existing record; refresh and retry",
            ),
            Error::DatabaseError(DatabaseErrorKind::ForeignKeyViolation, _) => Self::new(
                422,
                "INVALID_REFERENCE",
                "Referenced resource does not exist or belongs to another session",
            ),
            _ => {
                tracing::error!(error=%e,"Database operation failed");
                Self::new(500, "INTERNAL_ERROR", "Database operation failed")
            }
        }
    }
}
impl From<serde_json::Error> for ApiError {
    fn from(_: serde_json::Error) -> Self {
        Self::bad("Invalid request data")
    }
}
