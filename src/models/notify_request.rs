use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::ToSchema;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NotifyRequest {
    #[schema(example = "AIRFLOW")]
    pub service: String,

    #[schema(example = "notification_uuid")]
    pub code: String,

    #[schema(example = "trace_id")]
    pub trace_id: String,

    #[serde(default)]
    #[schema(value_type = Object)]
    pub variables: Map<String, Value>,
}

impl NotifyRequest {
    pub fn normalize(&mut self) {
        self.service = self.service.trim().to_string();
        self.code = self.code.trim().to_string();
        self.trace_id = self.trace_id.trim().to_string();
    }

    pub fn validate(&self) -> AppResult<()> {
        if self.service.is_empty() {
            return Err(AppError::BadRequest("service is required".to_string()));
        }

        if self.code.is_empty() {
            return Err(AppError::BadRequest("code is required".to_string()));
        }

        if self.trace_id.is_empty() {
            return Err(AppError::BadRequest("trace_id is required".to_string()));
        }
        Ok(())
    }
}
