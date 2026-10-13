use data_notification::bootstrap;
use data_notification::config::load_config;
use data_notification::error::{AppError, AppResult};
use data_notification::metrics;

const JOB_CONFIG_FILE: &str = "app-job.yaml";

#[tokio::main]
async fn main() -> AppResult<()> {
    bootstrap::init_tracing();
    tracing::info!("[Job] running...");

    let config = load_config(JOB_CONFIG_FILE)?;
    tracing::info!(version = %config.server.version, "Config loaded");

    if !config.kafka.consumer_enabled() {
        return Err(AppError::internal(
            "[Job] kafka.enabled and kafka.consumer.enabled must be true",
        ));
    }

    metrics::init(&config.server)?;

    let state = bootstrap::build_state(&config)
        .await
        .map_err(|e| AppError::internal_with("Failed to start notification-job", e))?;
    let consumer = state
        .consumer_service
        .clone()
        .ok_or_else(|| AppError::internal("Kafka is required for notification-job"))?;

    consumer.consume().await;
    Err(AppError::internal("notification-job stopped"))
}
