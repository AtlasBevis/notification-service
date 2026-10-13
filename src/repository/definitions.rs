use deadpool_postgres::Pool;

use crate::{error::AppResult, models::DefinitionData};

use crate::error::AppError;

const COLUMNS: &str = "d.id, d.code, d.name, d.status, d.description, d.level, d.source_id, d.channel_id, d.created_at, d.updated_at, (SELECT COALESCE(jsonb_agg(jsonb_build_object('target_id', dt.target_id, 'template_id', dt.template_id) ORDER BY dt.id), '[]'::jsonb) FROM definition_targets dt WHERE dt.definition_id = d.id) AS targets";

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
        let sql = format!("SELECT {COLUMNS} FROM definitions d WHERE d.code = $1");
        match client.query_opt(sql.as_str(), &[&code]).await? {
            Some(row) => DefinitionData::map_row(&row),
            None => Err(AppError::not_found(format!("code {code} not found"))),
        }
    }

    pub async fn find_by_id(&self, id: i64) -> AppResult<DefinitionData> {
        let client = self.pool.get().await?;
        let sql = format!("SELECT {COLUMNS} FROM definitions d WHERE d.id = $1");
        match client.query_opt(sql.as_str(), &[&id]).await? {
            Some(row) => DefinitionData::map_row(&row),
            None => Err(AppError::not_found(format!("id {id} not found"))),
        }
    }
}
