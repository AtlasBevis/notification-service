use serde_json::{Map, Value};
use tokio_postgres::Row;

use crate::enums::Method;
use crate::error::{AppError, AppResult};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WebhookAuthMode {
    Static,
    TokenEndpoint,
}

impl WebhookAuthMode {
    fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_uppercase().as_str() {
            "STATIC" => Some(Self::Static),
            "TOKEN_ENDPOINT" => Some(Self::TokenEndpoint),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TokenBodyFormat {
    Json,
    Form,
}

impl TokenBodyFormat {
    fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_uppercase().as_str() {
            "JSON" => Some(Self::Json),
            "FORM" => Some(Self::Form),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WebhookConfig {
    pub id: i32,
    pub target_id: i32,
    pub service_name: String,
    pub endpoint_url: String,
    pub http_method: Method,
    pub timeout_seconds: u64,
    pub max_retries: u32,
    /// Additional static headers sent with every webhook request.
    pub headers: Value,
}

impl WebhookConfig {
    pub fn map_row(row: &Row) -> AppResult<Self> {
        let domain: String = row.get("domain");
        let path: Option<String> = row.get("path");
        let configured_method: String = row.get("method");
        let http_method = Method::parse(&configured_method.to_ascii_uppercase())
            .ok_or_else(|| AppError::internal("webhook config has an invalid HTTP method"))?;
        let timeout_seconds: i32 = row.get("timeout_seconds");
        let max_retries: i32 = row.get("max_retries");
        let headers: Value = row.get("headers");
        if timeout_seconds <= 0 || max_retries < 0 {
            return Err(AppError::internal(
                "webhook config timeout or max_retries is invalid",
            ));
        }
        if !headers.is_object() {
            return Err(AppError::internal("webhook headers must be a JSON object"));
        }

        let endpoint_url = match path
            .as_deref()
            .map(str::trim)
            .filter(|path| !path.is_empty())
        {
            Some(path) => format!(
                "{}/{}",
                domain.trim_end_matches('/'),
                path.trim_start_matches('/')
            ),
            None => domain,
        };

        Ok(Self {
            id: row.get("id"),
            target_id: row.get("target_id"),
            service_name: row.get("service_name"),
            endpoint_url,
            http_method,
            timeout_seconds: timeout_seconds as u64,
            max_retries: max_retries as u32,
            headers,
        })
    }
}

#[derive(Clone)]
pub struct WebhookAuthConfig {
    pub id: i32,
    pub mode: WebhookAuthMode,
    pub header_name: String,
    pub header_value: Option<String>,
    pub token_url: Option<String>,
    pub token_method: Method,
    pub token_body_format: TokenBodyFormat,
    pub request_headers: Value,
    pub request_body: Value,
    pub timeout_seconds: u64,
    pub token_path: String,
    pub expires_in_path: Option<String>,
    pub default_ttl_seconds: u64,
    pub header_prefix: String,
}

impl WebhookAuthConfig {
    pub fn map_row(row: &Row) -> AppResult<Self> {
        let mode_value: String = row.get("auth_mode");
        let mode = WebhookAuthMode::parse(&mode_value).ok_or_else(|| {
            AppError::internal(format!("unknown webhook auth mode `{mode_value}`"))
        })?;
        let header_name: String = row.get("header_name");
        let header_value: Option<String> = row.get("header_value");
        if header_name.trim().is_empty() {
            return Err(AppError::internal(
                "webhook auth config header name is required",
            ));
        }
        let configured_method: String = row.get("token_method");
        let token_method = Method::parse(&configured_method.to_ascii_uppercase())
            .ok_or_else(|| AppError::internal("webhook token method is invalid"))?;
        let configured_format: String = row.get("token_body_format");
        let token_body_format = TokenBodyFormat::parse(&configured_format)
            .ok_or_else(|| AppError::internal("webhook token body format is invalid"))?;
        let request_headers: Value = row.get("request_headers");
        let request_body: Value = row.get("request_body");
        if !request_headers.is_object() || !request_body.is_object() {
            return Err(AppError::internal(
                "webhook token request headers and body must be JSON objects",
            ));
        }
        let timeout_seconds: i32 = row.get("timeout_seconds");
        let default_ttl_seconds: i32 = row.get("default_ttl_seconds");
        if timeout_seconds <= 0 || default_ttl_seconds <= 0 {
            return Err(AppError::internal(
                "webhook token timeout and default TTL must be positive",
            ));
        }
        let token_path: String = row.get("token_path");
        let token_url: Option<String> = row.get("token_url");
        let expires_in_path: Option<String> = row.get("expires_in_path");
        match mode {
            WebhookAuthMode::Static if header_value.as_deref().unwrap_or_default().is_empty() => {
                return Err(AppError::internal(
                    "webhook static auth header value is required",
                ));
            }
            WebhookAuthMode::TokenEndpoint
                if token_url.as_deref().unwrap_or_default().trim().is_empty()
                    || token_path.trim().is_empty() =>
            {
                return Err(AppError::internal(
                    "webhook token URL and token path are required",
                ));
            }
            _ => {}
        }
        Ok(Self {
            id: row.get("auth_id"),
            mode,
            header_name,
            header_value,
            token_url,
            token_method,
            token_body_format,
            request_headers,
            request_body,
            timeout_seconds: u64::try_from(timeout_seconds)
                .map_err(|_| AppError::internal("webhook token timeout is invalid"))?,
            token_path,
            expires_in_path: expires_in_path.filter(|path| !path.trim().is_empty()),
            default_ttl_seconds: u64::try_from(default_ttl_seconds)
                .map_err(|_| AppError::internal("webhook token default TTL is invalid"))?,
            header_prefix: row.get("header_prefix"),
        })
    }
}

pub type WebhookPayload = Map<String, Value>;
