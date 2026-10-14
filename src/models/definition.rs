use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::enums::{Level, Status};
use crate::error::{AppError, AppResult};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DefinitionData {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub status: Status,
    pub level: Level,
    pub source_id: i32,
    pub channel_id: i32,
    pub targets: Vec<DefinitionTarget>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl DefinitionData {
    pub fn map_row(row: &tokio_postgres::Row) -> AppResult<Self> {
        let status: String = row.get("status");
        let level: String = row.get("level");
        Ok(Self {
            id: row.get("id"),
            code: row.get("code"),
            name: row.get("name"),
            description: row.get("description"),
            status: Status::parse(&status).ok_or_else(|| {
                AppError::internal(format!("unknown status `{status}` in definitions"))
            })?,
            level: Level::parse(&level).ok_or_else(|| {
                AppError::internal(format!("unknown level `{level}` in definitions"))
            })?,
            source_id: row.get("source_id"),
            channel_id: row.get("channel_id"),
            targets: serde_json::from_value(row.get("targets")).map_err(|error| {
                AppError::internal(format!("invalid definition target mapping: {error}"))
            })?,
            created_at: row.get::<_, DateTime<Utc>>("created_at"),
            updated_at: row.get::<_, Option<DateTime<Utc>>>("updated_at"),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DefinitionTarget {
    pub target_id: i32,
    pub template_id: Option<i32>,
}

/// Body of both create and update. Update takes `code` from the path, and
/// keeps the stored `status` / `level` when they are omitted; `targets`
/// always replaces the linked targets, in the given order.
#[derive(Debug, Clone, Deserialize)]
pub struct SaveDefinitionRequest {
    #[serde(default)]
    pub code: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub level: Option<String>,
    pub source_code: String,
    pub channel_code: String,
    pub targets: Vec<DefinitionTargetRequest>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DefinitionTargetRequest {
    pub target_code: String,
    #[serde(default)]
    pub template_id: Option<i32>,
}

impl SaveDefinitionRequest {
    pub fn normalize(&mut self) {
        self.code = self.code.trim().to_ascii_uppercase();
        self.name = self.name.trim().to_string();
        self.description = self
            .description
            .take()
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty());
        self.status = self.status.take().map(|s| s.trim().to_ascii_uppercase());
        self.level = self.level.take().map(|l| l.trim().to_ascii_uppercase());
        self.source_code = self.source_code.trim().to_ascii_uppercase();
        self.channel_code = self.channel_code.trim().to_ascii_uppercase();
        for target in &mut self.targets {
            target.target_code = target.target_code.trim().to_ascii_uppercase();
        }
    }

    pub fn validate(&self) -> AppResult<()> {
        if self.code.is_empty() {
            return Err(AppError::bad_request("code is required"));
        }
        if self.code.len() > 50 {
            return Err(AppError::bad_request("code must be at most 50 characters"));
        }
        if !self
            .code
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(AppError::bad_request(
                "code must be alphanumeric, '_' or '-'",
            ));
        }
        if self.name.is_empty() {
            return Err(AppError::bad_request("name is required"));
        }
        if self.name.chars().count() > 255 {
            return Err(AppError::bad_request("name must be at most 255 characters"));
        }
        if let Some(description) = &self.description {
            if description.chars().count() > 255 {
                return Err(AppError::bad_request(
                    "description must be at most 255 characters",
                ));
            }
        }
        if let Some(status) = &self.status {
            if !matches!(
                Status::parse(status),
                Some(Status::Active | Status::Inactive)
            ) {
                return Err(AppError::bad_request("status must be ACTIVE or INACTIVE"));
            }
        }
        if let Some(level) = &self.level {
            if Level::parse(level).is_none() {
                return Err(AppError::bad_request(
                    "level must be INFO, WARN, ERROR or CRITICAL",
                ));
            }
        }
        if self.source_code.is_empty() {
            return Err(AppError::bad_request("source_code is required"));
        }
        if self.channel_code.is_empty() {
            return Err(AppError::bad_request("channel_code is required"));
        }
        if self.targets.is_empty() {
            return Err(AppError::bad_request("targets must not be empty"));
        }
        for (i, target) in self.targets.iter().enumerate() {
            if target.target_code.is_empty() {
                return Err(AppError::bad_request("targets.target_code is required"));
            }
            if self.targets[..i]
                .iter()
                .any(|t| t.target_code == target.target_code)
            {
                return Err(AppError::bad_request(format!(
                    "target `{}` is listed more than once",
                    target.target_code
                )));
            }
        }
        Ok(())
    }
}
