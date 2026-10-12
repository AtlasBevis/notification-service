use anyhow::{Context, Result};
use deadpool_postgres::Pool;

use crate::models::ChannelData;

use crate::error::AppError;

#[derive(Clone)]
pub struct ChannelRepository {
    pool: Pool,
}

impl ChannelRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn list(&self) -> Result<Vec<ChannelData>> {
        let client = self.pool.get().await.context("postgres pool")?;
        let rows = client
            .query(
                "SELECT id, code, name, status, metadata, created_at, updated_at
                 FROM channels",
                &[],
            )
            .await
            .context("list channels")?;
        Ok(rows.iter().map(ChannelData::map_row).collect())
    }

    pub async fn find_by_code(&self, code: &str) -> Result<Option<ChannelData>, AppError> {
        let client = self.pool.get().await?;
        let key = code.trim().to_ascii_uppercase();
        let row = client
            .query_opt(
                "SELECT id, code, name, status, metadata, created_at, updated_at
                 FROM channels WHERE code = $1",
                &[&key],
            )
            .await?;
        Ok(row.map(|r| ChannelData::map_row(&r)))
    }

    pub async fn find_by_id(&self, id: i32) -> Result<Option<ChannelData>, AppError> {
        let client = self.pool.get().await?;
        let row = client
            .query_opt(
                "SELECT id, code, name, status, metadata, created_at, updated_at
                 FROM channels WHERE id = $1",
                &[&id],
            )
            .await?;
        Ok(row.map(|r| ChannelData::map_row(&r)))
    }
}
