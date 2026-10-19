use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::enums::{Channel, Status};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetData {
    pub id: i32,
    pub code: String,
    pub channel: Channel,
    pub name: String,
    pub status: Status,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl TargetData {
    pub fn map_row(row: &tokio_postgres::Row) -> AppResult<Self> {
        let channel: String = row.get("channel");
        let status: String = row.get("status");
        Ok(Self {
            id: row.get("id"),
            code: row.get("code"),
            channel: Channel::parse(&channel).ok_or_else(|| {
                AppError::internal(format!("unknown channel `{channel}` in targets"))
            })?,
            name: row.get("name"),
            status: Status::parse(&status).ok_or_else(|| {
                AppError::internal(format!("unknown status `{status}` in targets"))
            })?,
            created_at: row.get::<_, DateTime<Utc>>("created_at"),
            updated_at: row.get::<_, Option<DateTime<Utc>>>("updated_at"),
        })
    }
}

/// EMAIL recipients snapshot sent with a delivery.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Recipients {
    pub to: Vec<String>,
    #[serde(default)]
    pub cc: Vec<String>,
    #[serde(default)]
    pub bcc: Vec<String>,
}

impl Recipients {
    pub fn has_to(&self) -> bool {
        !self.to.is_empty()
    }
}

/// Body of `POST /targets`.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateTargetRequest {
    pub code: String,

    pub channel: Channel,

    pub name: String,
}

impl CreateTargetRequest {
    pub fn normalize(&mut self) {
        self.code = self.code.trim().to_ascii_uppercase();
        self.name = self.name.trim().to_string();
    }

    pub fn validate(&self) -> AppResult<()> {
        if self.code.is_empty() {
            return Err(AppError::bad_request("code is required"));
        }

        if !self
            .code
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(AppError::bad_request("code must be alphanumeric, '_'"));
        }

        if self.name.is_empty() {
            return Err(AppError::bad_request("name is required"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PutTargetRequest {
    pub name: String,
}

impl PutTargetRequest {
    pub fn normalize(&mut self) {
        self.name = self.name.trim().to_string();
    }
}
