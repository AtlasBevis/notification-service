use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::enums::NotificationStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationData {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub status: String,
    pub source: String,
    pub target: String,
    pub channel: String,
    pub template_id: i64,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl NotificationData {
    pub fn map_row(row: &tokio_postgres::Row) -> Self {
        Self {
            id: row.get("id"),
            code: row.get("code"),
            name: row.get("name"),
            status: row.get("status"),
            source: row.get("source"),
            target: row.get("target"),
            channel: row.get("channel"),
            template_id: row.get("template_id"),
            metadata: row.get("metadata"),
            created_at: row.get::<_, DateTime<Utc>>("created_at"),
            updated_at: row.get::<_, DateTime<Utc>>("updated_at"),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateNotificationRequest {
    pub code: String,

    pub name: String,

    pub source: String,

    pub target: String,

    pub channel: String,

    pub template_id: i64,

    #[serde(default)]
    pub metadata: Map<String, Value>,
}

impl CreateNotificationRequest {
    pub fn normalize(&mut self) {
        self.code = self.code.trim().to_ascii_uppercase();
        self.name = self.name.trim().to_string();
        self.source = self.source.trim().to_ascii_uppercase();
        self.target = self.target.trim().to_ascii_uppercase();
        self.channel = self.channel.trim().to_ascii_uppercase();
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_code(&self.code)?;
        if self.name.is_empty() {
            return Err("name is required".into());
        }
        if self.source.is_empty() {
            return Err("source is required".into());
        }
        if self.target.is_empty() {
            return Err("target is required".into());
        }
        if self.channel.is_empty() {
            return Err("channel is required".into());
        }
        if self.template_id <= 0 {
            return Err("template_id is required".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateNotificationRequest {
    pub name: String,

    pub status: String,

    pub source: String,

    pub target: String,

    pub channel: String,

    pub template_id: i64,

    #[serde(default)]
    pub metadata: Map<String, Value>,
}

impl UpdateNotificationRequest {
    pub fn normalize(&mut self) {
        self.name = self.name.trim().to_string();
        self.status = self.status.trim().to_ascii_uppercase();
        self.source = self.source.trim().to_ascii_uppercase();
        self.target = self.target.trim().to_ascii_uppercase();
        self.channel = self.channel.trim().to_ascii_uppercase();
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("name is required".into());
        }
        match NotificationStatus::parse(&self.status) {
            Some(NotificationStatus::Active | NotificationStatus::Inactive) => {}
            _ => return Err("status must be ACTIVE or INACTIVE".into()),
        }
        if self.source.is_empty() {
            return Err("source is required".into());
        }
        if self.target.is_empty() {
            return Err("target is required".into());
        }
        if self.channel.is_empty() {
            return Err("channel is required".into());
        }
        if self.template_id <= 0 {
            return Err("template_id is required".into());
        }
        Ok(())
    }
}

fn validate_code(code: &str) -> Result<(), String> {
    if code.is_empty() {
        return Err("code is required".into());
    }
    if !code
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err("code must be alphanumeric, '_' or '-'".into());
    }
    Ok(())
}
