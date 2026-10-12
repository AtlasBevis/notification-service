use serde::{Deserialize, Serialize};

pub const INFO: &str = "INFO";
pub const ERROR: &str = "ERROR";
pub const WARN: &str = "WARN";
pub const CRITICAL: &str = "CRITICAL";

#[derive(Debug, Clone, PartialEq, Eq, Copy, Serialize, Deserialize)]
pub enum Level {
    INFO,
    ERROR,
    WARN,
    CRITICAL,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::INFO => INFO,
            Self::ERROR => ERROR,
            Self::WARN => WARN,
            Self::CRITICAL => CRITICAL,
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            INFO => Some(Self::INFO),
            ERROR => Some(Self::ERROR),
            WARN => Some(Self::WARN),
            CRITICAL => Some(Self::CRITICAL),
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
