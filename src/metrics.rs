use crate::config::ServerConfig;
use anyhow::{Context, Result};
use metrics_exporter_prometheus::PrometheusBuilder;
use std::net::SocketAddr;

/// Install Prometheus scrape endpoint on a dedicated listener when enabled.
///
/// Scrapes respond on any path (typically `GET /metrics`).
pub fn init(server: &ServerConfig) -> Result<()> {
    if !server.metrics.enabled {
        tracing::info!("Prometheus metrics exporter disabled");
        return Ok(());
    }

    let addr: SocketAddr = server
        .metrics_addr()
        .parse()
        .with_context(|| format!("Invalid metrics addr {}", server.metrics_addr()))?;

    PrometheusBuilder::new()
        .with_http_listener(addr)
        .install()
        .map_err(|e| anyhow::anyhow!("Failed to install Prometheus exporter: {e}"))?;

    tracing::info!(%addr, "Prometheus metrics listening");
    Ok(())
}
