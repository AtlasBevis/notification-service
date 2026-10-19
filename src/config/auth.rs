use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    enabled: bool,
    admin: String,
    api_key: String,
}

impl AuthConfig {
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn is_admin(&self, key: &str) -> bool {
        self.admin == key
    }

    pub fn matches(&self, provided: &str) -> bool {
        let expected = self.api_key.as_bytes();
        let provided = provided.as_bytes();

        if expected.len() != provided.len() {
            return false;
        }

        let diff = expected
            .iter()
            .zip(provided.iter())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b));

        diff == 0
    }
}
