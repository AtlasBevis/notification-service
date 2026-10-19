use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Method, StatusCode, Url};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::enums::Channel;
use crate::enums::Method as ConfiguredMethod;
use crate::error::{AppError, AppResult};
use crate::infras::http::HttpClient;
use crate::models::{
    DefinitionData, NotifyValue, PageData, SaveWebhookRequest, TokenBodyFormat, WebhookAuthConfig,
    WebhookAuthMode, WebhookConfig, WebhookData, WebhookPayload,
};
use crate::repository::WebhookRepository;
use crate::services::TargetTrait;

const TOKEN_EXPIRY_SKEW: Duration = Duration::from_secs(30);

#[async_trait]
pub trait WebhookServiceTrait: Send + Sync {
    /// Checks the webhook config and auth config a WEBHOOK definition points at.
    async fn validate(&self, definition: &DefinitionData) -> AppResult<()>;
    async fn get_config(&self, webhook_config_id: i32) -> AppResult<WebhookConfig>;
    async fn get_webhook_auth_configs(
        &self,
        webhook_config_id: i32,
    ) -> AppResult<WebhookAuthConfig>;
    async fn max_attempts(&self, definition_id: i64) -> AppResult<u32>;
    async fn send_webhook(&self, request: &NotifyValue) -> AppResult<()>;
    async fn list(&self) -> AppResult<PageData<WebhookData>>;
    async fn get_by_target_code(&self, target_code: &str) -> AppResult<WebhookData>;
    async fn create(&self, req: SaveWebhookRequest) -> AppResult<WebhookData>;
    async fn update(&self, req: SaveWebhookRequest) -> AppResult<WebhookData>;
    async fn delete(&self, target_code: &str) -> AppResult<()>;
}

#[derive(Clone)]
pub struct WebhookService {
    client: Arc<HttpClient>,
    repo: WebhookRepository,
    targets: Arc<dyn TargetTrait>,
    tokens: Arc<Mutex<HashMap<i32, CachedToken>>>,
}

#[derive(Clone)]
struct CachedToken {
    header_value: HeaderValue,
    expires_at: Instant,
}

impl WebhookService {
    pub fn new(
        client: Arc<HttpClient>,
        repo: WebhookRepository,
        targets: Arc<dyn TargetTrait>,
    ) -> Self {
        Self {
            client,
            repo,
            targets,
            tokens: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// The id of WEBHOOK target `code`; a missing or non-WEBHOOK target is a bad request.
    async fn webhook_target_id(&self, code: &str) -> AppResult<i32> {
        let target = match self.targets.get_by_code(code).await {
            Ok(target) => target,
            Err(AppError::NotFound(_)) => {
                return Err(AppError::bad_request(format!("unknown target `{code}`")))
            }
            Err(err) => return Err(err),
        };
        if target.channel_code != Channel::WEBHOOK {
            return Err(AppError::bad_request(format!(
                "target `{code}` belongs to channel `{}`, not WEBHOOK",
                target.channel_code
            )));
        }
        Ok(target.id)
    }

    /// Drops a cached token so the next send fetches one with the new settings.
    async fn forget_token(&self, data: &WebhookData) {
        if let Some(auth) = &data.auth {
            self.tokens.lock().await.remove(&auth.id);
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

    fn map_send_error(step: &str, err: AppError) -> AppError {
        if err.to_string().to_ascii_lowercase().contains("timed out") {
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

    fn plain_headers(headers: &HeaderMap) -> AppResult<HashMap<String, String>> {
        headers
            .iter()
            .map(|(name, value)| {
                let value = value
                    .to_str()
                    .map_err(|_| AppError::internal("webhook header value is invalid"))?;
                Ok((name.as_str().to_string(), value.to_string()))
            })
            .collect()
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

    fn form_body(fields: &[(String, String)]) -> Vec<u8> {
        fn encode(value: &str) -> String {
            let mut encoded = String::new();
            for byte in value.bytes() {
                if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'*') {
                    encoded.push(char::from(byte));
                } else if byte == b' ' {
                    encoded.push('+');
                } else {
                    encoded.push_str(&format!("%{byte:02X}"));
                }
            }
            encoded
        }
        fields
            .iter()
            .map(|(key, value)| format!("{}={}", encode(key), encode(value)))
            .collect::<Vec<_>>()
            .join("&")
            .into_bytes()
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
        Self::validate_endpoint(token_url)?;
        let method = Self::token_method(auth.token_method);
        let headers = Self::plain_headers(&Self::static_headers(&auth.request_headers)?)?;
        let body = if method == Method::GET {
            None
        } else {
            Some(match auth.token_body_format {
                TokenBodyFormat::Json => serde_json::to_vec(&auth.request_body).map_err(|_| {
                    AppError::internal("webhook token request body serialization failed")
                })?,
                TokenBodyFormat::Form => Self::form_body(&Self::form_fields(&auth.request_body)),
            })
        };
        let response = self
            .client
            .request_with_timeout(
                method,
                token_url,
                &headers,
                body,
                Duration::from_secs(auth.timeout_seconds.max(1)),
            )
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
        Self::validate_endpoint(&config.endpoint_url)?;
        let method = Self::request_method(config.http_method)?;
        let timeout = Duration::from_secs(config.timeout_seconds.max(1));
        let headers = Self::plain_headers(&headers)?;
        let body = serde_json::to_vec(payload)
            .map_err(|_| AppError::internal("webhook payload serialization failed"))?;
        let response = self
            .client
            .request_with_timeout(method, &config.endpoint_url, &headers, Some(body), timeout)
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

    fn merge_payload(value: &Value, variables: &serde_json::Map<String, Value>) -> Value {
        match value {
            Value::String(template) => {
                let trimmed = template.trim();
                if trimmed.starts_with("{{") && trimmed.ends_with("}}") && trimmed.len() > 4 {
                    let key = trimmed[2..trimmed.len() - 2].trim();
                    if let Some(value) = Self::variable_path(variables, key) {
                        return value.clone();
                    }
                }
                let mut rendered = template.clone();
                for (key, value) in variables {
                    let replacement = match value {
                        Value::String(text) => text.clone(),
                        other => other.to_string(),
                    };
                    rendered = rendered.replace(&format!("{{{{{key}}}}}"), &replacement);
                }
                Value::String(rendered)
            }
            Value::Array(items) => Value::Array(
                items
                    .iter()
                    .map(|item| Self::merge_payload(item, variables))
                    .collect(),
            ),
            Value::Object(items) => Value::Object(
                items
                    .iter()
                    .map(|(key, value)| (key.clone(), Self::merge_payload(value, variables)))
                    .collect(),
            ),
            other => other.clone(),
        }
    }

    fn variable_path<'a>(
        variables: &'a serde_json::Map<String, Value>,
        path: &str,
    ) -> Option<&'a Value> {
        let mut segments = path.split('.');
        let first = segments.next()?;
        segments.try_fold(variables.get(first)?, |value, segment| value.get(segment))
    }
}

#[async_trait]
impl WebhookServiceTrait for WebhookService {
    async fn validate(&self, definition: &DefinitionData) -> AppResult<()> {
        let configs = self.repo.find_by_definition_id(definition.id).await?;
        if configs.len() != definition.targets.len() {
            return Err(AppError::bad_request(format!(
                "definition `{}` has a WEBHOOK target without webhook_configs",
                definition.code
            )));
        }
        for config in configs {
            Self::validate_endpoint(&config.endpoint_url)?;
            Self::request_method(config.http_method)?;
            Self::static_headers(&config.headers)?;
            let auth = self.repo.find_auth_by_webhook_config_id(config.id).await?;
            Self::validate_auth(&auth)?;
        }
        Ok(())
    }

    async fn get_config(&self, webhook_config_id: i32) -> AppResult<WebhookConfig> {
        let config = self.repo.find_by_id(webhook_config_id).await?;
        Ok(config)
    }

    async fn get_webhook_auth_configs(
        &self,
        webhook_config_id: i32,
    ) -> AppResult<WebhookAuthConfig> {
        self.repo
            .find_auth_by_webhook_config_id(webhook_config_id)
            .await
    }

    async fn max_attempts(&self, definition_id: i64) -> AppResult<u32> {
        let configs = self.repo.find_by_definition_id(definition_id).await?;
        Ok(configs
            .iter()
            .map(|config| config.max_retries.saturating_add(1))
            .max()
            .unwrap_or(1))
    }

    async fn send_webhook(&self, request: &NotifyValue) -> AppResult<()> {
        let configs = self
            .repo
            .find_by_definition_id(request.definition_id)
            .await?;
        let mut failures = Vec::new();
        for config in configs {
            let result = async {
                let auth = self.repo.find_auth_by_webhook_config_id(config.id).await?;
                // A target without a template sends the request variables as-is.
                let configured_payload = request
                    .target_payloads
                    .iter()
                    .find(|item| item.target_id == config.target_id)
                    .map(|item| &item.payload);
                let Some(configured_payload) = configured_payload else {
                    return self.deliver(&config, &request.variables, &auth).await;
                };
                let payload = Self::merge_payload(configured_payload, &request.variables);
                let payload = payload.as_object().ok_or_else(|| {
                    AppError::bad_request(format!(
                        "webhook payload for target `{}` must be a JSON object",
                        config.target_id
                    ))
                })?;
                self.deliver(&config, payload, &auth).await
            }
            .await;
            if let Err(error) = result {
                failures.push(format!("{}: {}", config.service_name, error));
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(AppError::internal(format!(
                "webhook delivery failed: {}",
                failures.join("; ")
            )))
        }
    }

    async fn list(&self) -> AppResult<PageData<WebhookData>> {
        Ok(PageData::new(self.repo.list().await?))
    }

    async fn get_by_target_code(&self, target_code: &str) -> AppResult<WebhookData> {
        self.repo.find_by_target_code(target_code).await
    }

    async fn create(&self, req: SaveWebhookRequest) -> AppResult<WebhookData> {
        let target_id = self.webhook_target_id(&req.target_code).await?;
        match self.repo.insert(target_id, &req).await {
            Ok(data) => {
                tracing::info!(id = data.id, target = %data.target_code, "Webhook config created");
                Ok(data)
            }
            Err(AppError::Conflict(_)) => Err(AppError::conflict(format!(
                "webhook for target `{}` already exists",
                req.target_code
            ))),
            Err(err) => {
                tracing::error!(error = %err, target = %req.target_code, "create webhook config failed");
                Err(err)
            }
        }
    }

    async fn update(&self, req: SaveWebhookRequest) -> AppResult<WebhookData> {
        let target_id = self.webhook_target_id(&req.target_code).await?;
        match self.repo.update(target_id, &req).await {
            Ok(data) => {
                self.forget_token(&data).await;
                tracing::info!(id = data.id, target = %data.target_code, "Webhook config updated");
                Ok(data)
            }
            Err(err @ AppError::NotFound(_)) => Err(err),
            Err(err) => {
                tracing::error!(error = %err, target = %req.target_code, "update webhook config failed");
                Err(err)
            }
        }
    }

    async fn delete(&self, target_code: &str) -> AppResult<()> {
        let data = self.repo.find_by_target_code(target_code).await?;
        self.repo.delete(data.id).await?;
        self.forget_token(&data).await;
        tracing::info!(id = data.id, target = %target_code, "Webhook config deleted");
        Ok(())
    }
}
