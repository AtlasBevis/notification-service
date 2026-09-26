use anyhow::{Context, Result};
use chrono::Utc;
use deadpool_postgres::Pool;
use serde_json::Value;

use crate::{
    enums::NotificationStatus,
    models::{CreateTargetRequest, TargetData},
};

use crate::error::AppError;

#[derive(Clone)]
pub struct TargetsRepository {
    pool: Pool,
}

impl TargetsRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn list(&self) -> Result<Vec<TargetData>> {
        let client = self.pool.get().await.context("postgres pool")?;
        let rows = client
            .query(
                "SELECT code, name, status, metadata, created_at, updated_at
                  FROM targets",
                &[],
            )
            .await
            .context("list targets")?;
        Ok(rows.iter().map(TargetData::map_row).collect())
    }

    pub async fn find_by_code(&self, code: &str) -> Result<Option<TargetData>, AppError> {
        let client = self.pool.get().await?;
        let row = client
            .query_opt(
                "SELECT code, name, status, metadata, created_at, updated_at
                    FROM targets
                    WHERE code = $1",
                &[&code],
            )
            .await?;
        Ok(row.map(|r| TargetData::map_row(&r)))
    }

    pub async fn insert(&self, req: &CreateTargetRequest) -> Result<TargetData, AppError> {
        let client = self.pool.get().await?;

        let row = client
            .query_one(
                "INSERT INTO targets (code, name, status, metadata, updated_at)
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
        Ok(TargetData::map_row(&row))
    }

    pub async fn patch_metadata(
        &self,
        code: &str,
        metadata: &serde_json::Map<String, Value>,
    ) -> Result<(), AppError> {
        let client = self.pool.get().await?;

        let n = client
            .execute(
                "UPDATE targets
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
