use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::enums::StatusEnum;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationData {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub status: String,
    pub description: Option<String>,
    pub source_code: String,
    pub channel_code: String,
    pub target_code: String,
    pub template_id: i64,
    pub level: String,
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
            description: row.get("description"),
            source_code: row.get("source_code"),
            channel_code: row.get("channel_code"),
            target_code: row.get("target_code"),
            template_id: row.get("template_id"),
            level: row.get("level"),
            created_at: row.get::<_, DateTime<Utc>>("created_at"),
            updated_at: row.get::<_, DateTime<Utc>>("updated_at"),
        }
    }
}

/// Runtime row in `notifications`, created when a caller triggers a definition.
#[derive(Debug, Clone)]
pub struct AcceptedNotification {
    pub id: i64,
    pub public_id: Uuid,
    pub delivery_id: i64,
}

#[derive(Debug, Clone)]
pub struct DeliveryDraft {
    pub target_code: String,
    pub channel_code: String,
    pub title: String,
    pub body: String,
    pub payload: Value,
    pub destination: Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateNotificationRequest {
    pub code: String,
    pub name: String,
    pub source_code: String,
    pub channel_code: String,
    pub target_code: String,
    pub template_id: i64,
    #[serde(default = "default_level")]
    pub level: String,
    #[serde(default)]
    pub description: Option<String>,
}

fn default_level() -> String {
    "INFO".to_string()
}

impl CreateNotificationRequest {
    pub fn normalize(&mut self) {
        self.code = self.code.trim().to_ascii_uppercase();
        self.name = self.name.trim().to_string();
        self.source_code = self.source_code.trim().to_ascii_uppercase();
        self.channel_code = self.channel_code.trim().to_ascii_uppercase();
        self.target_code = self.target_code.trim().to_ascii_uppercase();
        self.level = self.level.trim().to_ascii_uppercase();
        if self.level.is_empty() {
            self.level = default_level();
        }
        self.description = self
            .description
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_code(&self.code)?;
        if self.name.is_empty() {
            return Err("name is required".into());
        }
        if self.source_code.is_empty() {
            return Err("source_code is required".into());
        }
        if self.channel_code.is_empty() {
            return Err("channel_code is required".into());
        }
        if self.target_code.is_empty() {
            return Err("target_code is required".into());
        }
        if self.template_id <= 0 {
            return Err("template_id is required".into());
        }
        validate_level(&self.level)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateNotificationRequest {
    pub name: String,
    pub status: String,
    pub source_code: String,
    pub channel_code: String,
    pub target_code: String,
    pub template_id: i64,
    pub level: String,
    #[serde(default)]
    pub description: Option<String>,
}

impl UpdateNotificationRequest {
    pub fn normalize(&mut self) {
        self.name = self.name.trim().to_string();
        self.status = self.status.trim().to_ascii_uppercase();
        self.source_code = self.source_code.trim().to_ascii_uppercase();
        self.channel_code = self.channel_code.trim().to_ascii_uppercase();
        self.target_code = self.target_code.trim().to_ascii_uppercase();
        self.level = self.level.trim().to_ascii_uppercase();
        self.description = self
            .description
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("name is required".into());
        }
        match StatusEnum::parse(&self.status) {
            Some(StatusEnum::Active | StatusEnum::Inactive) => {}
            _ => return Err("status must be ACTIVE or INACTIVE".into()),
        }
        if self.source_code.is_empty() {
            return Err("source_code is required".into());
        }
        if self.channel_code.is_empty() {
            return Err("channel_code is required".into());
        }
        if self.target_code.is_empty() {
            return Err("target_code is required".into());
        }
        if self.template_id <= 0 {
            return Err("template_id is required".into());
        }
        validate_level(&self.level)?;
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

fn validate_level(level: &str) -> Result<(), String> {
    match level {
        "INFO" | "WARN" | "ERROR" | "CRITICAL" => Ok(()),
        _ => Err("level must be INFO, WARN, ERROR or CRITICAL".into()),
    }
}
