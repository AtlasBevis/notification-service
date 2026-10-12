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
    NotifyValue, TokenBodyFormat, WebhookAuthConfig, WebhookAuthMode, WebhookConfig, WebhookPayload,
};
use crate::repository::WebhookRepository;
use crate::services::ChannelTrait;

const TOKEN_EXPIRY_SKEW: Duration = Duration::from_secs(30);

#[async_trait]
pub trait WebhookServiceTrait: Send + Sync {
    async fn validate(&self, channel_id: i32) -> AppResult<()>;
    async fn get_config(&self, channel_id: i32) -> AppResult<WebhookConfig>;
    async fn get_webhook_auth_configs(
        &self,
        webhook_config_id: i32,
    ) -> AppResult<WebhookAuthConfig>;
    async fn max_attempts(&self, channel_id: i32) -> AppResult<u32>;
    async fn send_webhook(&self, request: &NotifyValue) -> AppResult<()>;
}

#[derive(Clone)]
pub struct WebhookService {
    client: Client,
    repo: WebhookRepository,
    channels: Arc<dyn ChannelTrait>,
    tokens: Arc<Mutex<HashMap<i32, CachedToken>>>,
}

#[derive(Clone)]
struct CachedToken {
    header_value: HeaderValue,
    expires_at: Instant,
}

impl WebhookService {
    pub fn new(client: Client, repo: WebhookRepository, channels: Arc<dyn ChannelTrait>) -> Self {
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

    fn map_send_error(step: &str, err: reqwest::Error) -> AppError {
        if err.is_timeout() {
            AppError::internal(format!("{step} request timed out"))
        } else {
            AppError::internal(format!("{step} connection failed"))
        }
    }

    fn auth_header_name(auth: &WebhookAuthConfig) -> AppResult<HeaderName> {
        let name = HeaderName::from_bytes(auth.header_name.as_bytes())
            .map_err(|_| AppError::internal("webhook auth header name is invalid"))?;
        Ok(name)
    }

    fn validate_auth(auth: &WebhookAuthConfig) -> AppResult<()> {
        Self::auth_header_name(auth)?;
        match auth.mode {
            WebhookAuthMode::Static => {
                let value = auth.header_value.as_deref().ok_or_else(|| {
                    AppError::internal("webhook static auth header value is required")
                })?;
                HeaderValue::from_str(value)
                    .map_err(|_| AppError::internal("webhook auth header value is invalid"))?;
            }
            WebhookAuthMode::TokenEndpoint => {
                let token_url = auth
                    .token_url
                    .as_deref()
                    .ok_or_else(|| AppError::internal("webhook token URL is required"))?;
                Self::validate_endpoint(token_url)?;
                Self::static_headers(&auth.request_headers)?;
                HeaderValue::from_str(&auth.header_prefix)
                    .map_err(|_| AppError::internal("webhook auth header prefix is invalid"))?;
            }
        }
        Ok(())
    }

    fn static_headers(headers: &Value) -> AppResult<HeaderMap> {
        let mut map = HeaderMap::new();
        for (name, value) in headers.as_object().into_iter().flatten() {
            let value = value
                .as_str()
                .ok_or_else(|| AppError::internal("webhook header values must be strings"))?;
            let name = HeaderName::from_bytes(name.as_bytes())
                .map_err(|_| AppError::internal("webhook header name is invalid"))?;
            let value = HeaderValue::from_str(value)
                .map_err(|_| AppError::internal("webhook header value is invalid"))?;
            map.insert(name, value);
        }
        Ok(map)
    }

    fn outgoing_headers(
        config: &WebhookConfig,
        auth: (&HeaderName, HeaderValue),
    ) -> AppResult<HeaderMap> {
        let mut headers = Self::static_headers(&config.headers)?;
        // The auth value takes precedence if a configured header uses the same name.
        headers.insert(auth.0.clone(), auth.1);
        Ok(headers)
    }

    fn token_method(method: ConfiguredMethod) -> Method {
        match method {
            ConfiguredMethod::GET => Method::GET,
            ConfiguredMethod::POST => Method::POST,
            ConfiguredMethod::PUT => Method::PUT,
            ConfiguredMethod::PATCH => Method::PATCH,
        }
    }

    fn form_fields(body: &Value) -> Vec<(String, String)> {
        body.as_object()
            .into_iter()
            .flatten()
            .map(|(key, value)| {
                let value = match value {
                    Value::String(value) => value.clone(),
                    other => other.to_string(),
                };
                (key.clone(), value)
            })
            .collect()
    }

    fn json_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
        path.split('.')
            .filter(|segment| !segment.is_empty())
            .try_fold(value, |current, segment| current.get(segment))
    }

    async fn token(&self, auth: &WebhookAuthConfig) -> AppResult<HeaderValue> {
        if let Some(cached) = self.tokens.lock().await.get(&auth.id) {
            if cached.expires_at > Instant::now() {
                return Ok(cached.header_value.clone());
            }
        }

        let fetched = self.fetch_token(auth).await?;
        let header_value = fetched.header_value.clone();
        self.tokens.lock().await.insert(auth.id, fetched);
        Ok(header_value)
    }

    async fn fetch_token(&self, auth: &WebhookAuthConfig) -> AppResult<CachedToken> {
        let token_url = auth
            .token_url
            .as_deref()
            .ok_or_else(|| AppError::internal("webhook token URL is required"))?;
        let url = Self::validate_endpoint(token_url)?;
        let method = Self::token_method(auth.token_method);
        let mut request = self
            .client
            .request(method.clone(), url)
            .timeout(Duration::from_secs(auth.timeout_seconds.max(1)))
            .headers(Self::static_headers(&auth.request_headers)?);
        if method != Method::GET {
            request = match auth.token_body_format {
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
            .ok_or_else(|| AppError::internal("webhook token response is missing the token"))?;
        let header_value = HeaderValue::from_str(&format!("{}{}", auth.header_prefix, token))
            .map_err(|_| AppError::internal("webhook token header value is invalid"))?;
        let ttl_seconds = auth
            .expires_in_path
            .as_deref()
            .and_then(|path| Self::json_path(&body, path))
            .and_then(|value| {
                value
                    .as_u64()
                    .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
            })
            .filter(|ttl| *ttl > 0)
            .unwrap_or(auth.default_ttl_seconds);
        let ttl = Duration::from_secs(ttl_seconds).saturating_sub(TOKEN_EXPIRY_SKEW);
        Ok(CachedToken {
            header_value,
            expires_at: Instant::now() + ttl,
        })
    }

    async fn auth_header(&self, auth: &WebhookAuthConfig) -> AppResult<(HeaderName, HeaderValue)> {
        Self::validate_auth(auth)?;
        let name = Self::auth_header_name(auth)?;
        let value = match auth.mode {
            WebhookAuthMode::Static => HeaderValue::from_str(
                auth.header_value
                    .as_deref()
                    .ok_or_else(|| AppError::internal("webhook static auth value is required"))?,
            )
            .map_err(|_| AppError::internal("webhook auth header value is invalid"))?,
            WebhookAuthMode::TokenEndpoint => self.token(auth).await?,
        };
        Ok((name, value))
    }

    /// Calls the webhook once, attaching the configured authentication header.
    async fn trigger(
        &self,
        config: &WebhookConfig,
        payload: &WebhookPayload,
        headers: HeaderMap,
    ) -> AppResult<StatusCode> {
        let url = Self::validate_endpoint(&config.endpoint_url)?;
        let method = Self::request_method(config.http_method)?;
        let timeout = Duration::from_secs(config.timeout_seconds.max(1));

        let request = self
            .client
            .request(method, url)
            .timeout(timeout)
            .json(payload)
            .headers(headers);
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
        auth: &WebhookAuthConfig,
    ) -> AppResult<()> {
        let (header_name, header_value) = self.auth_header(auth).await?;
        let headers = Self::outgoing_headers(config, (&header_name, header_value))?;
        let status = self.trigger(config, payload, headers).await?;
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
        let config = self.repo.find_by_channel_id(channel_id).await?;
        Self::static_headers(&config.headers)?;
        let auth = self.repo.find_auth_by_webhook_config_id(config.id).await?;
        Self::validate_auth(&auth)?;
        Ok(())
    }

    async fn get_config(&self, channel_id: i32) -> AppResult<WebhookConfig> {
        self.channels.get_active_by_id(channel_id).await?;
        self.repo.find_by_channel_id(channel_id).await
    }

    async fn get_webhook_auth_configs(
        &self,
        webhook_config_id: i32,
    ) -> AppResult<WebhookAuthConfig> {
        self.repo
            .find_auth_by_webhook_config_id(webhook_config_id)
            .await
    }

    async fn max_attempts(&self, _channel_id: i32) -> AppResult<u32> {
        Ok(1)
    }

    async fn send_webhook(&self, request: &NotifyValue) -> AppResult<()> {
        let config = self.get_config(request.channel_id).await?;
        let auth = self.get_webhook_auth_configs(config.id).await?;
        self.deliver(&config, &request.variables, &auth).await
    }
}
