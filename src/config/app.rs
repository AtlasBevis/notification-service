use super::{AuthConfig, KafkaConfig, PostgresConfig, ServerConfig};
use crate::error::{AppError, AppResult};
use crate::utils::get_env;
use serde::Deserialize;
use std::fs;
use std::sync::OnceLock;

const CONFIG_PATH: &str = "CONFIG_PATH";
static CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub auth: Option<AuthConfig>,
    pub postgres: PostgresConfig,
    pub kafka: KafkaConfig,
}

impl Config {
    /// Read and parse the config file, then apply secret overrides from env.
    ///
    /// The file is `CONFIG_PATH` when set, otherwise `default_path`
    /// (`app.yaml` for the API, `app-job.yaml` for `notification-job`).
    pub fn new(default_path: &str) -> AppResult<Config> {
        let path = get_env(CONFIG_PATH).unwrap_or_else(|| default_path.to_string());
        tracing::info!("CONFIG_PATH={} => loading config", path);

        let content = fs::read_to_string(&path).map_err(|e| {
            AppError::internal_with(format!("Failed to read config file `{path}`"), e)
        })?;
        let mut config: Config = serde_yaml::from_str(&content).map_err(|e| {
            AppError::internal_with(format!("Failed to parse config file `{path}`"), e)
        })?;
        config.apply_secrets();
        config.server.validate()?;
        Ok(config)
    }

    /// Store `config` as the process-wide config. Call once at startup.
    pub fn set(config: Config) -> AppResult<&'static Config> {
        CONFIG
            .set(config)
            .map_err(|_| AppError::internal("config already initialized; can only set once"))?;
        Ok(Self::load())
    }

    /// Process-wide config stored by `Config::set`.
    pub fn load() -> &'static Config {
        CONFIG
            .get()
            .expect("config not initialized; call Config::set() first")
    }

    /// Override sensitive fields from env when set (non-empty).
    fn apply_secrets(&mut self) {
        if let Some(v) = get_env("POSTGRES_PASSWORD") {
            self.postgres.password = v;
        }
        if let Some(v) = get_env("KAFKA_USERNAME") {
            self.kafka.sasl.username = v;
        }
        if let Some(v) = get_env("KAFKA_PASSWORD") {
            self.kafka.sasl.password = v;
        }
    }
}
