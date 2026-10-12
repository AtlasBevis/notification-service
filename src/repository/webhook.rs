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

    pub async fn find_by_id(&self, id: i32) -> AppResult<WebhookConfig> {
        let client = self.pool.get().await?;
        let row = client
            .query_opt(
                "SELECT id, target_id, service_name, domain, path, method, timeout_seconds, max_retries,
                        headers
                 FROM webhook_configs
                 WHERE id = $1",
                &[&id],
            )
            .await?
            .ok_or_else(|| AppError::not_found(format!("webhook config id `{id}` not found")))?;

        WebhookConfig::map_row(&row)
    }

    pub async fn find_by_definition_id(&self, definition_id: i64) -> AppResult<Vec<WebhookConfig>> {
        let client = self.pool.get().await?;
        let rows = client
            .query(
                "SELECT w.id, w.target_id, w.service_name, w.domain, w.path, w.method,
                    w.timeout_seconds, w.max_retries, w.headers
             FROM definition_targets dt
             JOIN webhook_configs w ON w.target_id = dt.target_id
             WHERE dt.definition_id = $1 ORDER BY dt.id",
                &[&definition_id],
            )
            .await?;
        if rows.is_empty() {
            return Err(AppError::not_found(format!(
                "webhook routes for definition `{definition_id}` not found"
            )));
        }
        rows.iter().map(WebhookConfig::map_row).collect()
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
