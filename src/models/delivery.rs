use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeliveryData {
    pub id: i64,
    pub notification_id: i64,
    pub correlation_id: String,
    pub level: String,
    pub title: String,
    pub message: String,
    #[schema(value_type = Object)]
    pub recipients: Value,
    #[schema(value_type = Object)]
    pub payload: Value,
    #[schema(value_type = Object)]
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DeliveryData {
    pub fn map_row(row: &tokio_postgres::Row) -> Self {
        Self {
            id: row.get("id"),
            notification_id: row.get("notification_id"),
            correlation_id: row.get("correlation_id"),
            level: row.get("level"),
            title: row.get("title"),
            message: row.get("message"),
            recipients: row.get("recipients"),
            payload: row.get("payload"),
            metadata: row.get("metadata"),
            created_at: row.get::<_, DateTime<Utc>>("created_at"),
            updated_at: row.get::<_, DateTime<Utc>>("updated_at"),
        }
    }
}
