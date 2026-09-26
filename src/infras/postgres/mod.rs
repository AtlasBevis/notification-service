use crate::config::PostgresConfig;
use anyhow::{Context, Result};
use deadpool_postgres::{Manager, Pool, PoolConfig, Runtime};
use tokio_postgres::{Config, NoTls};

#[derive(Clone)]
pub struct PostgresManager {
    pool: Pool,
}

impl PostgresManager {
    pub fn new(config: &PostgresConfig) -> Result<Self> {
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
            .context("Failed to build postgres pool")?;

        Ok(Self { pool })
    }

    /// Shared pool for repositories / services.
    pub fn pool(&self) -> Pool {
        self.pool.clone()
    }

    pub async fn ping(&self) -> Result<()> {
        let client = self
            .pool
            .get()
            .await
            .context("Failed to get postgres connection")?;
        client
            .simple_query("SELECT 1")
            .await
            .context("Postgres ping failed")?;
        Ok(())
    }
}
