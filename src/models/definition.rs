use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::enums::{Level, Status};
use crate::error::{AppError, AppResult};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DefinitionData {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub status: Status,
    pub level: Level,
    pub source_id: i32,
    pub channel_id: i32,
    pub targets: Vec<DefinitionTarget>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl DefinitionData {
    pub fn map_row(row: &tokio_postgres::Row) -> AppResult<Self> {
        let status: String = row.get("status");
        let level: String = row.get("level");
        Ok(Self {
            id: row.get("id"),
            code: row.get("code"),
            name: row.get("name"),
            description: row.get("description"),
            status: Status::parse(&status).ok_or_else(|| {
                AppError::internal(format!("unknown status `{status}` in definitions"))
            })?,
            level: Level::parse(&level).ok_or_else(|| {
                AppError::internal(format!("unknown level `{level}` in definitions"))
            })?,
            source_id: row.get("source_id"),
            channel_id: row.get("channel_id"),
            targets: serde_json::from_value(row.get("targets")).map_err(|error| {
                AppError::internal(format!("invalid definition target mapping: {error}"))
            })?,
            created_at: row.get::<_, DateTime<Utc>>("created_at"),
            updated_at: row.get::<_, Option<DateTime<Utc>>>("updated_at"),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DefinitionTarget {
    pub target_id: i32,
    pub template_id: Option<i32>,
}
