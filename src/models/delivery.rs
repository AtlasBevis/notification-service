use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryData {
    pub id: i64,
    pub notification_id: i64,
    pub target_code: String,
    pub channel_code: String,
    pub status: String,
    pub title: String,
    pub body: String,
    pub payload: Value,
    pub destination: Value,
    pub attempt_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DeliveryData {
    pub fn map_row(row: &tokio_postgres::Row) -> Self {
        Self {
            id: row.get("id"),
            notification_id: row.get("notification_id"),
            target_code: row.get("target_code"),
            channel_code: row.get("channel_code"),
            status: row.get("status"),
            title: row.get("title"),
            body: row.get("body"),
            payload: row.get("payload"),
            destination: row.get("destination"),
            attempt_count: row.get("attempt_count"),
            created_at: row.get::<_, DateTime<Utc>>("created_at"),
            updated_at: row.get::<_, DateTime<Utc>>("updated_at"),
        }
    }
}
