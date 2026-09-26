use anyhow::{Context, Result};
use chrono::Utc;
use deadpool_postgres::Pool;
use serde_json::Value;

use crate::enums::NotificationStatus;
use crate::models::{DeliveryData, OutboxData};

use crate::error::AppError;

const OUTBOX_SELECT: &str = "id, notification_id, delivery_id, correlation_id, topic, partition_key,
    \"partition\", \"offset\", status, payload, error_message, published_at, created_at, updated_at";

#[derive(Clone)]
pub struct OutboxRepository {
    pool: Pool,
}

impl OutboxRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Insert delivery audit + outbox entry atomically.
    pub async fn insert_delivery_with_outbox<F>(
        &self,
        notification_id: i64,
        correlation_id: &str,
        level: &str,
        title: &str,
        message: &str,
        recipients: &Value,
        payload: &Value,
        metadata: &Value,
        topic: &str,
        partition_key: &str,
        build_kafka_payload: F,
    ) -> Result<(DeliveryData, OutboxData), AppError>
    where
        F: FnOnce(&DeliveryData) -> Value,
    {
        let mut client = self.pool.get().await?;
        let tx = client
            .transaction()
            .await?;
        let now = Utc::now();

        let delivery_row = tx
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

        let delivery = DeliveryData::map_row(&delivery_row);
        let kafka_payload = build_kafka_payload(&delivery);
        let status = NotificationStatus::Queued.as_str();

        let outbox_row = tx
            .query_one(
                &format!(
                    "INSERT INTO outbox (
                        notification_id, delivery_id, correlation_id, topic, partition_key,
                        status, payload, updated_at
                     ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
                     RETURNING {OUTBOX_SELECT}"
                ),
                &[
                    &Some(notification_id),
                    &Some(delivery.id),
                    &correlation_id,
                    &topic,
                    &partition_key,
                    &status,
                    &kafka_payload,
                    &now,
                ],
            )
            .await?;

        tx.commit()
            .await?;

        Ok((delivery, OutboxData::map_row(&outbox_row)))
    }

    /// Claim up to `limit` QUEUED rows → PROCESSING (skip locked).
    pub async fn claim_queued(&self, limit: i64) -> Result<Vec<OutboxData>> {
        let mut client = self.pool.get().await.context("postgres pool")?;
        let tx = client.transaction().await.context("outbox claim tx")?;
        let now = Utc::now();
        let queued = NotificationStatus::Queued.as_str();
        let processing = NotificationStatus::Processing.as_str();

        let rows = tx
            .query(
                &format!(
                    "UPDATE outbox
                     SET status = $3, updated_at = $2
                     WHERE id IN (
                         SELECT id FROM outbox
                         WHERE status = $4
                         ORDER BY created_at
                         FOR UPDATE SKIP LOCKED
                         LIMIT $1
                     )
                     RETURNING {OUTBOX_SELECT}"
                ),
                &[&limit, &now, &processing, &queued],
            )
            .await
            .context("claim queued outbox rows")?;

        tx.commit().await.context("outbox claim commit")?;
        Ok(rows.iter().map(OutboxData::map_row).collect())
    }

    pub async fn mark_sent(
        &self,
        id: i64,
        partition: i32,
        offset: i64,
    ) -> Result<(), AppError> {
        let client = self.pool.get().await?;
        let now = Utc::now();
        let status = NotificationStatus::Sent.as_str();
        let n = client
            .execute(
                "UPDATE outbox
                 SET status = $5,
                     \"partition\" = $2,
                     \"offset\" = $3,
                     published_at = $4,
                     updated_at = $4,
                     error_message = NULL
                 WHERE id = $1",
                &[&id, &partition, &offset, &now, &status],
            )
            .await?;
        if n == 0 {
            return Err(AppError::not_found("not found"));
        }
        Ok(())
    }

    pub async fn mark_failed(&self, id: i64, error: &str) -> Result<(), AppError> {
        let client = self.pool.get().await?;
        let now = Utc::now();
        let status = NotificationStatus::Failed.as_str();
        let n = client
            .execute(
                "UPDATE outbox
                 SET status = $4,
                     error_message = $2,
                     updated_at = $3
                 WHERE id = $1",
                &[&id, &error, &now, &status],
            )
            .await?;
        if n == 0 {
            return Err(AppError::not_found("not found"));
        }
        Ok(())
    }
}
