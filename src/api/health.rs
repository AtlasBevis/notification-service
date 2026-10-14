use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use serde::Serialize;

use crate::config::Config;
use crate::infras::kafka::KafkaBus;
use crate::infras::postgres::PostgresManager;
use crate::models::ApiResponse;

#[derive(Serialize)]
pub struct DependencyStatus {
    pub status: String,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub profile: String,
    pub version: String,
    pub postgres: DependencyStatus,
    pub kafka: DependencyStatus,
}

pub struct HealthController {
    postgres: Arc<PostgresManager>,
    kafka: Option<Arc<KafkaBus>>,
}

impl HealthController {
    pub fn new(postgres: Arc<PostgresManager>, kafka: Option<Arc<KafkaBus>>) -> Self {
        Self { postgres, kafka }
    }

    pub async fn health(State(this): State<Arc<Self>>) -> ApiResponse<HealthResponse> {
        let server = &Config::load().server;
        let kafka_enabled = Config::load().kafka.enabled;

        let postgres = match this.postgres.ping().await {
            Ok(()) => DependencyStatus {
                status: "ok".to_string(),
                error: None,
            },
            Err(err) => DependencyStatus {
                status: "error".to_string(),
                error: Some(err.to_string()),
            },
        };

        let kafka = if !kafka_enabled {
            DependencyStatus {
                status: "disabled".to_string(),
                error: None,
            }
        } else {
            match &this.kafka {
                Some(bus) => match bus.ping().await {
                    Ok(()) => DependencyStatus {
                        status: "ok".to_string(),
                        error: None,
                    },
                    Err(err) => DependencyStatus {
                        status: "error".to_string(),
                        error: Some(err.to_string()),
                    },
                },
                None => DependencyStatus {
                    status: "error".to_string(),
                    error: Some("Kafka is not connected".to_string()),
                },
            }
        };

        let ok = postgres.status == "ok" && kafka.status != "error";
        let body = HealthResponse {
            status: if ok {
                "ok".to_string()
            } else {
                "error".to_string()
            },
            profile: server.profile.clone(),
            version: server.version.clone(),
            postgres,
            kafka,
        };

        if ok {
            ApiResponse::success(body)
        } else {
            ApiResponse::new(
                StatusCode::SERVICE_UNAVAILABLE,
                StatusCode::SERVICE_UNAVAILABLE.as_u16(),
                "Service Unavailable",
                Some(body),
            )
        }
    }
}
