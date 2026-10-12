use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyRequest {
    pub code: String,
    pub trace_id: String,

    #[serde(default)]
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
