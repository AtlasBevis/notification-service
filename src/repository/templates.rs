use chrono::Utc;
use deadpool_postgres::Pool;

use crate::models::{CreateTemplateRequest, TemplateData};

use crate::error::{AppError, AppResult};

const COLUMNS: &str = r#"
    t.id, t.name, t.status,
    v.id AS version_id, v.version, v.status AS version_status,
    v.title, v.content, v.format,
    t.created_at, t.updated_at
"#;

#[derive(Clone)]
pub struct TemplateRepository {
    pool: Pool,
}

impl TemplateRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Template with its latest ACTIVE version.
    pub async fn find_by_id(&self, id: i32) -> AppResult<TemplateData> {
        let client = self.pool.get().await?;
        let sql = format!(
            r#"
            SELECT {COLUMNS}
            FROM templates t
            JOIN template_versions v ON v.template_id = t.id
            WHERE t.id = $1 AND v.status = 'ACTIVE'
            ORDER BY v.version DESC
            LIMIT 1
            "#
        );
        let row = client.query_opt(sql.as_str(), &[&id]).await?;
        let row = row.ok_or_else(|| {
            AppError::not_found(format!("template id `{id}` has no ACTIVE version"))
        })?;
        TemplateData::map_row(&row)
    }

    /// Creates the template on first use of `name`, then appends the next version.
    pub async fn insert_version(&self, req: &CreateTemplateRequest) -> AppResult<TemplateData> {
        let mut client = self.pool.get().await?;
        let tx = client.transaction().await?;
        let name = req.name.trim().to_string();
        let title = req.title.trim().to_string();
        let content = req.content.clone();
        let format = req.format.trim().to_ascii_uppercase();
        let status = req.status.trim().to_ascii_uppercase();
        let now = Utc::now();

        // The upsert locks the template row, so concurrent creates for one name
        // take consecutive version numbers.
        let template = tx
            .query_one(
                r#"
                INSERT INTO templates (name, updated_at)
                VALUES ($1, $2)
                ON CONFLICT (name) DO UPDATE SET updated_at = EXCLUDED.updated_at
                RETURNING id
                "#,
                &[&name, &now],
            )
            .await?;
        let template_id: i32 = template.get("id");

        let version = tx
            .query_one(
                r#"
                INSERT INTO template_versions (template_id, version, status, title, content, format)
                SELECT $1, COALESCE(MAX(version), 0) + 1, $2, $3, $4, $5
                FROM template_versions
                WHERE template_id = $1
                RETURNING id
                "#,
                &[&template_id, &status, &title, &content, &format],
            )
            .await?;
        let version_id: i32 = version.get("id");

        let sql = format!(
            r#"
            SELECT {COLUMNS}
            FROM templates t
            JOIN template_versions v ON v.template_id = t.id
            WHERE v.id = $1
            "#
        );
        let row = tx.query_one(sql.as_str(), &[&version_id]).await?;
        let data = TemplateData::map_row(&row)?;
        tx.commit().await?;
        Ok(data)
    }
}
