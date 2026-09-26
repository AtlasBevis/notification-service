use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Catalog / outbox statuses stored as uppercase strings in Postgres.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
#[schema(rename_all = "UPPERCASE")]
pub enum NotificationStatus {
    Active,
    Inactive,
    Queued,
    Processing,
    Sent,
    Failed,
}

impl NotificationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "ACTIVE",
            Self::Inactive => "INACTIVE",
            Self::Queued => "QUEUED",
            Self::Processing => "PROCESSING",
            Self::Sent => "SENT",
            Self::Failed => "FAILED",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_uppercase().as_str() {
            "ACTIVE" => Some(Self::Active),
            "INACTIVE" => Some(Self::Inactive),
            "QUEUED" => Some(Self::Queued),
            "PROCESSING" => Some(Self::Processing),
            "SENT" => Some(Self::Sent),
            "FAILED" => Some(Self::Failed),
            _ => None,
        }
    }
}

impl std::fmt::Display for NotificationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<NotificationStatus> for String {
    fn from(value: NotificationStatus) -> Self {
        value.as_str().to_string()
    }
}
