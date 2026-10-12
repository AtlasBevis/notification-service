use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DefinitionData {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub level: String,
    pub source_id: i32,
    pub channel_id: i32,
    pub target_id: i32,
    pub template_id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl DefinitionData {
    pub fn map_row(row: &tokio_postgres::Row) -> Self {
        Self {
            id: row.get("id"),
            code: row.get("code"),
            name: row.get("name"),
            description: row.get("description"),
            status: row.get("status"),
            level: row.get("level"),
            source_id: row.get("source_id"),
            channel_id: row.get("channel_id"),
            target_id: row.get("target_id"),
            template_id: row.get("template_id"),
            created_at: row.get::<_, DateTime<Utc>>("created_at"),
            updated_at: row.get::<_, Option<DateTime<Utc>>>("updated_at"),
        }
    }
}
