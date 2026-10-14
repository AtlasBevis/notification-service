use crate::utils::TimeUtils;
use deadpool_postgres::Pool;
use serde_json::Value;

use crate::enums::Status;
use crate::error::{AppError, AppResult};
use crate::models::{CreateSourceRequest, SourceData};

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
                "#,
                &[],
            )
            .await?;
        rows.iter().map(SourceData::map_row).collect()
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

    pub async fn insert(&self, req: &CreateSourceRequest) -> AppResult<SourceData> {
        let client = self.pool.get().await?;

        let row = client
            .query_one(
                r#"
                INSERT INTO sources (code, name, status, metadata, updated_at)
                VALUES ($1, $2, $3, $4, $5)
                RETURNING id, code, name, status, metadata, created_at, updated_at
                "#,
                &[
                    &req.code,
                    &req.name,
                    &Status::Active.as_str(),
                    &Value::Object(req.metadata.clone()),
                    &TimeUtils::now(),
                ],
            )
            .await?;
        SourceData::map_row(&row)
    }

    pub async fn patch_metadata(
        &self,
        code: &str,
        metadata: &serde_json::Map<String, Value>,
    ) -> AppResult<()> {
        let client = self.pool.get().await?;

        let n = client
            .execute(
                r#"
                UPDATE sources
                SET metadata = $2,
                    updated_at = $3
                WHERE code = $1
                "#,
                &[&code, &Value::Object(metadata.clone()), &TimeUtils::now()],
            )
            .await?;

        if n == 0 {
            return Err(AppError::not_found("not found"));
        }
        Ok(())
    }
}
