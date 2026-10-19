use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::enums::{Channel, Level, Status};
use crate::error::{AppError, AppResult};
use crate::models::{ChannelData, SourceData, TargetInternalData, TemplateData};

const DEFINITION_CODE_LEN: usize = 50;

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
    pub targets: Vec<DefinitionTargetRequest>,
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
pub struct DefinitionTargetRequest {
    pub code: String,
    pub template_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateDefinitionRequest {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub status: Status,
    pub level: Level,
    pub source: String,
    pub channel: Channel,
    pub targets: Vec<DefinitionTargetRequest>,
}

impl CreateDefinitionRequest {
    pub fn normalize(&mut self) {
        self.code = self.code.trim().to_ascii_uppercase();
        self.name = self.name.trim().to_string();
        self.source = self.source.trim().to_ascii_uppercase();
        self.description = self
            .description
            .take()
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty());
        for target in &mut self.targets {
            target.code = target.code.trim().to_ascii_uppercase();
            target.template_code = target
                .template_code
                .take()
                .map(|c| c.trim().to_ascii_uppercase())
                .filter(|c| !c.is_empty());
        }
    }

    pub fn validate(&self) -> AppResult<()> {
        if self.code.is_empty() {
            return Err(AppError::bad_request("code is required"));
        }
        if self.code.len() > DEFINITION_CODE_LEN {
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
        if self.source.is_empty() {
            return Err(AppError::bad_request("source is required"));
        }
        if self.targets.is_empty() {
            return Err(AppError::bad_request("targets must not be empty"));
        }
        for (i, target) in self.targets.iter().enumerate() {
            if target.code.is_empty() {
                return Err(AppError::bad_request("targets.code is required"));
            }
            if self.targets[..i].iter().any(|t| t.code == target.code) {
                return Err(AppError::bad_request(format!(
                    "target `{}` is listed more than once",
                    target.code
                )));
            }
        }
        Ok(())
    }
}

/// Body of `PUT /definitions/{code}`; the code comes from the URL. Omitted
/// `status` / `level` keep the stored value; `targets` replaces the linked
/// targets, in the given order.
#[derive(Debug, Clone, Deserialize)]
pub struct PutDefinitionRequest {
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub level: Option<String>,
    pub source: String,
    pub channel: Channel,
    pub targets: Vec<DefinitionTargetRequest>,
}

impl PutDefinitionRequest {
    pub fn normalize(&mut self) {
        self.name = self.name.trim().to_string();
        self.source = self.source.trim().to_ascii_uppercase();
        self.description = self
            .description
            .take()
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty());
        self.status = self.status.take().map(|s| s.trim().to_ascii_uppercase());
        self.level = self.level.take().map(|l| l.trim().to_ascii_uppercase());
        for target in &mut self.targets {
            target.code = target.code.trim().to_ascii_uppercase();
            target.template_code = target
                .template_code
                .take()
                .map(|c| c.trim().to_ascii_uppercase())
                .filter(|c| !c.is_empty());
        }
    }

    pub fn validate(&self) -> AppResult<()> {
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
        if self.source.is_empty() {
            return Err(AppError::bad_request("source is required"));
        }
        if self.targets.is_empty() {
            return Err(AppError::bad_request("targets must not be empty"));
        }
        for (i, target) in self.targets.iter().enumerate() {
            if target.code.is_empty() {
                return Err(AppError::bad_request("targets.code is required"));
            }
            if self.targets[..i].iter().any(|t| t.code == target.code) {
                return Err(AppError::bad_request(format!(
                    "target `{}` is listed more than once",
                    target.code
                )));
            }
        }
        Ok(())
    }
}

// --------------------------
//  use for INTERNAL
pub struct DefinitionTargetDataInternal {
    targets_map: HashMap<i32, TargetInternalData>,
    template_map: HashMap<i32, Option<TemplateData>>,
}

pub struct DefinitionInternal {
    source: SourceData,
    channel: ChannelData,
    targets: DefinitionTargetDataInternal,
}
