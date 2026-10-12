use serde::{Deserialize, Serialize};

pub const GET: &str = "GET";
pub const POST: &str = "POST";
pub const PUT: &str = "PUT";
pub const PATCH: &str = "PATCH";

#[derive(Clone, Debug, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum Method {
    GET,
    POST,
    PUT,
    PATCH,
}

impl Method {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::GET => GET,
            Self::POST => POST,
            Self::PUT => PUT,
            Self::PATCH => PATCH,
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            GET => Some(Self::GET),
            POST => Some(Self::POST),
            PUT => Some(Self::PUT),
            PATCH => Some(Self::PATCH),
            _ => None,
        }
    }
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<Method> for String {
    fn from(value: Method) -> Self {
        value.as_str().to_string()
    }
}
