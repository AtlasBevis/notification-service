use crate::api::routes;
use crate::bootstrap;
use crate::config::load_config;
use crate::metrics;
use anyhow::{Context, Result};
use chrono::{FixedOffset, Utc};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing_subscriber::fmt::{format::Writer, time::FormatTime};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

struct VietnamTime;

impl FormatTime for VietnamTime {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        let vn = FixedOffset::east_opt(7 * 3600).expect("UTC+7");
        let now = Utc::now().with_timezone(&vn);
        write!(
            w,
            "{}.{:03}{}",
            now.format("%Y-%m-%dT%H:%M:%S"),
            now.timestamp_subsec_millis(),
            now.format("%:z"),
        )
    }
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "data_notification=info,tower_http=info".into()),
        ))
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .flatten_event(true)
                .with_current_span(true)
                .with_span_list(false)
                .with_timer(VietnamTime),
        )
        .init();
}

pub async fn run_api() -> Result<()> {
    init_tracing();
    tracing::info!("Starting Data Notification API...");

    let config = load_config()?;
    tracing::info!(version = %config.server.version, "Config loaded");

    metrics::init(&config.server)?;

    let state = bootstrap::build_state(&config).await?;
    let app = routes::app_routes(state);

    let addr: SocketAddr = config
        .server
        .http_addr()
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid server http addr: {e}"))?;
    tracing::info!("Listening on {}", addr);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Kafka worker: `notification-consumer` binary. Dispatch TEAMS / EMAIL.
pub async fn run_notification_consumer() -> Result<()> {
    init_tracing();
    tracing::info!("Starting notification-consumer...");

    let config = load_config()?;
    tracing::info!(version = %config.server.version, "Config loaded");

    if !config.kafka.consumer_enabled() {
        anyhow::bail!("kafka.enabled and kafka.consumer.enabled must be true for notification-consumer");
    }

    metrics::init(&config.server)?;

    let service = bootstrap::build_notification_consumer(&config)
        .await
        .context("Failed to start notification-consumer")?;
    service.run_consumers().await;
    anyhow::bail!("notification-consumer stopped")
}
