use std::time::Duration;

use async_trait::async_trait;
use reqwest::{Client, Method, Url};

use crate::enums::Method as ConfiguredMethod;
use crate::error::{AppError, AppResult};
use crate::models::{WebhookConfig, WebhookPayload};

#[async_trait]
pub trait WebhookServiceTrait: Send + Sync {
    async fn send_webhook(&self, config: &WebhookConfig, payload: &WebhookPayload)
        -> AppResult<()>;
}

#[derive(Clone)]
pub struct WebhookService {
    client: Client,
}

impl WebhookService {
    pub fn new(client: Client) -> Self {
        Self { client }
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
}

#[async_trait]
impl WebhookServiceTrait for WebhookService {
    async fn send_webhook(
        &self,
        config: &WebhookConfig,
        payload: &WebhookPayload,
    ) -> AppResult<()> {
        let url = Self::validate_endpoint(&config.endpoint_url)?;
        let method = Self::request_method(config.http_method)?;
        let timeout = Duration::from_secs(config.timeout_seconds.max(1));

        let response = self
            .client
            .request(method, url)
            .timeout(timeout)
            .json(payload)
            .send()
            .await
            .map_err(|err| {
                if err.is_timeout() {
                    AppError::internal("webhook request timed out")
                } else {
                    AppError::internal("webhook connection failed")
                }
            })?;

        if !response.status().is_success() {
            return Err(AppError::internal(format!(
                "webhook returned HTTP {}",
                response.status().as_u16()
            )));
        }
        Ok(())
    }
}
