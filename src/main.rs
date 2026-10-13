use data_notification::api::routes;
use data_notification::bootstrap;
use data_notification::config::Config;
use data_notification::error::{AppError, AppResult};
use data_notification::grpc;
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
    let app = routes::app_routes(state.clone());

    let addr: SocketAddr = config
        .server
        .http_addr()
        .parse()
        .map_err(|e| AppError::internal_with("Invalid server http addr", e))?;
    tracing::info!("Listening on {}", addr);

    let listener = TcpListener::bind(addr).await?;
    let http = async { axum::serve(listener, app).await.map_err(AppError::from) };

    if !config.server.grpc.enabled {
        return http.await;
    }
    let grpc_addr: SocketAddr = config
        .server
        .grpc_addr()
        .parse()
        .map_err(|e| AppError::internal_with("Invalid server grpc addr", e))?;
    tokio::try_join!(http, grpc::serve(state, grpc_addr))?;

    Ok(())
}
