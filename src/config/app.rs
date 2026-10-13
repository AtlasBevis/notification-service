use super::{AuthConfig, KafkaConfig, PostgresConfig, ServerConfig};
use crate::error::{AppError, AppResult};
use crate::utils::get_env;
use serde::Deserialize;
use std::fs;
use std::sync::OnceLock;

const CONFIG_PATH: &str = "CONFIG_PATH";
static CONFIG: OnceLock<Config> = OnceLock::new();
static PROFILE: OnceLock<String> = OnceLock::new();

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub auth: Option<AuthConfig>,
    pub postgres: PostgresConfig,
    #[serde(default)]
    pub kafka: KafkaConfig,
}

impl Config {
    fn new_from_yaml(content: &str) -> AppResult<Self> {
        serde_yaml::from_str(content)
            .map_err(|e| AppError::internal_with("Failed to parse YAML", e))
    }

    /// Resolve the config path, then load config and init the profile.
    ///
    /// - `CONFIG_PATH`, otherwise `default_path` (`app.yaml` for the API,
    ///   `app-job.yaml` for `notification-job`)
    pub fn load_from_env(default_path: &str) -> AppResult<Self> {
        let config_path = get_env(CONFIG_PATH).unwrap_or_else(|| default_path.to_string());

        tracing::info!("CONFIG_PATH={} => loading config", config_path);
        let content = fs::read_to_string(&config_path)
            .map_err(|e| AppError::internal_with("Failed to read config file", e))?;
        let mut config = Self::new_from_yaml(&content)?;
        init_profile(config.server.profile.clone());

        apply_secrets(&mut config);

        set(config.clone());
        Ok(config)
    }
}

/// Initialize global config once at startup (via `load_config`).
/// Panics if called again.
pub fn set(config: Config) {
    CONFIG
        .set(config)
        .expect("config already initialized; can only set once");
}

/// Immutable config loaded at startup.
pub fn load() -> &'static Config {
    CONFIG
        .get()
        .expect("config not initialized; call load_config() first")
}

/// Set once at process startup from loaded config profile.
pub fn init_profile(profile: impl Into<String>) {
    let value = profile.into().trim().to_ascii_lowercase();
    if PROFILE.set(value).is_err() {
        tracing::warn!("PROFILE already initialized; ignoring re-init");
    }
}

pub fn profile() -> &'static str {
    PROFILE.get().map(String::as_str).unwrap_or("app")
}

pub fn load_config(default_path: &str) -> AppResult<Config> {
    Config::load_from_env(default_path)
}

/// Override sensitive fields from env when set (non-empty).
fn apply_secrets(config: &mut Config) {
    if let Some(v) = get_env("POSTGRES_PASSWORD") {
        config.postgres.password = v;
    }
    if let Some(v) = get_env("KAFKA_BROKERS") {
        config.kafka.brokers = v
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }
    if let Some(v) = get_env("KAFKA_SASL_USERNAME") {
        config.kafka.sasl.username = v;
    }
    if let Some(v) = get_env("KAFKA_SASL_PASSWORD") {
        config.kafka.sasl.password = v;
    }
}
