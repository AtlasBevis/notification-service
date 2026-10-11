use chrono::Utc;
use deadpool_postgres::Pool;
use serde_json::Value;
use tokio_postgres::error::SqlState;
use uuid::Uuid;

use crate::{
    enums::StatusEnum,
    error::AppResult,
    models::{
        AcceptedNotification, CreateNotificationRequest, DefinitionData, DeliveryDraft,
        NotificationData, UpdateNotificationRequest,
    },
};

use crate::error::AppError;

const COLUMNS: &str = "id, code, name, status, description, source_code, channel_code, target_code, template_id, level, created_at, updated_at";

#[derive(Clone)]
pub struct DefinitionRepository {
    pool: Pool,
}

impl DefinitionRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    // pub async fn list(&self) -> Result<Vec<NotificationData>, AppError> {
    //     let client = self.pool.get().await?;
    //     let sql = format!("SELECT {COLUMNS} FROM definitions ORDER BY code");
    //     let rows = client.query(sql.as_str(), &[]).await?;
    //     Ok(rows.iter().map(NotificationData::map_row).collect())
    // }

    pub async fn find_by_code(&self, code: &str) -> AppResult<DefinitionData> {
        let client = self.pool.get().await?;
        let sql = format!("SELECT {COLUMNS} FROM definitions WHERE code = $1");
        match client.query_opt(sql.as_str(), &[&code]).await? {
            Some(row) => Ok(DefinitionData::map_row(&row)),
            None => Err(AppError::not_found(format!("code {code} not found"))),
        }
    }

    pub async fn find_by_id(&self, id: i32) -> AppResult<DefinitionData> {
        let client = self.pool.get().await?;
        let sql = format!("SELECT {COLUMNS} FROM definitions WHERE id = $1");
        match client.query_opt(sql.as_str(), &[&id]).await? {
            Some(row) => Ok(DefinitionData::map_row(&row)),
            None => Err(AppError::not_found(format!("id {id} not found"))),
        }
    }

    // pub async fn insert(
    //     &self,
    //     req: &CreateNotificationRequest,
    // ) -> Result<NotificationData, AppError> {
    //     let client = self.pool.get().await?;
    //     let now = Utc::now();

    //     let sql = format!(
    //         "INSERT INTO definitions (
    //             code, name, status, description, source_code,
    //             channel_code, target_code, template_id, level, updated_at
    //          ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
    //          RETURNING {COLUMNS}"
    //     );
    //     let row = client
    //         .query_one(
    //             sql.as_str(),
    //             &[
    //                 &req.code,
    //                 &req.name,
    //                 &StatusEnum::Active.as_str(),
    //                 &req.description,
    //                 &req.source_code,
    //                 &req.channel_code,
    //                 &req.target_code,
    //                 &req.template_id,
    //                 &req.level,
    //                 &now,
    //             ],
    //         )
    //         .await?;
    //     Ok(NotificationData::map_row(&row))
    // }

    // pub async fn update(
    //     &self,
    //     code: &str,
    //     req: &UpdateNotificationRequest,
    // ) -> Result<NotificationData, AppError> {
    //     let client = self.pool.get().await?;
    //     let now = Utc::now();

    //     let sql = format!(
    //         "UPDATE definitions
    //          SET name = $2,
    //              status = $3,
    //              description = $4,
    //              source_code = $5,
    //              channel_code = $6,
    //              target_code = $7,
    //              template_id = $8,
    //              level = $9,
    //              updated_at = $10
    //          WHERE code = $1
    //          RETURNING {COLUMNS}"
    //     );
    //     let row = client
    //         .query_opt(
    //             sql.as_str(),
    //             &[
    //                 &code,
    //                 &req.name,
    //                 &req.status,
    //                 &req.description,
    //                 &req.source_code,
    //                 &req.channel_code,
    //                 &req.target_code,
    //                 &req.template_id,
    //                 &req.level,
    //                 &now,
    //             ],
    //         )
    //         .await?;
    //     row.as_ref()
    //         .map(NotificationData::map_row)
    //         .ok_or_else(|| AppError::not_found("not found"))
    // }

    // pub async fn delete(&self, code: &str) -> Result<(), AppError> {
    //     let client = self.pool.get().await?;
    //     let n = match client
    //         .execute("DELETE FROM definitions WHERE code = $1", &[&code])
    //         .await
    //     {
    //         Ok(n) => n,
    //         Err(err) => {
    //             if let Some(db) = err.as_db_error() {
    //                 if db.code() == &SqlState::FOREIGN_KEY_VIOLATION {
    //                     return Err(AppError::conflict(
    //                         "definition is referenced by notifications",
    //                     ));
    //                 }
    //             }
    //             return Err(err.into());
    //         }
    //     };
    //     if n == 0 {
    //         return Err(AppError::not_found("not found"));
    //     }
    //     Ok(())
    // }
}
