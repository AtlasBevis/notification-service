use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client, Method, StatusCode, Url};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::enums::Method as ConfiguredMethod;
use crate::error::{AppError, AppResult};
use crate::models::{
    NotifyValue, TokenBodyFormat, WebhookAuthConfig, WebhookConfig, WebhookPayload,
};
use crate::repository::WebhookRepository;
use crate::services::{ChannelService, ChannelTrait};

/// Refresh a cached token this long before the provider says it expires.
const TOKEN_EXPIRY_SKEW: Duration = Duration::from_secs(30);

#[async_trait]
pub trait WebhookServiceTrait: Send + Sync {
    /// Ensures the channel has a webhook config (auth config is not loaded here).
    async fn validate(&self, channel_id: i32) -> AppResult<()>;
    async fn get_config(&self, channel_code: &str) -> AppResult<WebhookConfig>;
    async fn get_webhook_auth_configs(
        &self,
        webhook_config_id: i32,
    ) -> AppResult<Option<WebhookAuthConfig>>;
    /// `max_retries + 1`, from the channel's webhook config.
    async fn max_attempts(&self, channel_code: &str) -> AppResult<u32>;
    async fn send_webhook(&self, request: &NotifyValue) -> AppResult<()>;
}

#[derive(Clone)]
struct CachedToken {
    /// Ready-to-send header value, e.g. `Bearer eyJ...`.
    header_value: HeaderValue,
    expires_at: Instant,
}

#[derive(Clone)]
pub struct WebhookService {
    client: Client,
    repo: WebhookRepository,
    channels: Arc<ChannelService>,
    /// Keyed by `webhook_auth_configs.id`.
    tokens: Arc<Mutex<HashMap<i32, CachedToken>>>,
}

impl WebhookService {
    pub fn new(client: Client, repo: WebhookRepository, channels: Arc<ChannelService>) -> Self {
        Self {
            client,
            repo,
            channels,
            tokens: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn validate_endpoint(endpoint: &str) -> AppResult<Url> {
        let url = Url::parse(endpoint)
            .map_err(|_| AppError::bad_request("webhook endpoint URL is invalid"))?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(AppError::bad_request(
                "webhook endpoint must use http or https",
            ));
        }
        if url.host_str().is_none() || !url.username().is_empty() || url.password().is_some() {
            return Err(AppError::bad_request(
                "webhook endpoint must have a host and must not contain credentials",
            ));
        }
        Ok(url)
    }

    fn request_method(method: ConfiguredMethod) -> AppResult<Method> {
        match method {
            ConfiguredMethod::POST => Ok(Method::POST),
            ConfiguredMethod::PUT => Ok(Method::PUT),
            ConfiguredMethod::PATCH => Ok(Method::PATCH),
            ConfiguredMethod::GET => Err(AppError::bad_request(
                "webhook method must be POST, PUT or PATCH",
            )),
        }
    }

    fn token_method(method: ConfiguredMethod) -> Method {
        match method {
            ConfiguredMethod::GET => Method::GET,
            ConfiguredMethod::POST => Method::POST,
            ConfiguredMethod::PUT => Method::PUT,
            ConfiguredMethod::PATCH => Method::PATCH,
        }
    }

    fn map_send_error(step: &str, err: reqwest::Error) -> AppError {
        if err.is_timeout() {
            AppError::internal(format!("{step} request timed out"))
        } else {
            AppError::internal(format!("{step} connection failed"))
        }
    }

    /// Walks a dot path (`data.access_token`) through a JSON value.
    fn json_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
        path.split('.')
            .filter(|segment| !segment.is_empty())
            .try_fold(value, |current, segment| current.get(segment))
    }

    fn static_headers(headers: &Value) -> AppResult<HeaderMap> {
        let mut map = HeaderMap::new();
        for (name, value) in headers.as_object().into_iter().flatten() {
            let value = value
                .as_str()
                .ok_or_else(|| AppError::internal("webhook auth header values must be strings"))?;
            let name = HeaderName::from_bytes(name.as_bytes())
                .map_err(|_| AppError::internal("webhook auth header name is invalid"))?;
            let value = HeaderValue::from_str(value)
                .map_err(|_| AppError::internal("webhook auth header value is invalid"))?;
            map.insert(name, value);
        }
        Ok(map)
    }

    /// Form bodies are flat `key=value` pairs; non-string JSON values are stringified.
    fn form_fields(body: &Value) -> Vec<(String, String)> {
        body.as_object()
            .into_iter()
            .flatten()
            .map(|(key, value)| {
                let value = match value {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                (key.clone(), value)
            })
            .collect()
    }

    /// Returns a cached token, or calls the token endpoint (step 1).
    async fn token(&self, auth: &WebhookAuthConfig, force_refresh: bool) -> AppResult<HeaderValue> {
        if !force_refresh {
            let tokens = self.tokens.lock().await;
            if let Some(cached) = tokens.get(&auth.id) {
                if cached.expires_at > Instant::now() {
                    return Ok(cached.header_value.clone());
                }
            }
        }

        let fetched = self.fetch_token(auth).await?;
        let header_value = fetched.header_value.clone();
        self.tokens.lock().await.insert(auth.id, fetched);
        Ok(header_value)
    }

    async fn fetch_token(&self, auth: &WebhookAuthConfig) -> AppResult<CachedToken> {
        let url = Self::validate_endpoint(&auth.token_url)?;
        let method = Self::token_method(auth.http_method);
        let has_body = method != Method::GET;

        let mut request = self
            .client
            .request(method, url)
            .timeout(Duration::from_secs(auth.timeout_seconds.max(1)))
            .headers(Self::static_headers(&auth.request_headers)?);
        if has_body {
            request = match auth.body_format {
                TokenBodyFormat::Json => request.json(&auth.request_body),
                TokenBodyFormat::Form => request.form(&Self::form_fields(&auth.request_body)),
            };
        }

        let response = request
            .send()
            .await
            .map_err(|err| Self::map_send_error("webhook token", err))?;
        if !response.status().is_success() {
            return Err(AppError::internal(format!(
                "webhook token endpoint returned HTTP {}",
                response.status().as_u16()
            )));
        }
        let body: Value = response
            .json()
            .await
            .map_err(|_| AppError::internal("webhook token response is not valid JSON"))?;

        let token = Self::json_path(&body, &auth.token_path)
            .and_then(Value::as_str)
            .filter(|token| !token.is_empty())
            .ok_or_else(|| {
                AppError::internal(format!(
                    "webhook token response has no string at `{}`",
                    auth.token_path
                ))
            })?;
        let header_value = HeaderValue::from_str(&format!("{}{}", auth.header_prefix, token))
            .map_err(|_| AppError::internal("webhook token is not a valid header value"))?;

        let ttl_seconds = auth
            .expires_in_path
            .as_deref()
            .and_then(|path| Self::json_path(&body, path))
            .and_then(|value| {
                value
                    .as_u64()
                    .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
            })
            .filter(|ttl| *ttl > 0)
            .unwrap_or(auth.default_ttl_seconds);
        let ttl = Duration::from_secs(ttl_seconds).saturating_sub(TOKEN_EXPIRY_SKEW);

        Ok(CachedToken {
            header_value,
            expires_at: Instant::now() + ttl,
        })
    }

    /// Step 2: calls the webhook, attaching the token when auth is configured.
    async fn trigger(
        &self,
        config: &WebhookConfig,
        payload: &WebhookPayload,
        auth_header: Option<(&HeaderName, HeaderValue)>,
    ) -> AppResult<StatusCode> {
        let url = Self::validate_endpoint(&config.endpoint_url)?;
        let method = Self::request_method(config.http_method)?;
        let timeout = Duration::from_secs(config.timeout_seconds.max(1));

        let mut request = self
            .client
            .request(method, url)
            .timeout(timeout)
            .json(payload);
        if let Some((name, value)) = auth_header {
            request = request.header(name, value);
        }
        let response = request
            .send()
            .await
            .map_err(|err| Self::map_send_error("webhook", err))?;
        Ok(response.status())
    }

    /// Continues a webhook delivery after the auth config has been loaded.
    async fn deliver(
        &self,
        config: &WebhookConfig,
        payload: &WebhookPayload,
        auth: Option<&WebhookAuthConfig>,
    ) -> AppResult<()> {
        let Some(auth) = auth else {
            let status = self.trigger(config, payload, None).await?;
            return Self::check_status(status);
        };

        let header_name = HeaderName::from_bytes(auth.header_name.as_bytes())
            .map_err(|_| AppError::internal("webhook auth header_name is invalid"))?;
        let token = self.token(auth, false).await?;
        let status = self
            .trigger(config, payload, Some((&header_name, token)))
            .await?;
        if status == StatusCode::UNAUTHORIZED {
            tracing::warn!(
                webhook_config_id = config.id,
                "Webhook rejected token, refreshing"
            );
            let token = self.token(auth, true).await?;
            let status = self
                .trigger(config, payload, Some((&header_name, token)))
                .await?;
            return Self::check_status(status);
        }
        Self::check_status(status)
    }

    fn check_status(status: StatusCode) -> AppResult<()> {
        if status.is_success() {
            Ok(())
        } else {
            Err(AppError::internal(format!(
                "webhook returned HTTP {}",
                status.as_u16()
            )))
        }
    }
}

#[async_trait]
impl WebhookServiceTrait for WebhookService {
    async fn validate(&self, channel_id: i32) -> AppResult<()> {
        self.repo.find_by_channel_id(channel_id).await.map(|_| ())
    }

    async fn get_config(&self, channel_code: &str) -> AppResult<WebhookConfig> {
        let channel = self
            .channels
            .must_active(channel_code)
            .await
            .map_err(AppError::bad_request)?;
        self.repo.find_by_channel_id(channel.id).await
    }

    async fn get_webhook_auth_configs(
        &self,
        webhook_config_id: i32,
    ) -> AppResult<Option<WebhookAuthConfig>> {
        self.repo
            .find_auth_by_webhook_config_id(webhook_config_id)
            .await
    }

    async fn max_attempts(&self, channel_code: &str) -> AppResult<u32> {
        let config = self.get_config(channel_code).await?;
        Ok(config.max_retries.saturating_add(1))
    }

    async fn send_webhook(&self, request: &NotifyValue) -> AppResult<()> {
        let config = self.get_config(&request.channel).await?;
        let auth = self.get_webhook_auth_configs(config.id).await?;
        self.deliver(&config, &request.variables, auth.as_ref())
            .await
    }
}
