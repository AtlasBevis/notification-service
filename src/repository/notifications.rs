use chrono::Utc;
use deadpool_postgres::Pool;
use serde_json::Value;
use uuid::Uuid;

use crate::models::{AcceptedNotification, DeliveryDraft};

use crate::error::AppError;

#[derive(Clone)]
pub struct NotificationsRepository {
    pool: Pool,
}

impl NotificationsRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Insert the runtime request and one delivery per route in one transaction.
    pub async fn accept(
        &self,
        definition_id: i64,
        source_code: &str,
        code: &str,
        trace_id: &str,
        variables: &Value,
        drafts: &[DeliveryDraft],
    ) -> Result<AcceptedNotification, AppError> {
        let mut client = self.pool.get().await?;
        let tx = client.transaction().await?;
        let now = Utc::now();
        let public_id = Uuid::new_v4().to_string();

        let row = tx
            .query_one(
                "INSERT INTO notifications (
                    public_id, code, trace_id, definition_id, source, variables
                 ) VALUES ($1,$2,$3,$4,$5,$6)
                 RETURNING id",
                &[
                    &public_id,
                    &code,
                    &trace_id,
                    &definition_id,
                    &source_code,
                    variables,
                ],
            )
            .await?;
        let notification_id: i64 = row.get("id");

        let draft = drafts
            .first()
            .ok_or_else(|| AppError::bad_request("no destination"))?;
        let delivery = tx
            .query_one(
                "INSERT INTO deliveries (
                    notification_id, target_code, channel_code,
                    title, body, payload, destination, updated_at
                 ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
                 RETURNING id",
                &[
                    &notification_id,
                    &draft.target_code,
                    &draft.channel_code,
                    &draft.title,
                    &draft.body,
                    &draft.payload,
                    &draft.destination,
                    &now,
                ],
            )
            .await?;
        let delivery_id: i64 = delivery.get("id");

        tx.commit().await?;
        Ok(AcceptedNotification {
            id: notification_id,
            delivery_id,
        })
    }
}
