use serde::{Deserialize, Serialize};

pub const EMAIL: &str = "EMAIL";
pub const WEBHOOK: &str = "WEBHOOK";
pub const MSTEAMS: &str = "MSTEAMS";

#[derive(Debug, Clone, PartialEq, Eq, Copy, Serialize, Deserialize)]
pub enum ChannelEnum {
    MSTEAMS,
    WEBHOOK,
    EMAIL,
}

impl ChannelEnum {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MSTEAMS => MSTEAMS,
            Self::EMAIL => EMAIL,
            Self::WEBHOOK => WEBHOOK,
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            EMAIL => Some(Self::EMAIL),
            WEBHOOK => Some(Self::WEBHOOK),
            MSTEAMS => Some(Self::MSTEAMS),
            _ => None,
        }
    }
}

impl std::fmt::Display for ChannelEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<ChannelEnum> for String {
    fn from(value: ChannelEnum) -> Self {
        value.as_str().to_string()
    }
}
