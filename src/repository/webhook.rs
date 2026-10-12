use deadpool_postgres::Pool;

use crate::enums::Method;
use crate::error::{AppError, AppResult};
use crate::models::WebhookConfig;

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
}
