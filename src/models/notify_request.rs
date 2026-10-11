use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyRequest {
    pub code: String,

    #[serde(default)]
    pub variables: Map<String, Value>,
}

impl NotifyRequest {
    pub fn normalize(&mut self) {
        self.code = self.code.trim().to_ascii_uppercase();
    }

    pub fn validate(&self) -> AppResult<()> {
        if self.code.is_empty() {
            return Err(AppError::BadRequest("code is required".to_string()));
        }
        Ok(())
    }
}
