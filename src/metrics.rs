use crate::config::ServerConfig;
use crate::error::{AppError, AppResult};
use metrics_exporter_prometheus::PrometheusBuilder;
use std::net::SocketAddr;

/// Install Prometheus scrape endpoint on a dedicated listener when enabled.
///
/// Scrapes respond on any path (typically `GET /metrics`).
pub fn init(server: &ServerConfig) -> AppResult<()> {
    if !server.metrics.enabled {
        tracing::info!("Prometheus metrics exporter disabled");
        return Ok(());
    }

    let addr: SocketAddr = server.metrics_addr().parse().map_err(|e| {
        AppError::internal_with(format!("Invalid metrics addr {}", server.metrics_addr()), e)
    })?;

    PrometheusBuilder::new()
        .with_http_listener(addr)
        .install()
        .map_err(|e| AppError::internal_with("Failed to install Prometheus exporter", e))?;

    tracing::info!(%addr, "Prometheus metrics listening");
    Ok(())
}
