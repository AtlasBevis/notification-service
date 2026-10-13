use deadpool_postgres::Pool;

use crate::models::ChannelData;

use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub struct ChannelRepository {
    pool: Pool,
}

impl ChannelRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn list(&self) -> AppResult<Vec<ChannelData>> {
        let client = self.pool.get().await?;
        let rows = client
            .query(
                r#"
                SELECT id, code, name, status, metadata, created_at, updated_at
                FROM channels
                "#,
                &[],
            )
            .await?;
        rows.iter().map(ChannelData::map_row).collect()
    }

    pub async fn find_by_code(&self, code: &str) -> AppResult<ChannelData> {
        let client = self.pool.get().await?;
        let key = code.trim().to_ascii_uppercase();
        let row = client
            .query_opt(
                r#"
                SELECT id, code, name, status, metadata, created_at, updated_at
                FROM channels
                WHERE code = $1
                "#,
                &[&key],
            )
            .await?;
        let row = row.ok_or_else(|| AppError::not_found(format!("unknown channel `{key}`")))?;
        ChannelData::map_row(&row)
    }

    pub async fn find_by_id(&self, id: i32) -> AppResult<ChannelData> {
        let client = self.pool.get().await?;
        let row = client
            .query_opt(
                r#"
                SELECT id, code, name, status, metadata, created_at, updated_at
                FROM channels
                WHERE id = $1
                "#,
                &[&id],
            )
            .await?;
        let row = row.ok_or_else(|| AppError::not_found(format!("unknown channel id `{id}`")))?;
        ChannelData::map_row(&row)
    }
}
