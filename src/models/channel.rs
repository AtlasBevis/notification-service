use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::enums::{Channel, Status};
use crate::error::{AppError, AppResult};
use crate::utils::json::get_string;

/// SMTP settings from EMAIL channel `metadata`.
#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelData {
    pub id: i32,
    pub code: Channel,
    pub name: String,
    pub status: Status,
    pub metadata: Value,
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
            metadata: row.get("metadata"),
            created_at: row.get::<_, DateTime<Utc>>("created_at"),
            updated_at: row.get::<_, Option<DateTime<Utc>>>("updated_at"),
        })
    }

    /// MSTEAMS: `metadata.url` — incoming webhook URL.
    pub fn teams_webhook_url(&self) -> AppResult<String> {
        get_string(&self.metadata, "url").ok_or_else(|| {
            AppError::bad_request(format!("channel `{}` metadata.url is required", self.code))
        })
    }

    /// EMAIL channel — SMTP via `metadata.host`, `username`, `password`.
    pub fn smtp_config(&self) -> AppResult<SmtpConfig> {
        Ok(SmtpConfig {
            host: get_string(&self.metadata, "host").ok_or_else(|| {
                AppError::bad_request(format!("channel `{}` metadata.host is required", self.code))
            })?,
            username: get_string(&self.metadata, "username").ok_or_else(|| {
                AppError::bad_request(format!(
                    "channel `{}` metadata.username is required",
                    self.code
                ))
            })?,
            password: get_string(&self.metadata, "password").ok_or_else(|| {
                AppError::bad_request(format!(
                    "channel `{}` metadata.password is required",
                    self.code
                ))
            })?,
        })
    }
}
