use deadpool_postgres::{GenericClient, Pool};
use serde_json::Value;

use crate::error::{AppError, AppResult};
use crate::models::{SaveWebhookRequest, WebhookAuthConfig, WebhookConfig, WebhookData};
use crate::utils::TimeUtils;

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
                r#"
                SELECT
                    w.id,
                    w.target_id,
                    t.code AS service_name,
                    w.domain,
                    w.path,
                    w.method,
                    w.timeout_seconds,
                    w.max_retries,
                    w.headers
                FROM webhook_configs w
                JOIN targets t
                    ON t.id = w.target_id
                WHERE w.id = $1
                "#,
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
                r#"
                SELECT
                    w.id,
                    w.target_id,
                    t.code AS service_name,
                    w.domain,
                    w.path,
                    w.method,
                    w.timeout_seconds,
                    w.max_retries,
                    w.headers
                FROM definition_targets dt
                JOIN webhook_configs w
                    ON w.target_id = dt.target_id
                JOIN targets t
                    ON t.id = w.target_id
                WHERE dt.definition_id = $1
                ORDER BY dt.id
                "#,
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
                r#"
                SELECT id AS auth_id, auth_mode, header_name, header_value, token_url,
                       token_method, token_body_format, request_headers, request_body,
                       timeout_seconds, token_path, expires_in_path,
                       default_ttl_seconds, header_prefix
                FROM webhook_auth_configs
                WHERE webhook_config_id = $1
                "#,
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

    /// All webhook configs with their auth config, ordered by target code.
    pub async fn list(&self) -> AppResult<Vec<WebhookData>> {
        let client = self.pool.get().await?;
        Self::select(&client, None).await
    }

    pub async fn find_by_target_code(&self, target_code: &str) -> AppResult<WebhookData> {
        let client = self.pool.get().await?;
        Self::select(&client, Some(target_code))
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| {
                AppError::not_found(format!("webhook for target `{target_code}` not found"))
            })
    }

    /// Inserts the webhook config and its auth config in one transaction.
    pub async fn insert(&self, target_id: i32, req: &SaveWebhookRequest) -> AppResult<WebhookData> {
        let mut client = self.pool.get().await?;
        let tx = client.transaction().await?;

        let row = tx
            .query_one(
                r#"
                INSERT INTO webhook_configs (target_id, domain, path, method, timeout_seconds, max_retries, headers, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                RETURNING id
                "#,
                &[
                    &target_id,
                    &req.domain,
                    &req.path,
                    &req.method,
                    &req.timeout_seconds,
                    &req.max_retries,
                    &Value::Object(req.headers.clone()),
                    &TimeUtils::now(),
                ],
            )
            .await?;
        let config_id: i32 = row.get("id");
        Self::upsert_auth(&tx, config_id, req).await?;

        let data = Self::select(&tx, Some(&req.target_code))
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| AppError::internal("inserted webhook config cannot be read back"))?;
        tx.commit().await?;
        Ok(data)
    }

    /// Replaces the webhook config and its auth config of `target_id`.
    pub async fn update(&self, target_id: i32, req: &SaveWebhookRequest) -> AppResult<WebhookData> {
        let mut client = self.pool.get().await?;
        let tx = client.transaction().await?;

        let row = tx
            .query_opt(
                r#"
                UPDATE webhook_configs
                SET domain = $2,
                    path = $3,
                    method = $4,
                    timeout_seconds = $5,
                    max_retries = $6,
                    headers = $7,
                    updated_at = $8
                WHERE target_id = $1
                RETURNING id
                "#,
                &[
                    &target_id,
                    &req.domain,
                    &req.path,
                    &req.method,
                    &req.timeout_seconds,
                    &req.max_retries,
                    &Value::Object(req.headers.clone()),
                    &TimeUtils::now(),
                ],
            )
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!(
                    "webhook for target `{}` not found",
                    req.target_code
                ))
            })?;
        let config_id: i32 = row.get("id");
        Self::upsert_auth(&tx, config_id, req).await?;

        let data = Self::select(&tx, Some(&req.target_code))
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| AppError::internal("updated webhook config cannot be read back"))?;
        tx.commit().await?;
        Ok(data)
    }

    /// The auth config goes with it (`ON DELETE CASCADE`).
    pub async fn delete(&self, config_id: i32) -> AppResult<()> {
        let client = self.pool.get().await?;
        let n = client
            .execute(
                r#"
                DELETE FROM webhook_configs
                WHERE id = $1
                "#,
                &[&config_id],
            )
            .await?;
        if n == 0 {
            return Err(AppError::not_found(format!(
                "webhook config id `{config_id}` not found"
            )));
        }
        Ok(())
    }

    async fn upsert_auth(
        client: &impl GenericClient,
        config_id: i32,
        req: &SaveWebhookRequest,
    ) -> AppResult<()> {
        let auth = &req.auth;
        client
            .execute(
                r#"
                INSERT INTO webhook_auth_configs (
                    webhook_config_id, auth_mode, header_name, header_value, header_prefix,
                    token_url, token_method, token_body_format, request_headers, request_body,
                    timeout_seconds, token_path, expires_in_path, default_ttl_seconds, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
                ON CONFLICT (webhook_config_id) DO UPDATE
                SET auth_mode = EXCLUDED.auth_mode,
                    header_name = EXCLUDED.header_name,
                    header_value = EXCLUDED.header_value,
                    header_prefix = EXCLUDED.header_prefix,
                    token_url = EXCLUDED.token_url,
                    token_method = EXCLUDED.token_method,
                    token_body_format = EXCLUDED.token_body_format,
                    request_headers = EXCLUDED.request_headers,
                    request_body = EXCLUDED.request_body,
                    timeout_seconds = EXCLUDED.timeout_seconds,
                    token_path = EXCLUDED.token_path,
                    expires_in_path = EXCLUDED.expires_in_path,
                    default_ttl_seconds = EXCLUDED.default_ttl_seconds,
                    updated_at = EXCLUDED.updated_at
                "#,
                &[
                    &config_id,
                    &auth.auth_mode,
                    &auth.header_name,
                    &auth.header_value,
                    &auth.header_prefix,
                    &auth.token_url,
                    &auth.token_method,
                    &auth.token_body_format,
                    &Value::Object(auth.request_headers.clone()),
                    &Value::Object(auth.request_body.clone()),
                    &auth.timeout_seconds,
                    &auth.token_path,
                    &auth.expires_in_path,
                    &auth.default_ttl_seconds,
                    &TimeUtils::now(),
                ],
            )
            .await?;
        Ok(())
    }

    /// Webhook configs joined with their target and auth config; all of them
    /// when `target_code` is `None`. Secret columns are not selected.
    async fn select(
        client: &impl GenericClient,
        target_code: Option<&str>,
    ) -> AppResult<Vec<WebhookData>> {
        let rows = client
            .query(
                r#"
                SELECT
                    w.id,
                    w.target_id,
                    t.code AS target_code,
                    w.domain,
                    w.path,
                    w.method,
                    w.timeout_seconds,
                    w.max_retries,
                    w.headers,
                    w.created_at,
                    w.updated_at,
                    a.id AS auth_id,
                    a.auth_mode,
                    a.header_name,
                    COALESCE(a.header_value, '') <> '' AS has_header_value,
                    a.header_prefix,
                    a.token_url,
                    a.token_method,
                    a.token_body_format,
                    a.token_path,
                    a.expires_in_path,
                    a.timeout_seconds AS auth_timeout_seconds,
                    a.default_ttl_seconds
                FROM webhook_configs w
                JOIN targets t
                    ON t.id = w.target_id
                LEFT JOIN webhook_auth_configs a
                    ON a.webhook_config_id = w.id
                WHERE $1::text IS NULL
                    OR t.code = $1
                ORDER BY t.code
                "#,
                &[&target_code],
            )
            .await?;
        rows.iter().map(WebhookData::map_row).collect()
    }
}
