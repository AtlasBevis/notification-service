use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::enums::{Channel, Status};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelData {
    pub id: i32,
    pub code: Channel,
    pub name: String,
    pub status: Status,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl ChannelData {
    pub fn map_row(row: &tokio_postgres::Row) -> AppResult<Self> {
        let code: String = row.get("code");
        let status: String = row.get("status");
        Ok(Self {
            id: row.get("id"),
            code: Channel::parse(&code).ok_or_else(|| {
                AppError::internal(format!("unknown channel `{code}` in database"))
            })?,
            name: row.get("name"),
            status: Status::parse(&status).ok_or_else(|| {
                AppError::internal(format!("unknown status `{status}` in channels"))
            })?,
            created_at: row.get::<_, DateTime<Utc>>("created_at"),
            updated_at: row.get::<_, Option<DateTime<Utc>>>("updated_at"),
        })
    }
}
