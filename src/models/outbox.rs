use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use crate::enums::NotificationStatus;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OutboxData {
    pub id: i64,
    pub notification_id: Option<i64>,
    pub delivery_id: Option<i64>,
    pub correlation_id: String,
    pub topic: String,
    pub partition_key: String,
    pub partition: Option<i32>,
    pub offset: Option<i64>,
    pub status: NotificationStatus,
    pub payload: Value,
    pub error_message: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl OutboxData {
    pub fn new(
        notification_id: Option<i64>,
        delivery_id: Option<i64>,
        correlation_id: impl Into<String>,
        topic: impl Into<String>,
        partition_key: impl Into<String>,
        payload: Value,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: 0,
            notification_id,
            delivery_id,
            correlation_id: correlation_id.into(),
            topic: topic.into(),
            partition_key: partition_key.into(),
            partition: None,
            offset: None,
            status: NotificationStatus::Queued,
            payload,
            error_message: None,
            published_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn map_row(row: &tokio_postgres::Row) -> Self {
        let status_raw: String = row.get("status");
        Self {
            id: row.get("id"),
            notification_id: row.get("notification_id"),
            delivery_id: row.get("delivery_id"),
            correlation_id: row.get("correlation_id"),
            topic: row.get("topic"),
            partition_key: row.get("partition_key"),
            partition: row.get("partition"),
            offset: row.get("offset"),
            status: NotificationStatus::parse(&status_raw).unwrap_or(NotificationStatus::Failed),
            payload: row.get("payload"),
            error_message: row.get("error_message"),
            published_at: row.get("published_at"),
            created_at: row.get::<_, DateTime<Utc>>("created_at"),
            updated_at: row.get::<_, DateTime<Utc>>("updated_at"),
        }
    }
}
