use crate::error::{AppError, AppResult};
use crate::models::{WebhookAuthConfig, WebhookConfig};
use deadpool_postgres::Pool;

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
                "SELECT id, channel_id, domain, path, method, timeout_seconds, max_retries,
                        headers
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

        WebhookConfig::map_row(&row)
    }

    /// Loads the required authentication configuration for a webhook config.
    pub async fn find_auth_by_webhook_config_id(
        &self,
        webhook_config_id: i32,
    ) -> AppResult<WebhookAuthConfig> {
        let client = self.pool.get().await?;
        let row = client
            .query_opt(
                "SELECT id AS auth_id, auth_mode, header_name, header_value, token_url,
                        token_method, token_body_format, request_headers, request_body,
                        timeout_seconds, token_path, expires_in_path,
                        default_ttl_seconds, header_prefix
                 FROM webhook_auth_configs
                 WHERE webhook_config_id = $1",
                &[&webhook_config_id],
            )
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!(
                    "webhook auth config for webhook config id `{webhook_config_id}` not found"
                ))
            })?;
        WebhookAuthConfig::map_row(&row)
    }
}
