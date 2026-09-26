use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::ToSchema;

use crate::{
    error::{AppError, AppResult},
    models::Recipients,
};

pub const EVENT_TYPE_SEND: &str = "notification.send";

/// API body: source triggers send with notification code + payload.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NotifyRequest {
    /// Public `notifications.code`.
    #[schema(example = "AIRFLOW_FAIL")]
    pub code: String,

    #[schema(example = "trace_id")]
    pub trace_id: String,

    #[serde(default)]
    #[schema(value_type = Object)]
    pub variables: Map<String, Value>,
}

impl NotifyRequest {
    pub fn normalize(&mut self) {
        self.code = self.code.trim().to_ascii_uppercase();
        self.trace_id = self.trace_id.trim().to_string();
    }

    pub fn validate(&self) -> AppResult<()> {
        if self.code.is_empty() {
            return Err(AppError::BadRequest("code is required".to_string()));
        }

        if self.trace_id.is_empty() {
            return Err(AppError::BadRequest("trace_id is required".to_string()));
        }
        Ok(())
    }
}

/// Resolved notify context for template render + channel dispatch (Kafka payload).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyContext {
    pub notification_id: i64,
    pub code: String,
    pub source: String,
    pub target: String,
    pub channel: String,
    pub template: String,
    pub correlation_id: String,
    pub kind: String,
    pub title: Option<String>,
    pub message: Option<String>,
    pub payload: Value,
    pub recipients: Recipients,
    pub metadata: Map<String, Value>,
}

impl NotifyContext {
    pub fn team(&self) -> Option<&str> {
        self.metadata
            .get("team")
            .and_then(|v| v.as_str())
            .map(str::trim)
    }
}

/// Kafka payload for `notification.send`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyMessage {
    pub delivery_id: i64,
    pub notification_id: i64,
    pub code: String,
    #[serde(flatten)]
    pub context: NotifyContext,
}
