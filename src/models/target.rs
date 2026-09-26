use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::ToSchema;

use crate::utils::json::get_string;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TargetData {
    pub code: String,
    pub name: String,
    pub status: String,
    #[schema(value_type = Object)]
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TargetData {
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

    /// EMAIL addresses from metadata `{ "to": [], "cc": [], "bcc": [] }`.
    pub fn recipients(&self) -> Recipients {
        Recipients::from_metadata(&self.metadata)
    }

    /// MSTEAMS webhook URL from `metadata.url` (fallback when channel has no url).
    pub fn teams_webhook_url(&self) -> Option<String> {
        get_string(&self.metadata, "url")
    }
}

/// Resolved email destinations (snapshot for deliveries / dispatch).
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct Recipients {
    #[serde(default)]
    pub to: Vec<String>,

    #[serde(default)]
    pub cc: Vec<String>,

    #[serde(default)]
    pub bcc: Vec<String>,
}

impl Recipients {
    pub fn from_metadata(metadata: &Value) -> Self {
        Self {
            to: string_list(metadata, "to"),
            cc: string_list(metadata, "cc"),
            bcc: string_list(metadata, "bcc"),
        }
    }

    pub fn has_to(&self) -> bool {
        !self.to.is_empty()
    }
}

fn string_list(metadata: &Value, key: &str) -> Vec<String> {
    metadata
        .get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateTargetRequest {
    #[schema(example = "TEAM_MIS")]
    pub code: String,

    #[schema(example = "MIS Teams webhook")]
    pub name: String,

    #[serde(default)]
    #[schema(value_type = Object)]
    pub metadata: Map<String, Value>,
}

impl CreateTargetRequest {
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
