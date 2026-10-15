use crate::config::PostgresConfig;
use crate::error::{AppError, AppResult};
use deadpool_postgres::{Manager, Pool, PoolConfig, Runtime};
use tokio_postgres::{Config, NoTls};

#[derive(Clone)]
pub struct PostgresManager {
    pool: Pool,
}

impl PostgresManager {
    pub fn new(config: &PostgresConfig) -> AppResult<Self> {
        let mut pg = Config::new();
        pg.host(&config.host);
        pg.port(config.port);
        pg.user(&config.username);
        if !config.password.is_empty() {
            pg.password(&config.password);
        }
        pg.dbname(&config.database);
        pg.application_name(PostgresConfig::application_name());

        let mgr = Manager::new(pg, NoTls);
        let pool = Pool::builder(mgr)
            .config(PoolConfig::new(config.max_pool_size.max(1)))
            .runtime(Runtime::Tokio1)
            .build()
            .map_err(|e| AppError::internal_with("Failed to build postgres pool", e))?;

        Ok(Self { pool })
    }

    /// Shared pool for repositories / services.
    pub fn pool(&self) -> Pool {
        self.pool.clone()
    }

    pub async fn ping(&self) -> AppResult<()> {
        let client = self.pool.get().await.map_err(|e| {
            AppError::internal_with("Failed to get postgres connection", AppError::from(e))
        })?;
        client
            .simple_query("SELECT 1")
            .await
            .map_err(|e| AppError::internal_with("Postgres ping failed", AppError::from(e)))?;
        Ok(())
    }
}
