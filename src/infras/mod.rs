use crate::config::{Config, HttpConfig, PostgresConfig};
use crate::error::AppResult;
use std::sync::Arc;

pub mod http;
pub mod kafka;
pub mod postgres;

use http::{build_http_client, HttpClient};
use kafka::KafkaBus;
use postgres::PostgresManager;

#[derive(Clone)]
pub struct Infrastructure {
    pub http_client: Arc<HttpClient>,
    pub postgres: Arc<PostgresManager>,
    pub kafka: Option<Arc<KafkaBus>>,
}

impl Infrastructure {
    pub async fn new(config: &Config) -> AppResult<Self> {
        tracing::info!("Initializing infrastructure...");

        let http_client = Self::init_http_client(&config.server.http)?;
        let postgres = Self::init_postgres(&config.postgres).await?;
        let kafka = Self::init_kafka(config).await;

        Ok(Self {
            http_client,
            postgres,
            kafka,
        })
    }

    async fn init_kafka(config: &Config) -> Option<Arc<KafkaBus>> {
        if !config.kafka.enabled {
            tracing::info!("Kafka is disabled");
            return None;
        }
        match KafkaBus::connect(&config.kafka).await {
            Ok(bus) => {
                tracing::info!(brokers = ?config.kafka.brokers, "Kafka connected");
                Some(Arc::new(bus))
            }
            Err(err) => {
                tracing::error!(error = %err, "Kafka unavailable");
                None
            }
        }
    }

    fn init_http_client(config: &HttpConfig) -> AppResult<Arc<HttpClient>> {
        tracing::info!("Initializing HTTP client...");
        Ok(Arc::new(build_http_client(config.timeout)?))
    }

    async fn init_postgres(config: &PostgresConfig) -> AppResult<Arc<PostgresManager>> {
        tracing::info!(
            host = %config.host,
            port = config.port,
            database = %config.database,
            "Initializing Postgres connection pool..."
        );
        let manager = PostgresManager::new(config)?;
        manager.ping().await?;
        tracing::info!("Postgres connected");
        Ok(Arc::new(manager))
    }
}
