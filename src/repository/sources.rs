use anyhow::{Context, Result};
use chrono::Utc;
use deadpool_postgres::Pool;
use serde_json::Value;

use crate::{
    enums::NotificationStatus,
    models::{CreateSourceRequest, SourceData},
};

use crate::error::AppError;

#[derive(Clone)]
pub struct SourcesRepository {
    pool: Pool,
}

impl SourcesRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn list(&self) -> Result<Vec<SourceData>> {
        let client = self.pool.get().await.context("postgres pool")?;
        let rows = client
            .query(
                "SELECT code, name, status, metadata, created_at, updated_at
                  FROM sources",
                &[],
            )
            .await
            .context("list sources")?;
        Ok(rows.iter().map(SourceData::map_row).collect())
    }

    pub async fn find_by_code(&self, code: &str) -> Result<Option<SourceData>, AppError> {
        let client = self.pool.get().await?;
        let row = client
            .query_opt(
                "SELECT code, name, status, metadata, created_at, updated_at
                    FROM sources
                    WHERE code = $1",
                &[&code],
            )
            .await?;
        Ok(row.map(|r| SourceData::map_row(&r)))
    }

    pub async fn must_active(&self, code: &str) -> Result<SourceData, String> {
        match self.find_by_code(code).await {
            Ok(Some(s)) if s.status.eq_ignore_ascii_case("ACTIVE") => Ok(s),
            Ok(Some(_)) => Err(format!("source `{code}` is not ACTIVE")),
            Ok(None) => Err(format!("unknown source `{code}`")),
            Err(err) => Err(format!("failed to load source `{code}`: {err}")),
        }
    }

    pub async fn insert(&self, req: &CreateSourceRequest) -> Result<SourceData, AppError> {
        let client = self.pool.get().await?;

        let row = client
            .query_one(
                "INSERT INTO sources (code, name, status, metadata, updated_at)
                 VALUES ($1, $2, $3, $4, $5)
                 RETURNING code, name, status, metadata, created_at, updated_at",
                &[
                    &req.code,
                    &req.name,
                    &NotificationStatus::Active.as_str(),
                    &Value::Object(req.metadata.clone()),
                    &Utc::now(),
                ],
            )
            .await?;
        Ok(SourceData::map_row(&row))
    }

    pub async fn patch_metadata(
        &self,
        code: &str,
        metadata: &serde_json::Map<String, Value>,
    ) -> Result<(), AppError> {
        let client = self.pool.get().await?;

        let n = client
            .execute(
                "UPDATE sources
                 SET metadata = $2,
                     updated_at = $3
                 WHERE code = $1",
                &[&code, &Value::Object(metadata.clone()), &Utc::now()],
            )
            .await?;

        if n == 0 {
            return Err(AppError::not_found("not found"));
        }
        Ok(())
    }
}
