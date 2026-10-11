use serde::{Deserialize, Serialize};

pub const ACTIVE: &str = "ACTIVE";
pub const INACTIVE: &str = "INACTIVE";
pub const QUEUED: &str = "QUEUED";
pub const PROCESSING: &str = "PROCESSING";
pub const SENT: &str = "SENT";
pub const FAILED: &str = "FAILED";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StatusEnum {
    ACTIVE,
    INACTIVE,
    QUEUED,
    PROCESSING,
    SENT,
    FAILED,
}

impl StatusEnum {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ACTIVE => ACTIVE,
            Self::INACTIVE => INACTIVE,
            Self::QUEUED => QUEUED,
            Self::PROCESSING => PROCESSING,
            Self::SENT => SENT,
            Self::FAILED => FAILED,
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            ACTIVE => Some(Self::ACTIVE),
            INACTIVE => Some(Self::INACTIVE),
            QUEUED => Some(Self::QUEUED),
            PROCESSING => Some(Self::PROCESSING),
            SENT => Some(Self::SENT),
            FAILED => Some(Self::FAILED),
            _ => None,
        }
    }
}

impl std::fmt::Display for StatusEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<StatusEnum> for String {
    fn from(value: StatusEnum) -> Self {
        value.as_str().to_string()
    }
}
