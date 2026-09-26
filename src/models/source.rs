use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SourceData {
    pub code: String,
    pub name: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[schema(value_type = Object)]
    pub metadata: Value,
}

impl SourceData {
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
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateSourceRequest {
    #[schema(example = "AIRFLOW")]
    pub code: String,

    #[schema(example = "Airflow")]
    pub name: String,

    #[serde(default)]
    #[schema(value_type = Object)]
    pub metadata: Map<String, Value>,
}

impl CreateSourceRequest {
    pub fn normalize(&mut self) {
        self.code = self.code.trim().to_ascii_uppercase();
        self.name = self.name.trim().to_string();
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.code.is_empty() {
            return Err("code is required".to_string());
        }
        if !self
            .code
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err("code must be alphanumeric, '_' or '-'".to_string());
        }
        if self.name.is_empty() {
            return Err("name is required".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PatchMetadataRequest {
    #[schema(example = "AIRFLOW")]
    pub code: String,

    #[serde(default)]
    #[schema(value_type = Object)]
    pub metadata: Map<String, Value>,
}

impl PatchMetadataRequest {
    pub fn normalize(&mut self) {
        self.code = self.code.trim().to_ascii_uppercase();
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.code.is_empty() {
            return Err("code is required".into());
        }
        if self.metadata.is_empty() {
            return Err("metadata must not be empty".into());
        }
        Ok(())
    }
}
