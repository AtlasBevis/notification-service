use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::enums::Status;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceData {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub status: Status,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub metadata: Value,
}

impl SourceData {
    pub fn map_row(row: &tokio_postgres::Row) -> AppResult<Self> {
        let status: String = row.get("status");
        Ok(Self {
            id: row.get("id"),
            code: row.get("code"),
            name: row.get("name"),
            status: Status::parse(&status).ok_or_else(|| {
                AppError::internal(format!("unknown status `{status}` in sources"))
            })?,
            metadata: row.get("metadata"),
            created_at: row.get::<_, DateTime<Utc>>("created_at"),
            updated_at: row.get::<_, Option<DateTime<Utc>>>("updated_at"),
        })
    }
}

/// Body of `POST /sources`.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateSourceRequest {
    pub code: String,
    pub name: String,
    #[serde(default)]
    pub metadata: Map<String, Value>,
}

impl CreateSourceRequest {
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

/// Body of `PUT /sources/{code}`; the code comes from the URL.
#[derive(Debug, Clone, Deserialize)]
pub struct PutSourceRequest {
    pub name: String,

    #[serde(default)]
    pub metadata: Map<String, Value>,
}

impl PutSourceRequest {
    pub fn normalize(&mut self) {
        self.name = self.name.trim().to_string();
    }
}
