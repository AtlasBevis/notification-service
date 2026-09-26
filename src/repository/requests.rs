use deadpool_postgres::Pool;
use serde_json::{Map, Value};

use crate::error::AppError;

#[derive(Clone)]
pub struct RequestsRepository {
    pool: Pool,
}

impl RequestsRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn insert(
        &self,
        code: &str,
        trace_id: &str,
        variables: &Map<String, Value>,
    ) -> Result<(), AppError> {
        let client = self.pool.get().await?;
        let variables = Value::Object(variables.clone());
        client
            .execute(
                "INSERT INTO requests (code, trace_id, variables)
                 VALUES ($1, $2, $3)",
                &[&code, &trace_id, &variables],
            )
            .await?;
        Ok(())
    }
}
