use anyhow::{Context, Result};
use chrono::Utc;
use deadpool_postgres::Pool;
use serde_json::Value;

use crate::{
    enums::Status,
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
                "SELECT t.id, t.code, c.code AS channel_code, t.name, t.status,
                        t.metadata, t.created_at, t.updated_at
                 FROM targets t
                 JOIN channels c ON c.id = t.channel_id",
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
                "SELECT t.id, t.code, c.code AS channel_code, t.name, t.status,
                        t.metadata, t.created_at, t.updated_at
                 FROM targets t
                 JOIN channels c ON c.id = t.channel_id
                 WHERE t.code = $1",
                &[&code],
            )
            .await?;
        Ok(row.map(|r| TargetData::map_row(&r)))
    }

    pub async fn find_by_id(&self, id: i32) -> Result<Option<TargetData>, AppError> {
        let client = self.pool.get().await?;
        let row = client
            .query_opt(
                "SELECT t.id, t.code, c.code AS channel_code, t.name, t.status,
                        t.metadata, t.created_at, t.updated_at
                 FROM targets t
                 JOIN channels c ON c.id = t.channel_id
                 WHERE t.id = $1",
                &[&id],
            )
            .await?;
        Ok(row.map(|r| TargetData::map_row(&r)))
    }

    pub async fn insert(&self, req: &CreateTargetRequest) -> Result<TargetData, AppError> {
        let client = self.pool.get().await?;

        let row = client
            .query_one(
                "WITH inserted AS (
                    INSERT INTO targets (code, channel_id, name, status, metadata, updated_at)
                    VALUES ($1, (SELECT id FROM channels WHERE code = $2), $3, $4, $5, $6)
                    RETURNING id, code, channel_id, name, status, metadata, created_at, updated_at
                 )
                 SELECT i.id, i.code, c.code AS channel_code, i.name, i.status,
                        i.metadata, i.created_at, i.updated_at
                 FROM inserted i
                 JOIN channels c ON c.id = i.channel_id",
                &[
                    &req.code,
                    &req.channel_code,
                    &req.name,
                    &Status::Active.as_str(),
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
