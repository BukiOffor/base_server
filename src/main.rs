use axum::http::{Method, header::*};
use diesel_migrations::{EmbeddedMigrations, embed_migrations};
use server::{AppState, handlers};
use std::sync::Arc;

use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};
use utoipa_swagger_ui::SwaggerUi;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .pretty()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "server=info,tower_http=info".into()),
        )
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

    {
        let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
        assert!(
            secret.len() >= 32,
            "JWT_SECRET must contain at least 32 bytes"
        );
        let mut conn = pool.get().await.expect("Database unavailable");
        server::auth::admin::bootstrap(&mut conn)
            .await
            .expect("Admin bootstrap failed");
    }
    server::services::roster::start_maintenance(state.clone());
    let origins = state
        .config
        .client_origin
        .split(',')
        .map(|s| {
            s.trim()
                .parse::<HeaderValue>()
                .expect("Invalid CLIENT_ORIGIN")
        })
        .collect::<Vec<_>>();
    let cors = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::PUT,
        ])
        .allow_headers([CONTENT_TYPE, AUTHORIZATION])
        .expose_headers([CONTENT_DISPOSITION])
        .allow_origin(origins);
    server::info!("Starting Web Server ............");
    let app = handlers::get_routes(state)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .layer(axum::extract::DefaultBodyLimit::max(
            5 * 1024 * 1024 + 16384,
        ));

    let api = axum::Router::new()
        .merge(
            SwaggerUi::new("/swagger-ui")
                .url("/api-docs/openapi.json", server::documentation::openapi()),
        )
        .nest("/api/v1", app);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:9000").await.unwrap();
    server::info!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(
        listener,
        api.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .unwrap();
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
