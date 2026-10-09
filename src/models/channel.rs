use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    pub code: String,
    pub name: String,
    pub status: String,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ChannelData {
    pub fn map_row(row: &tokio_postgres::Row) -> Self {
        Self {
            code: row.get("code"),
            name: row.get("name"),
            status: row.get("status"),
            metadata: row.get("metadata"),
            created_at: row.get::<_, DateTime<Utc>>("created_at"),
            updated_at: row.get::<_, DateTime<Utc>>("updated_at"),
        }
    }

    /// MSTEAMS: `metadata.url` — incoming webhook URL.
    pub fn teams_webhook_url(&self) -> Result<String, String> {
        get_string(&self.metadata, "url")
            .ok_or_else(|| format!("channel `{}` metadata.url is required", self.code))
    }

    /// EMAIL channel — SMTP via `metadata.host`, `username`, `password`.
    pub fn smtp_config(&self) -> Result<SmtpConfig, String> {
        Ok(SmtpConfig {
            host: get_string(&self.metadata, "host")
                .ok_or_else(|| format!("channel `{}` metadata.host is required", self.code))?,
            username: get_string(&self.metadata, "username")
                .ok_or_else(|| format!("channel `{}` metadata.username is required", self.code))?,
            password: get_string(&self.metadata, "password")
                .ok_or_else(|| format!("channel `{}` metadata.password is required", self.code))?,
        })
    }
}
