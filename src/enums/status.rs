use serde::{Deserialize, Serialize};

pub const ACTIVE: &str = "ACTIVE";
pub const INACTIVE: &str = "INACTIVE";
pub const QUEUED: &str = "QUEUED";
pub const PROCESSING: &str = "PROCESSING";
pub const SENT: &str = "SENT";
pub const FAILED: &str = "FAILED";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Status {
    Active,
    Inactive,
    Queued,
    Processing,
    Sent,
    Failed,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => ACTIVE,
            Self::Inactive => INACTIVE,
            Self::Queued => QUEUED,
            Self::Processing => PROCESSING,
            Self::Sent => SENT,
            Self::Failed => FAILED,
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            ACTIVE => Some(Self::Active),
            INACTIVE => Some(Self::Inactive),
            QUEUED => Some(Self::Queued),
            PROCESSING => Some(Self::Processing),
            SENT => Some(Self::Sent),
            FAILED => Some(Self::Failed),
            _ => None,
        }
    }
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<Status> for String {
    fn from(value: Status) -> Self {
        value.as_str().to_string()
    }
}
