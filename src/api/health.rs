use axum::extract::State;
use axum::http::StatusCode;
use serde::Serialize;

use crate::config;
use crate::models::ApiResponse;
use crate::state::SharedState;

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

pub async fn health(State(state): State<SharedState>) -> ApiResponse<HealthResponse> {
    let server = &config::load().server;
    let kafka_enabled = config::load().kafka.enabled;

    let postgres = match state.postgres.ping().await {
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
        match &state.kafka {
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
