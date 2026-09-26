use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub enabled: bool,
    #[serde(default)]
    pub api_key: String,
}

impl AuthConfig {
    pub fn matches(&self, provided: &str) -> bool {
        let expected = self.api_key.trim();
        if expected.is_empty() || provided.len() != expected.len() {
            return false;
        }
        provided
            .bytes()
            .zip(expected.bytes())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
    }
}
