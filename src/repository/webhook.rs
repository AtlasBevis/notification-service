use deadpool_postgres::Pool;
use serde_json::Value;
use tokio_postgres::Row;

use crate::enums::Method;
use crate::error::{AppError, AppResult};
use crate::models::{TokenBodyFormat, WebhookAuthConfig, WebhookConfig};

#[derive(Clone)]
pub struct WebhookRepository {
    pool: Pool,
}

impl WebhookRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn find_by_channel_id(&self, channel_id: i32) -> AppResult<WebhookConfig> {
        let client = self.pool.get().await?;
        let row = client
            .query_opt(
                "SELECT id, channel_id, domain, path, method, timeout_seconds, max_retries
                 FROM webhook_configs
                 WHERE channel_id = $1",
                &[&channel_id],
            )
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!(
                    "webhook config for channel id `{channel_id}` not found"
                ))
            })?;

        let domain: String = row.get("domain");
        let path: Option<String> = row.get("path");
        let configured_method: String = row.get("method");
        let http_method = Method::parse(&configured_method.to_ascii_uppercase())
            .ok_or_else(|| AppError::internal("webhook config has an invalid HTTP method"))?;
        let timeout_seconds: i32 = row.get("timeout_seconds");
        let max_retries: i32 = row.get("max_retries");
        if timeout_seconds <= 0 || max_retries < 0 {
            return Err(AppError::internal(
                "webhook config timeout or max_retries is invalid",
            ));
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

        Ok(WebhookConfig {
            id: row.get("id"),
            channel_id,
            endpoint_url,
            http_method,
            timeout_seconds: timeout_seconds as u64,
            max_retries: max_retries as u32,
        })
    }

    /// Token step of a webhook config; `None` => the webhook is called without auth.
    pub async fn find_auth_by_webhook_config_id(
        &self,
        webhook_config_id: i32,
    ) -> AppResult<Option<WebhookAuthConfig>> {
        let client = self.pool.get().await?;
        let row = client
            .query_opt(
                "SELECT id                  AS auth_id,
                        token_url           AS auth_token_url,
                        method              AS auth_method,
                        body_format         AS auth_body_format,
                        request_headers     AS auth_request_headers,
                        request_body        AS auth_request_body,
                        timeout_seconds     AS auth_timeout_seconds,
                        token_path          AS auth_token_path,
                        expires_in_path     AS auth_expires_in_path,
                        default_ttl_seconds AS auth_default_ttl_seconds,
                        header_name         AS auth_header_name,
                        header_prefix       AS auth_header_prefix
                 FROM webhook_auth_configs
                 WHERE webhook_config_id = $1",
                &[&webhook_config_id],
            )
            .await?;
        row.as_ref().map(Self::auth_from_row).transpose()
    }

    fn auth_from_row(row: &Row) -> AppResult<WebhookAuthConfig> {
        let id: i32 = row.get("auth_id");
        let configured_method: String = row.get("auth_method");
        let http_method = Method::parse(&configured_method.to_ascii_uppercase())
            .ok_or_else(|| AppError::internal("webhook auth config has an invalid HTTP method"))?;
        let configured_format: String = row.get("auth_body_format");
        let body_format = TokenBodyFormat::parse(&configured_format.to_ascii_uppercase())
            .ok_or_else(|| AppError::internal("webhook auth config has an invalid body_format"))?;
        let request_headers: Value = row.get("auth_request_headers");
        let request_body: Value = row.get("auth_request_body");
        if !request_headers.is_object() || !request_body.is_object() {
            return Err(AppError::internal(
                "webhook auth config request_headers and request_body must be JSON objects",
            ));
        }
        let timeout_seconds: i32 = row.get("auth_timeout_seconds");
        let default_ttl_seconds: i32 = row.get("auth_default_ttl_seconds");
        if timeout_seconds <= 0 || default_ttl_seconds <= 0 {
            return Err(AppError::internal(
                "webhook auth config timeout or default_ttl_seconds is invalid",
            ));
        }
        let token_path: String = row.get("auth_token_path");
        if token_path.trim().is_empty() {
            return Err(AppError::internal(
                "webhook auth config token_path is empty",
            ));
        }
        let expires_in_path: Option<String> = row.get("auth_expires_in_path");

        Ok(WebhookAuthConfig {
            id,
            token_url: row.get("auth_token_url"),
            http_method,
            body_format,
            request_headers,
            request_body,
            timeout_seconds: timeout_seconds as u64,
            token_path,
            expires_in_path: expires_in_path.filter(|p| !p.trim().is_empty()),
            default_ttl_seconds: default_ttl_seconds as u64,
            header_name: row.get("auth_header_name"),
            header_prefix: row.get("auth_header_prefix"),
        })
    }
}
