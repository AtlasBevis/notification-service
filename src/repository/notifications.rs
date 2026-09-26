use anyhow::Result;
use chrono::Utc;
use deadpool_postgres::Pool;
use serde_json::Value;
use tokio_postgres::error::SqlState;

use crate::{
    enums::NotificationStatus,
    models::{CreateNotificationRequest, NotificationData, UpdateNotificationRequest},
};

use crate::error::AppError;

const COLUMNS: &str =
    "id, code, name, status, source, target, channel, template_id, metadata, created_at, updated_at";

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
        let sql = format!("SELECT {COLUMNS} FROM notifications ORDER BY code");
        let rows = client.query(sql.as_str(), &[]).await?;
        Ok(rows.iter().map(NotificationData::map_row).collect())
    }

    pub async fn find_by_code(&self, code: &str) -> Result<NotificationData, AppError> {
        let client = self.pool.get().await?;
        let sql = format!("SELECT {COLUMNS} FROM notifications WHERE code = $1");
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

        let sql = format!(
            "INSERT INTO notifications (
                code, name, status, source, target, channel,
                template_id, metadata, updated_at
             ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
             RETURNING {COLUMNS}"
        );
        let row = client
            .query_one(
                sql.as_str(),
                &[
                    &req.code,
                    &req.name,
                    &NotificationStatus::Active.as_str(),
                    &req.source,
                    &req.target,
                    &req.channel,
                    &req.template_id,
                    &Value::Object(req.metadata.clone()),
                    &now,
                ],
            )
            .await?;
        Ok(NotificationData::map_row(&row))
    }

    pub async fn update(
        &self,
        code: &str,
        req: &UpdateNotificationRequest,
    ) -> Result<NotificationData, AppError> {
        let client = self.pool.get().await?;
        let now = Utc::now();

        let sql = format!(
            "UPDATE notifications
             SET name = $2,
                 status = $3,
                 source = $4,
                 target = $5,
                 channel = $6,
                 template_id = $7,
                 metadata = $8,
                 updated_at = $9
             WHERE code = $1
             RETURNING {COLUMNS}"
        );
        let row = client
            .query_opt(
                sql.as_str(),
                &[
                    &code,
                    &req.name,
                    &req.status,
                    &req.source,
                    &req.target,
                    &req.channel,
                    &req.template_id,
                    &Value::Object(req.metadata.clone()),
                    &now,
                ],
            )
            .await?;
        row.as_ref()
            .map(NotificationData::map_row)
            .ok_or_else(|| AppError::not_found("not found"))
    }

    pub async fn delete(&self, code: &str) -> Result<(), AppError> {
        let client = self.pool.get().await?;
        let n = match client
            .execute("DELETE FROM notifications WHERE code = $1", &[&code])
            .await
        {
            Ok(n) => n,
            Err(err) => {
                if let Some(db) = err.as_db_error() {
                    if db.code() == &SqlState::FOREIGN_KEY_VIOLATION {
                        return Err(AppError::conflict(
                            "notification is referenced by deliveries",
                        ));
                    }
                }
                return Err(err.into());
            }
        };
        if n == 0 {
            return Err(AppError::not_found("not found"));
        }
        Ok(())
    }
}
