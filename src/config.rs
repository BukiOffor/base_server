use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use dotenvy::dotenv;
use std::{error::Error, time::Duration};

#[derive(Debug, Clone)]
pub struct Config {
    pub client_origin: String,
    pub token_url: String,
    pub auth_url: String,
    pub google_oauth_client_id: String,
    pub google_oauth_client_secret: String,
    pub google_oauth_redirect_url: String, // redirect on your server
}

impl Config {
    pub fn init() -> Result<Config, Box<dyn Error>> {
        let client_origin = std::env::var("CLIENT_ORIGIN")?;
        let token_url = std::env::var("TOKEN_URL")
            .unwrap_or("https://www.googleapis.com/oauth2/v4/token".into());
        let auth_url = std::env::var("AUTH_URL")
            .unwrap_or("https://accounts.google.com/o/oauth2/v2/auth".to_string());
        let google_oauth_client_id = std::env::var("GOOGLE_OAUTH_CLIENT_ID")?;
        let google_oauth_client_secret = std::env::var("GOOGLE_OAUTH_CLIENT_SECRET")?;
        let google_oauth_redirect_url = std::env::var("GOOGLE_OAUTH_REDIRECT_URL")?;

        Ok(Config {
            client_origin,
            token_url,
            auth_url,
            google_oauth_client_id,
            google_oauth_client_secret,
            google_oauth_redirect_url,
        })
    }
}

pub async fn create_pool() -> crate::Pool {
    dotenv().ok();
    let db_url = std::env::var("DATABASE_URL").unwrap();
    // set up connection pool
    let config = AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(db_url);
    bb8::Pool::builder()
        .max_size(10)
        .min_idle(Some(8))
        .connection_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(300))
        .build(config)
        .await
        .unwrap()
}
