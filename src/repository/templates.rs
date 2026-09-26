use chrono::Utc;
use deadpool_postgres::Pool;

use crate::models::{CreateTemplateRequest, TemplateData};

use crate::error::AppError;

#[derive(Clone)]
pub struct TemplatesRepository {
    pool: Pool,
}

impl TemplatesRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn find_by_id(&self, id: i64) -> Result<Option<TemplateData>, AppError> {
        let client = self.pool.get().await?;
        let row = client
            .query_opt(
                "SELECT id, name, status, title, content, format, version, created_at, updated_at
                 FROM templates
                 WHERE id = $1",
                &[&id],
            )
            .await?;
        Ok(row.as_ref().map(TemplateData::map_row))
    }

    pub async fn next_version(&self, name: &str) -> Result<i32, AppError> {
        let client = self.pool.get().await?;
        let row = client
            .query_one(
                "SELECT COALESCE(MAX(version), 0) + 1 AS next_version FROM templates WHERE name = $1",
                &[&name],
            )
            .await?;
        Ok(row.get::<_, i32>("next_version"))
    }

    pub async fn insert(
        &self,
        req: &CreateTemplateRequest,
        version: i32,
    ) -> Result<TemplateData, AppError> {
        let client = self.pool.get().await?;
        let name = req.name.trim().to_string();
        let title = req.title.trim().to_string();
        let content = req.content.clone();
        let format = req.format.trim().to_ascii_uppercase();
        let status = req.status.trim().to_ascii_uppercase();
        let now = Utc::now();

        let row = client
            .query_one(
                "INSERT INTO templates (name, status, title, content, format, version, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)
                 RETURNING id, name, status, title, content, format, version, created_at, updated_at",
                &[&name, &status, &title, &content, &format, &version, &now],
            )
            .await?;
        Ok(TemplateData::map_row(&row))
    }
}
