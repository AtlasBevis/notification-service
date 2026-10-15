use chrono::{DateTime, Utc};
use reqwest::header::{HeaderName, HeaderValue};
use reqwest::Url;
use serde::{Deserialize, Serialize};
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

/// Body of both create and update. Update takes `target_code` from the path.
/// Secrets in `auth` are write-only: `WebhookData` never returns them.
#[derive(Clone, Deserialize)]
pub struct SaveWebhookRequest {
    #[serde(default)]
    pub target_code: String,
    pub domain: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default = "SaveWebhookRequest::default_method")]
    pub method: String,
    #[serde(default = "SaveWebhookRequest::default_timeout_seconds")]
    pub timeout_seconds: i32,
    #[serde(default = "SaveWebhookRequest::default_max_retries")]
    pub max_retries: i32,
    #[serde(default)]
    pub headers: Map<String, Value>,
    pub auth: SaveWebhookAuthRequest,
}

#[derive(Clone, Deserialize)]
pub struct SaveWebhookAuthRequest {
    #[serde(default = "SaveWebhookRequest::default_auth_mode")]
    pub auth_mode: String,
    #[serde(default = "SaveWebhookRequest::default_header_name")]
    pub header_name: String,
    #[serde(default)]
    pub header_value: Option<String>,
    #[serde(default = "SaveWebhookRequest::default_header_prefix")]
    pub header_prefix: String,
    #[serde(default)]
    pub token_url: Option<String>,
    #[serde(default = "SaveWebhookRequest::default_method")]
    pub token_method: String,
    #[serde(default = "SaveWebhookRequest::default_token_body_format")]
    pub token_body_format: String,
    #[serde(default)]
    pub request_headers: Map<String, Value>,
    #[serde(default)]
    pub request_body: Map<String, Value>,
    #[serde(default = "SaveWebhookRequest::default_timeout_seconds")]
    pub timeout_seconds: i32,
    #[serde(default = "SaveWebhookRequest::default_token_path")]
    pub token_path: String,
    #[serde(default = "SaveWebhookRequest::default_expires_in_path")]
    pub expires_in_path: Option<String>,
    #[serde(default = "SaveWebhookRequest::default_ttl_seconds")]
    pub default_ttl_seconds: i32,
}

/// Defaults for omitted body fields, shared by the HTTP body and the gRPC request.
impl SaveWebhookRequest {
    pub fn default_method() -> String {
        "POST".to_string()
    }

    pub fn default_timeout_seconds() -> i32 {
        10
    }

    pub fn default_max_retries() -> i32 {
        3
    }

    pub fn default_auth_mode() -> String {
        "STATIC".to_string()
    }

    pub fn default_header_name() -> String {
        "x-api-key".to_string()
    }

    pub fn default_header_prefix() -> String {
        "Bearer ".to_string()
    }

    pub fn default_token_body_format() -> String {
        "JSON".to_string()
    }

    pub fn default_token_path() -> String {
        "access_token".to_string()
    }

    pub fn default_expires_in_path() -> Option<String> {
        Some("expires_in".to_string())
    }

    pub fn default_ttl_seconds() -> i32 {
        300
    }
}

impl SaveWebhookRequest {
    pub fn normalize(&mut self) {
        self.target_code = self.target_code.trim().to_ascii_uppercase();
        self.domain = self.domain.trim().to_string();
        self.path = self
            .path
            .take()
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty());
        self.method = self.method.trim().to_ascii_uppercase();
        let auth = &mut self.auth;
        auth.auth_mode = auth.auth_mode.trim().to_ascii_uppercase();
        auth.header_name = auth.header_name.trim().to_string();
        auth.header_value = auth.header_value.take().filter(|v| !v.trim().is_empty());
        auth.token_url = auth
            .token_url
            .take()
            .map(|u| u.trim().to_string())
            .filter(|u| !u.is_empty());
        auth.token_method = auth.token_method.trim().to_ascii_uppercase();
        auth.token_body_format = auth.token_body_format.trim().to_ascii_uppercase();
        auth.token_path = auth.token_path.trim().to_string();
        auth.expires_in_path = auth
            .expires_in_path
            .take()
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty());
    }

    pub fn validate(&self) -> AppResult<()> {
        if self.target_code.is_empty() {
            return Err(AppError::bad_request("target_code is required"));
        }
        Self::check_url("domain", &self.domain)?;
        if !matches!(self.method.as_str(), "POST" | "PUT" | "PATCH") {
            return Err(AppError::bad_request("method must be POST, PUT or PATCH"));
        }
        if self.timeout_seconds <= 0 {
            return Err(AppError::bad_request("timeout_seconds must be positive"));
        }
        if self.max_retries < 0 {
            return Err(AppError::bad_request("max_retries must not be negative"));
        }
        Self::check_headers("headers", &self.headers)?;

        let auth = &self.auth;
        let mode = WebhookAuthMode::parse(&auth.auth_mode).ok_or_else(|| {
            AppError::bad_request("auth.auth_mode must be STATIC or TOKEN_ENDPOINT")
        })?;
        HeaderName::from_bytes(auth.header_name.as_bytes())
            .map_err(|_| AppError::bad_request("auth.header_name is not a valid header name"))?;
        match mode {
            WebhookAuthMode::Static => {
                let value = auth.header_value.as_deref().ok_or_else(|| {
                    AppError::bad_request("auth.header_value is required for STATIC")
                })?;
                HeaderValue::from_str(value).map_err(|_| {
                    AppError::bad_request("auth.header_value is not a valid header value")
                })?;
            }
            WebhookAuthMode::TokenEndpoint => {
                let token_url = auth.token_url.as_deref().ok_or_else(|| {
                    AppError::bad_request("auth.token_url is required for TOKEN_ENDPOINT")
                })?;
                Self::check_url("auth.token_url", token_url)?;
                if Method::parse(&auth.token_method).is_none() {
                    return Err(AppError::bad_request(
                        "auth.token_method must be GET, POST, PUT or PATCH",
                    ));
                }
                if TokenBodyFormat::parse(&auth.token_body_format).is_none() {
                    return Err(AppError::bad_request(
                        "auth.token_body_format must be JSON or FORM",
                    ));
                }
                if auth.token_path.is_empty() {
                    return Err(AppError::bad_request("auth.token_path is required"));
                }
                HeaderValue::from_str(&auth.header_prefix).map_err(|_| {
                    AppError::bad_request("auth.header_prefix is not a valid header value")
                })?;
                Self::check_headers("auth.request_headers", &auth.request_headers)?;
            }
        }
        if auth.timeout_seconds <= 0 || auth.default_ttl_seconds <= 0 {
            return Err(AppError::bad_request(
                "auth.timeout_seconds and auth.default_ttl_seconds must be positive",
            ));
        }
        Ok(())
    }

    fn check_url(field: &str, value: &str) -> AppResult<()> {
        let url = Url::parse(value)
            .map_err(|_| AppError::bad_request(format!("{field} is not a valid URL")))?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err(AppError::bad_request(format!(
                "{field} must be an http(s) URL with a host and no credentials"
            )));
        }
        Ok(())
    }

    fn check_headers(field: &str, headers: &Map<String, Value>) -> AppResult<()> {
        for (name, value) in headers {
            HeaderName::from_bytes(name.as_bytes()).map_err(|_| {
                AppError::bad_request(format!("{field}: `{name}` is not a valid header name"))
            })?;
            let value = value.as_str().ok_or_else(|| {
                AppError::bad_request(format!("{field}: `{name}` must be a string"))
            })?;
            HeaderValue::from_str(value).map_err(|_| {
                AppError::bad_request(format!("{field}: `{name}` has an invalid value"))
            })?;
        }
        Ok(())
    }
}

/// A webhook config with its auth config, as returned by the API. Secrets
/// (`header_value`, token request headers and body) are never included.
#[derive(Debug, Clone, Serialize)]
pub struct WebhookData {
    pub id: i32,
    pub target_id: i32,
    pub target_code: String,
    pub domain: String,
    pub path: Option<String>,
    pub method: String,
    pub timeout_seconds: i32,
    pub max_retries: i32,
    pub headers: Value,
    pub auth: Option<WebhookAuthData>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WebhookAuthData {
    pub id: i32,
    pub auth_mode: String,
    pub header_name: String,
    pub has_header_value: bool,
    pub header_prefix: String,
    pub token_url: Option<String>,
    pub token_method: String,
    pub token_body_format: String,
    pub token_path: String,
    pub expires_in_path: Option<String>,
    pub timeout_seconds: i32,
    pub default_ttl_seconds: i32,
}

impl WebhookData {
    pub fn map_row(row: &Row) -> AppResult<Self> {
        let auth_id: Option<i32> = row.get("auth_id");
        let auth = auth_id.map(|id| WebhookAuthData {
            id,
            auth_mode: row.get("auth_mode"),
            header_name: row.get("header_name"),
            has_header_value: row.get("has_header_value"),
            header_prefix: row.get("header_prefix"),
            token_url: row.get("token_url"),
            token_method: row.get("token_method"),
            token_body_format: row.get("token_body_format"),
            token_path: row.get("token_path"),
            expires_in_path: row.get("expires_in_path"),
            timeout_seconds: row.get("auth_timeout_seconds"),
            default_ttl_seconds: row.get("default_ttl_seconds"),
        });
        Ok(Self {
            id: row.get("id"),
            target_id: row.get("target_id"),
            target_code: row.get("target_code"),
            domain: row.get("domain"),
            path: row.get("path"),
            method: row.get("method"),
            timeout_seconds: row.get("timeout_seconds"),
            max_retries: row.get("max_retries"),
            headers: row.get("headers"),
            auth,
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }
}
