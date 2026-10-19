use deadpool_postgres::{Object, Pool, Transaction};

use crate::enums::Status;
use crate::error::{AppError, AppResult};
use crate::models::{SaveTemplateRequest, TemplateData};

#[derive(Clone)]
pub struct TemplateRepository {
    pool: Pool,
}

impl TemplateRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// A pooled connection to open a transaction on; the methods that take
    /// `tx` run inside that transaction.
    pub async fn client(&self) -> AppResult<Object> {
        Ok(self.pool.get().await?)
    }

    /// Template with its latest ACTIVE version.
    pub async fn find_by_id(&self, id: i32) -> AppResult<TemplateData> {
        let client = self.pool.get().await?;
        let row = client
            .query_opt(
                r#"
                SELECT
                    t.id,
                    t.code,
                    t.name,
                    t.status,
                    t.created_at,
                    t.updated_at,
                    v.id AS version_id,
                    v.version,
                    v.status AS version_status,
                    v.title,
                    v.content,
                    v.format
                FROM templates t
                JOIN template_versions v
                    ON v.template_id = t.id
                WHERE t.id = $1
                    AND v.status = 'ACTIVE'
                ORDER BY v.version DESC
                LIMIT 1
                "#,
                &[&id],
            )
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!("template id `{id}` has no ACTIVE version"))
            })?;
        TemplateData::map_row(&row)
    }

    /// Every template in `ids` with its latest ACTIVE version, in one query.
    /// Ids without an ACTIVE version are missing from the result.
    pub async fn find_by_ids(&self, ids: &[i32]) -> AppResult<Vec<TemplateData>> {
        let client = self.pool.get().await?;
        let rows = client
            .query(
                r#"
                SELECT DISTINCT ON (t.id)
                    t.id,
                    t.code,
                    t.name,
                    t.status,
                    t.created_at,
                    t.updated_at,
                    v.id AS version_id,
                    v.version,
                    v.status AS version_status,
                    v.title,
                    v.content,
                    v.format
                FROM templates t
                JOIN template_versions v
                    ON v.template_id = t.id
                WHERE t.id = ANY($1)
                    AND v.status = 'ACTIVE'
                ORDER BY t.id, v.version DESC
                "#,
                &[&ids],
            )
            .await?;
        rows.iter().map(TemplateData::map_row).collect()
    }

    /// Template with its latest ACTIVE version.
    pub async fn find_by_code(&self, code: &str) -> AppResult<TemplateData> {
        let client = self.pool.get().await?;
        let row = client
            .query_opt(
                r#"
                SELECT
                    t.id,
                    t.code,
                    t.name,
                    t.status,
                    t.created_at,
                    t.updated_at,
                    v.id AS version_id,
                    v.version,
                    v.status AS version_status,
                    v.title,
                    v.content,
                    v.format
                FROM templates t
                JOIN template_versions v
                    ON v.template_id = t.id
                WHERE t.code = $1
                    AND v.status = 'ACTIVE'
                ORDER BY v.version DESC
                LIMIT 1
                "#,
                &[&code],
            )
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!("template code `{code}` has no ACTIVE version"))
            })?;
        TemplateData::map_row(&row)
    }

    /// The template joined with one specific version, whatever its status.
    pub async fn find_by_version_id(
        &self,
        tx: &Transaction<'_>,
        version_id: i32,
    ) -> AppResult<TemplateData> {
        let row = tx
            .query_one(
                r#"
                SELECT
                    t.id,
                    t.code,
                    t.name,
                    t.status,
                    t.created_at,
                    t.updated_at,
                    v.id AS version_id,
                    v.version,
                    v.status AS version_status,
                    v.title,
                    v.content,
                    v.format
                FROM templates t
                JOIN template_versions v
                    ON v.template_id = t.id
                WHERE v.id = $1
                "#,
                &[&version_id],
            )
            .await?;
        TemplateData::map_row(&row)
    }

    /// Locks the template row of `code`; waits while another transaction holds it.
    pub async fn lock_by_code(&self, tx: &Transaction<'_>, code: &str) -> AppResult<Option<i32>> {
        let row = tx
            .query_opt(
                r#"
                SELECT id
                FROM templates
                WHERE code = $1
                FOR UPDATE
                "#,
                &[&code],
            )
            .await?;
        Ok(row.map(|r| r.get("id")))
    }

    /// `None` when another transaction inserted `code` first; the insert waits
    /// for that transaction to finish before deciding.
    pub async fn insert_template(
        &self,
        tx: &Transaction<'_>,
        code: &str,
        name: &str,
    ) -> AppResult<Option<i32>> {
        let row = tx
            .query_opt(
                r#"
                INSERT INTO templates (code, name)
                VALUES ($1, $2)
                ON CONFLICT (code) DO NOTHING
                RETURNING id
                "#,
                &[&code, &name],
            )
            .await?;
        Ok(row.map(|r| r.get("id")))
    }

    /// Appends version `max(version) + 1` as ACTIVE and returns its id.
    pub async fn insert_version(
        &self,
        tx: &Transaction<'_>,
        template_id: i32,
        req: &SaveTemplateRequest,
    ) -> AppResult<i32> {
        let row = tx
            .query_one(
                r#"
                INSERT INTO template_versions (template_id, version, status, title, content, format)
                SELECT $1, COALESCE(MAX(version), 0) + 1, $2, $3, $4, $5
                FROM template_versions
                WHERE template_id = $1
                RETURNING id
                "#,
                &[
                    &template_id,
                    &Status::Active.as_str(),
                    &req.title,
                    &req.content,
                    &req.format.as_str(),
                ],
            )
            .await?;
        Ok(row.get("id"))
    }
}
