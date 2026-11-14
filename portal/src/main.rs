use clap::Parser;
use std::{net::SocketAddr, sync::Arc};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use iaop_developer_portal::{build_router, load_and_validate_catalog, AppConfig, AppState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,iaop_developer_portal=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = AppConfig::parse();

    info!(
        "Starting IAOP Developer Portal v2 (Scalar) on {}:{}",
        config.host, config.port
    );
    info!("Loading catalog from {:?}", config.catalog_path);

    let catalog =
        load_and_validate_catalog(&config.catalog_path, Some(&config.catalog_schema_path))?;

    info!(
        "Catalog validated (schema v{}). {} services registered.",
        catalog.schema_version,
        catalog.services.len()
    );

    let cors_info = if config.allowed_cors_origins.iter().any(|o| !o.is_empty()) {
        format!("{:?}", config.allowed_cors_origins)
    } else {
        "* (wildcard — dev mode)".to_string()
    };
    info!("CORS allowed origins: {}", cors_info);

    let state = AppState {
        config: config.clone(),
        catalog: Arc::new(catalog),
    };

    let app = build_router(state);

    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    info!("Landing page: http://{}/", addr);
    info!("Frontend APIs: http://{}/reference/frontend", addr);
    info!("Backend APIs:  http://{}/reference/backend", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Signal received, shutting down gracefully...");
}
