use serde::{Deserialize, Serialize};

pub const FORMAT_TEXT: &str = "TEXT";
pub const FORMAT_HTML: &str = "HTML";
pub const FORMAT_ADAPTIVE_CARD: &str = "ADAPTIVE_CARD";
pub const FORMAT_JSON: &str = "JSON";

#[derive(Debug, Clone, PartialEq, Eq, Copy, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TemplateFormat {
    Text,
    Html,
    AdaptiveCard,
    Json,
}

impl TemplateFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => FORMAT_TEXT,
            Self::Html => FORMAT_HTML,
            Self::AdaptiveCard => FORMAT_ADAPTIVE_CARD,
            Self::Json => FORMAT_JSON,
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            FORMAT_TEXT => Some(Self::Text),
            FORMAT_HTML => Some(Self::Html),
            FORMAT_ADAPTIVE_CARD => Some(Self::AdaptiveCard),
            FORMAT_JSON => Some(Self::Json),
            _ => None,
        }
    }
}

impl std::fmt::Display for TemplateFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<TemplateFormat> for String {
    fn from(value: TemplateFormat) -> Self {
        value.as_str().to_string()
    }
}
