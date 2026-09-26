use chrono::Utc;
use deadpool_postgres::Pool;
use serde_json::Value;

use crate::models::DeliveryData;

use crate::error::AppError;

#[derive(Clone)]
pub struct DeliveriesRepository {
    pool: Pool,
}

impl DeliveriesRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn insert(
        &self,
        notification_id: i64,
        correlation_id: &str,
        level: &str,
        title: &str,
        message: &str,
        recipients: &Value,
        payload: &Value,
        metadata: &Value,
    ) -> Result<DeliveryData, AppError> {
        let client = self.pool.get().await?;
        let now = Utc::now();

        let row = client
            .query_one(
                "INSERT INTO deliveries (
                    notification_id, correlation_id, level, title, message,
                    recipients, payload, metadata, updated_at
                 ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
                 RETURNING id, notification_id, correlation_id, level, title, message,
                           recipients, payload, metadata, created_at, updated_at",
                &[
                    &notification_id,
                    &correlation_id,
                    &level,
                    &title,
                    &message,
                    recipients,
                    payload,
                    metadata,
                    &now,
                ],
            )
            .await?;
        Ok(DeliveryData::map_row(&row))
    }
}
