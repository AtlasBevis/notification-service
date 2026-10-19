use crate::models::{CreateSourceRequest, PageData, PageRequest, PutSourceRequest};
use crate::utils::TimeUtils;
use deadpool_postgres::Pool;
use serde_json::Value;

use crate::error::{AppError, AppResult};
use crate::models::SourceData;

#[derive(Clone)]
pub struct SourceRepository {
    pool: Pool,
}

impl SourceRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn list(&self) -> AppResult<Vec<SourceData>> {
        let client = self.pool.get().await?;
        let rows = client
            .query(
                r#"
                SELECT id, code, name, status, metadata, created_at, updated_at
                FROM sources
                ORDER BY code
                "#,
                &[],
            )
            .await?;
        rows.iter().map(SourceData::map_row).collect()
    }

    pub async fn list_page(&self, page: &PageRequest) -> AppResult<PageData<SourceData>> {
        let client = self.pool.get().await?;
        let total: i64 = client
            .query_one(
                r#"
                SELECT COUNT(*)
                FROM sources
                "#,
                &[],
            )
            .await?
            .get(0);
        let rows = client
            .query(
                r#"
                SELECT id, code, name, status, metadata, created_at, updated_at
                FROM sources
                ORDER BY updated_at, created_at
                LIMIT $1
                OFFSET $2
                "#,
                &[&page.limit(), &page.offset()],
            )
            .await?;
        let items = rows
            .iter()
            .map(SourceData::map_row)
            .collect::<AppResult<Vec<_>>>()?;
        Ok(PageData::new(items, total))
    }

    pub async fn find_by_code(&self, code: &str) -> AppResult<SourceData> {
        let client = self.pool.get().await?;
        let row = client
            .query_opt(
                r#"
                SELECT id, code, name, status, metadata, created_at, updated_at
                FROM sources
                WHERE code = $1
                "#,
                &[&code],
            )
            .await?;
        match row {
            Some(r) => SourceData::map_row(&r),
            None => Err(AppError::NotFound(format!("source `{code}` not found"))),
        }
    }

    pub async fn find_by_id(&self, id: i32) -> AppResult<SourceData> {
        let client = self.pool.get().await?;
        let row = client
            .query_opt(
                r#"
                SELECT id, code, name, status, metadata, created_at, updated_at
                FROM sources
                WHERE id = $1
                "#,
                &[&id],
            )
            .await?;
        match row {
            Some(r) => SourceData::map_row(&r),
            None => Err(AppError::NotFound(format!("source id `{id}` not found"))),
        }
    }

    pub async fn insert(&self, req: &CreateSourceRequest) -> AppResult<i32> {
        let client = self.pool.get().await?;
        let row = client
            .query_one(
                r#"
                INSERT INTO sources (code, name, metadata)
                VALUES ($1, $2, $3)
                RETURNING id
                "#,
                &[&req.code, &req.name, &Value::Object(req.metadata.clone())],
            )
            .await?;
        Ok(row.get("id"))
    }

    /// An empty `name` keeps the stored name; `metadata` replaces the stored value.
    pub async fn update(&self, code: &str, req: &PutSourceRequest) -> AppResult<()> {
        let client = self.pool.get().await?;
        let n = client
            .execute(
                r#"
                UPDATE sources
                SET name = $2,
                    metadata = $3,
                    updated_at = $4
                WHERE code = $1
                "#,
                &[
                    &code,
                    &req.name,
                    &Value::Object(req.metadata.clone()),
                    &TimeUtils::now(),
                ],
            )
            .await?;

        if n == 0 {
            return Err(AppError::not_found(format!("source `{}` not found", code)));
        }
        Ok(())
    }
}
