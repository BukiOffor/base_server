use axum::http::{Method, header::*};
use diesel_migrations::{EmbeddedMigrations, embed_migrations};
use server::{AppState, handlers};
use std::sync::Arc;

use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

#[derive(OpenApi)]
#[openapi(
    paths(
        server::handlers::users::register_user,
    ),
    components(
        schemas(server::dto::Message, server::dto::users::NewUser, server::errors::ErrorMessage)
    ),
    tags(
        (name = "Users", description = "User management endpoints")
    )
)]
struct ApiDoc;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .pretty()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let pool: Arc<
        bb8::Pool<server::AsyncDieselConnectionManager<diesel_async::AsyncPgConnection>>,
    > = server::config::create_pool().await.into();

    // Run database migrations
    if let Err(e) = run_migration().await {
        tracing::error!("{}", e.to_string());
        std::process::exit(1);
    }

    let config = Arc::new(server::config::Config::init().unwrap_or_else(|e| {
        tracing::error!(e = %e, "Failed to load configuration");
        std::process::exit(1);
    }));
    let state: Arc<AppState> = AppState {
        pool: pool.clone(),
        config,
    }
    .into();

    let cors = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::PUT,
        ])
        .allow_headers([
            CONTENT_TYPE,
            ACCESS_CONTROL_ALLOW_HEADERS,
            ACCESS_CONTROL_ALLOW_CREDENTIALS,
            ACCESS_CONTROL_ALLOW_ORIGIN,
        ])
        .allow_credentials(true)
        .allow_origin([
            "http://localhost:3000".parse::<HeaderValue>().unwrap(),
            "http://localhost:54465".parse::<HeaderValue>().unwrap(),
            "http://127.0.0.1:3000".parse::<HeaderValue>().unwrap(),
        ]);
    server::info!("Starting Web Server ............");
    let app = handlers::get_routes(state)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    let api = axum::Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .nest("/api/v1", app);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:9000").await.unwrap();
    server::info!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, api).await.unwrap();
}

pub async fn run_migration() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use diesel::prelude::*;
    use diesel_migrations::MigrationHarness;

    // Get a synchronous connection string from your config
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Run migrations in a blocking thread
    tokio::task::spawn_blocking(move || {
        let mut conn = diesel::PgConnection::establish(&database_url)?;
        conn.run_pending_migrations(MIGRATIONS).map_err(|e| {
            tracing::error!("{}", e.to_string());
            std::process::exit(1);
        })?;
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    })
    .await?
}
