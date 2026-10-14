use serde_json::Value;

use crate::error::{AppError, AppResult};

pub struct JsonUtils;

impl JsonUtils {
    pub fn get_string(value: &Value, key: &str) -> Option<String> {
        value
            .get(key)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    }

    pub fn str_to_json(s: &str) -> AppResult<Value> {
        from_str(s)
    }

    pub fn is_object(s: &str) -> AppResult<bool> {
        let value = from_str(s)?;
        Ok(value.is_object())
    }
}

fn from_str(s: &str) -> AppResult<Value> {
    serde_json::from_str::<Value>(s)
        .map_err(|_| AppError::bad_request("invalid JSON str".to_string()))
}
