use data_notification::api::routes;
use data_notification::bootstrap;
use data_notification::config::Config;
use data_notification::error::{AppError, AppResult};
use data_notification::metrics;
use std::net::SocketAddr;
use tokio::net::TcpListener;

const APP_CONFIG_FILE: &str = "app.yaml";

#[tokio::main]
async fn main() -> AppResult<()> {
    bootstrap::init_tracing();
    tracing::info!("Starting Data Notification API...");

    let config = Config::set(Config::new(APP_CONFIG_FILE)?)?;
    tracing::info!(version = %config.server.version, "Config loaded");

    metrics::init(&config.server)?;

    let state = bootstrap::build_state(config).await?;
    let app = routes::app_routes(state);

    let addr: SocketAddr = config
        .server
        .http_addr()
        .parse()
        .map_err(|e| AppError::internal_with("Invalid server http addr", e))?;
    tracing::info!("Listening on {}", addr);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
