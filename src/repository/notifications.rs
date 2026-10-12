use chrono::Utc;
use deadpool_postgres::Pool;
use serde_json::Value;
use tokio_postgres::error::SqlState;
use uuid::Uuid;

use crate::{
    enums::Status,
    models::{
        AcceptedNotification, CreateNotificationRequest, DeliveryDraft, NotificationData,
        UpdateNotificationRequest,
    },
};

use crate::error::AppError;

const DEFINITION_SELECT: &str = "
    SELECT d.id, d.code, d.name, d.status, d.description,
           s.code AS source_code, c.code AS channel_code, t.code AS target_code,
           d.template_id::BIGINT AS template_id, d.level, d.created_at, d.updated_at
    FROM definitions d
    JOIN sources s ON s.id = d.source_id
    JOIN channels c ON c.id = d.channel_id
    JOIN targets t ON t.id = d.target_id";

#[derive(Clone)]
pub struct NotificationsRepository {
    pool: Pool,
}

impl NotificationsRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn list(&self) -> Result<Vec<NotificationData>, AppError> {
        let client = self.pool.get().await?;
        let sql = format!("{DEFINITION_SELECT} ORDER BY d.code");
        let rows = client.query(sql.as_str(), &[]).await?;
        Ok(rows.iter().map(NotificationData::map_row).collect())
    }

    pub async fn find_by_code(&self, code: &str) -> Result<NotificationData, AppError> {
        let client = self.pool.get().await?;
        let sql = format!("{DEFINITION_SELECT} WHERE d.code = $1");
        match client.query_opt(sql.as_str(), &[&code]).await? {
            Some(row) => Ok(NotificationData::map_row(&row)),
            None => Err(AppError::not_found(format!("code {code} not found"))),
        }
    }

    pub async fn insert(
        &self,
        req: &CreateNotificationRequest,
    ) -> Result<NotificationData, AppError> {
        let client = self.pool.get().await?;
        let now = Utc::now();
        let template_id = i32::try_from(req.template_id)
            .map_err(|_| AppError::bad_request("template_id is out of range"))?;
        client
            .execute(
                "INSERT INTO definitions (
                    code, name, status, description, source_id,
                    channel_id, target_id, template_id, level, updated_at
                 ) VALUES (
                    $1,$2,$3,$4,
                    (SELECT id FROM sources WHERE code = $5),
                    (SELECT id FROM channels WHERE code = $6),
                    (SELECT id FROM targets WHERE code = $7),
                    $8,$9,$10
                 )",
                &[
                    &req.code,
                    &req.name,
                    &Status::Active.as_str(),
                    &req.description,
                    &req.source_code,
                    &req.channel_code,
                    &req.target_code,
                    &template_id,
                    &req.level,
                    &now,
                ],
            )
            .await?;
        self.find_by_code(&req.code).await
    }

    pub async fn update(
        &self,
        code: &str,
        req: &UpdateNotificationRequest,
    ) -> Result<NotificationData, AppError> {
        let client = self.pool.get().await?;
        let now = Utc::now();
        let template_id = i32::try_from(req.template_id)
            .map_err(|_| AppError::bad_request("template_id is out of range"))?;
        let changed = client
            .execute(
                "UPDATE definitions
                 SET name = $2, status = $3, description = $4,
                     source_id = (SELECT id FROM sources WHERE code = $5),
                     channel_id = (SELECT id FROM channels WHERE code = $6),
                     target_id = (SELECT id FROM targets WHERE code = $7),
                     template_id = $8, level = $9, updated_at = $10
                 WHERE code = $1",
                &[
                    &code,
                    &req.name,
                    &req.status,
                    &req.description,
                    &req.source_code,
                    &req.channel_code,
                    &req.target_code,
                    &template_id,
                    &req.level,
                    &now,
                ],
            )
            .await?;
        if changed == 0 {
            return Err(AppError::not_found(format!("notification `{code}` not found")));
        }
        self.find_by_code(code).await
    }

    pub async fn delete(&self, code: &str) -> Result<(), AppError> {
        let client = self.pool.get().await?;
        let deleted = match client
            .execute("DELETE FROM definitions WHERE code = $1", &[&code])
            .await
        {
            Ok(count) => count,
            Err(err) => {
                if err.as_db_error().is_some_and(|db| db.code() == &SqlState::FOREIGN_KEY_VIOLATION) {
                    return Err(AppError::conflict(
                        "definition is referenced by notifications",
                    ));
                }
                return Err(err.into());
            }
        };
        if deleted == 0 {
            return Err(AppError::not_found(format!("notification `{code}` not found")));
        }
        Ok(())
    }

    /// Insert the runtime request and one delivery per route in one transaction.
    pub async fn accept(
        &self,
        definition_id: i64,
        source_code: &str,
        code: &str,
        idempotency_key: &str,
        variables: &Value,
        drafts: &[DeliveryDraft],
    ) -> Result<AcceptedNotification, AppError> {
        let mut client = self.pool.get().await?;
        let tx = client.transaction().await?;
        let now = Utc::now();

        let row = tx
            .query_one(
                "INSERT INTO notifications (
                    definition_id, source_code, code, idempotency_key, variables
                 ) VALUES ($1,$2,$3,$4,$5)
                 RETURNING id, public_id",
                &[
                    &definition_id,
                    &source_code,
                    &code,
                    &idempotency_key,
                    variables,
                ],
            )
            .await?;
        let notification_id: i64 = row.get("id");
        let public_id: Uuid = row.get("public_id");

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
            public_id,
            delivery_id,
        })
    }
}
