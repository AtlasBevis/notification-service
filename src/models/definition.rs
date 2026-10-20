use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::enums::{Channel, Level, Status};
use crate::error::{AppError, AppResult};
use crate::models::{ChannelData, SourceData};
use crate::utils::StringUtils;

struct DefinitionValidation;

impl DefinitionValidation {
    const CODE_LEN: usize = 50;
    const NAME_LEN: usize = 255;
    const DESCRIPTION_LEN: usize = 255;

    fn code(code: &str) -> AppResult<()> {
        if code.is_empty() {
            return Err(AppError::bad_request("code is required"));
        }
        if code.len() > Self::CODE_LEN {
            return Err(AppError::bad_request(format!(
                "code must be at most {} characters",
                Self::CODE_LEN
            )));
        }

        if !StringUtils::is_valid_code(code) {
            return Err(AppError::bad_request(
                "code must be alphanumeric, '_' or '-'",
            ));
        }
        Ok(())
    }

    fn name(name: &str, description: &Option<String>) -> AppResult<()> {
        if name.is_empty() {
            return Err(AppError::bad_request("name is required"));
        }
        if name.chars().count() > Self::NAME_LEN {
            return Err(AppError::bad_request(format!(
                "name must be at most {} characters",
                Self::NAME_LEN
            )));
        }
        if let Some(description) = description {
            if description.chars().count() > Self::DESCRIPTION_LEN {
                return Err(AppError::bad_request(format!(
                    "description must be at most {} characters",
                    Self::DESCRIPTION_LEN
                )));
            }
        }
        Ok(())
    }

    fn status(status: Status) -> AppResult<()> {
        if !matches!(status, Status::Active | Status::Inactive) {
            return Err(AppError::bad_request("status must be ACTIVE or INACTIVE"));
        }
        Ok(())
    }

    fn source(source: &str) -> AppResult<()> {
        if source.is_empty() {
            return Err(AppError::bad_request("source is required"));
        }
        Ok(())
    }

    fn targets(targets: &[DefinitionTargetRequest]) -> AppResult<()> {
        if targets.is_empty() {
            return Err(AppError::bad_request("targets must not be empty"));
        }
        for (i, target) in targets.iter().enumerate() {
            if target.code.is_empty() {
                return Err(AppError::bad_request("targets.code is required"));
            }
            if targets[..i].iter().any(|t| t.code == target.code) {
                return Err(AppError::bad_request(format!(
                    "target `{}` is listed more than once",
                    target.code
                )));
            }
        }
        Ok(())
    }

    fn normalize_targets(targets: &mut [DefinitionTargetRequest]) {
        for target in targets {
            target.code = target.code.trim().to_ascii_uppercase();
            target.template_code = target
                .template_code
                .take()
                .map(|c| c.trim().to_ascii_uppercase())
                .filter(|c| !c.is_empty());
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DefinitionData {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub status: Status,
    pub level: Level,
    pub source_id: i32,
    pub channel_id: i32,
    pub targets: Vec<DefinitionTargetData>,
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

/// One linked target, in `definition_targets` order. The ids are for sending
/// and are not part of the response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DefinitionTargetData {
    #[serde(skip_serializing)]
    pub target_id: i32,
    pub code: String,
    #[serde(skip_serializing)]
    pub template_id: Option<i32>,
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

#[derive(Clone, Debug, Deserialize)]
pub struct DefinitionTargetRequest {
    pub code: String,
    pub template_code: Option<String>,
}

impl CreateDefinitionRequest {
    pub fn normalize(&mut self) {
        self.code = self.code.trim().to_ascii_uppercase();
        self.name = self.name.trim().to_string();
        self.source = self.source.trim().to_ascii_uppercase();
        StringUtils::trim_op(&mut self.description);
        DefinitionValidation::normalize_targets(&mut self.targets);
    }

    pub fn validate(&self) -> AppResult<()> {
        DefinitionValidation::code(&self.code)?;
        DefinitionValidation::name(&self.name, &self.description)?;
        DefinitionValidation::status(self.status)?;
        DefinitionValidation::source(&self.source)?;
        DefinitionValidation::targets(&self.targets)
    }
}

/// Body of `PUT /definitions/{code}`; the code comes from the URL. Omitted
/// `status` / `level` keep the stored value; `targets` replaces the linked
/// targets, in the given order.
#[derive(Debug, Clone, Deserialize)]
pub struct PutDefinitionRequest {
    pub name: String,
    pub description: Option<String>,
    pub status: Option<Status>,
    pub level: Option<Level>,
    pub source: String,
    pub channel: Channel,
    pub targets: Vec<DefinitionTargetRequest>,
}

impl PutDefinitionRequest {
    pub fn normalize(&mut self) {
        self.name = self.name.trim().to_string();
        self.source = self.source.trim().to_ascii_uppercase();
        StringUtils::trim_op(&mut self.description);
        DefinitionValidation::normalize_targets(&mut self.targets);
    }

    pub fn validate(&self) -> AppResult<()> {
        DefinitionValidation::name(&self.name, &self.description)?;
        if let Some(status) = self.status {
            DefinitionValidation::status(status)?;
        }
        DefinitionValidation::source(&self.source)?;
        DefinitionValidation::targets(&self.targets)
    }
}

/// What `DefinitionService` resolved from a create / update request.
/// `target_ids[i]` is linked with `template_ids[i]`, in request order.
pub struct DefinitionInternal {
    pub source: SourceData,
    pub channel: ChannelData,
    pub target_ids: Vec<i32>,
    pub template_ids: Vec<Option<i32>>,
}
