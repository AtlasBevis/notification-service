use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct PostgresConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: String,
    pub max_pool_size: usize,
}

impl PostgresConfig {
    pub fn application_name() -> &'static str {
        "data-notification"
    }
}
