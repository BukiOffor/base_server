use super::*;
pub mod organisations;
pub mod projects;
pub mod tours;
pub mod users;

#[derive(Debug, Clone, Serialize, Deserialize, Default, utoipa::ToSchema)]
pub struct Message {
    pub message: String,
}

impl Message {
    pub fn new<S: Into<String>>(message: S) -> Self {
        Message {
            message: message.into(),
        }
    }
}

impl From<&str> for Message {
    fn from(message: &str) -> Self {
        Message::new(message)
    }
}
