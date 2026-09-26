use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct PostgresConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: String,
    #[serde(default = "default_pool_size")]
    pub max_pool_size: usize,
}

fn default_pool_size() -> usize {
    10
}

impl PostgresConfig {
    pub fn application_name() -> &'static str {
        "data-notification"
    }
}
