use serde_json::{Map, Value};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenBodyFormat {
    Json,
    Form,
}

impl TokenBodyFormat {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "JSON" => Some(Self::Json),
            "FORM" => Some(Self::Form),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WebhookAuthConfig {
    pub id: i32,
    pub token_url: String,
    pub http_method: Method,
    pub body_format: TokenBodyFormat,
    /// JSON object of static headers for the token request.
    pub request_headers: Value,
    /// JSON object sent as the token request body (JSON or form-encoded).
    pub request_body: Value,
    pub timeout_seconds: u64,
    /// Dot path to the token in the response, e.g. `data.access_token`.
    pub token_path: String,
    /// Dot path to the lifetime in seconds; `None` => `default_ttl_seconds`.
    pub expires_in_path: Option<String>,
    pub default_ttl_seconds: u64,
    pub header_name: String,
    pub header_prefix: String,
}

pub type WebhookPayload = Map<String, Value>;
