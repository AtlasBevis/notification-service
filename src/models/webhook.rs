use serde_json::Value;

use crate::enums::Method;

#[derive(Debug, Clone)]
pub struct WebhookConfig {
    pub id: i32,
    pub channel_id: i32,
    pub endpoint_url: String,
    pub http_method: Method,
    pub timeout_seconds: u64,
    pub max_retries: u32,
}

pub type WebhookPayload = Value;
