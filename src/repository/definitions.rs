use deadpool_postgres::Pool;

use crate::{error::AppResult, models::DefinitionData};

use crate::error::AppError;

const COLUMNS: &str = "id, code, name, status, description, level, source_id, channel_id, target_id, template_id, created_at, updated_at";

#[derive(Clone)]
pub struct DefinitionRepository {
    pool: Pool,
}

impl DefinitionRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn find_by_code(&self, code: &str) -> AppResult<DefinitionData> {
        let client = self.pool.get().await?;
        let sql = format!("SELECT {COLUMNS} FROM definitions WHERE code = $1");
        match client.query_opt(sql.as_str(), &[&code]).await? {
            Some(row) => Ok(DefinitionData::map_row(&row)),
            None => Err(AppError::not_found(format!("code {code} not found"))),
        }
    }

    pub async fn find_by_id(&self, id: i64) -> AppResult<DefinitionData> {
        let client = self.pool.get().await?;
        let sql = format!("SELECT {COLUMNS} FROM definitions WHERE id = $1");
        match client.query_opt(sql.as_str(), &[&id]).await? {
            Some(row) => Ok(DefinitionData::map_row(&row)),
            None => Err(AppError::not_found(format!("id {id} not found"))),
        }
    }
}
