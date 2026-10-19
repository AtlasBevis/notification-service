use serde::{Deserialize, Serialize};

pub const INFO: &str = "INFO";
pub const ERROR: &str = "ERROR";
pub const WARN: &str = "WARN";
pub const CRITICAL: &str = "CRITICAL";

#[derive(Debug, Clone, PartialEq, Eq, Copy, Serialize, Deserialize, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Level {
    Info,
    Error,
    Warn,
    Critical,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => INFO,
            Self::Error => ERROR,
            Self::Warn => WARN,
            Self::Critical => CRITICAL,
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            INFO => Some(Self::Info),
            ERROR => Some(Self::Error),
            WARN => Some(Self::Warn),
            CRITICAL => Some(Self::Critical),
            _ => None,
        }
    }
}

impl std::fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<Level> for String {
    fn from(value: Level) -> Self {
        value.as_str().to_string()
    }
}
