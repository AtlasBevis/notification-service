use anyhow::{Context, Result};
use deadpool_postgres::Pool;

use crate::models::ChannelData;

use crate::error::AppError;

#[derive(Clone)]
pub struct ChannelsRepository {
    pool: Pool,
}

impl ChannelsRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn list(&self) -> Result<Vec<ChannelData>> {
        let client = self.pool.get().await.context("postgres pool")?;
        let rows = client
            .query(
                "SELECT code, name, status, metadata, created_at, updated_at
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
                "SELECT code, name, status, metadata, created_at, updated_at
                 FROM channels WHERE code = $1",
                &[&key],
            )
            .await?;
        Ok(row.map(|r| ChannelData::map_row(&r)))
    }

    pub async fn must_active(&self, code: &str) -> Result<ChannelData, String> {
        match self.find_by_code(code).await {
            Ok(Some(c)) if c.status.eq_ignore_ascii_case("ACTIVE") => Ok(c),
            Ok(Some(_)) => Err(format!("channel `{code}` is not ACTIVE")),
            Ok(None) => Err(format!("unknown channel `{code}`")),
            Err(err) => Err(format!("failed to load channel `{code}`: {err}")),
        }
    }
}
