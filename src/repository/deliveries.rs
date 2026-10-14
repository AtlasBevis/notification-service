use crate::utils::TimeUtils;
use deadpool_postgres::Pool;

use crate::error::AppResult;

#[derive(Clone)]
pub struct DeliveriesRepository {
    pool: Pool,
}

impl DeliveriesRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Record the outcome of one channel call and append `delivery_attempts`.
    pub async fn record_result(
        &self,
        delivery_id: i64,
        status: &str,
        error_message: Option<&str>,
    ) -> AppResult<()> {
        let attempt_status = if status == "SENT" {
            "SUCCESS"
        } else if error_message.is_some_and(|message| message.contains("timed out")) {
            "TIMEOUT"
        } else {
            "FAILED"
        };
        self.record_attempt(delivery_id, status, attempt_status, error_message)
            .await
    }

    pub async fn record_attempt(
        &self,
        delivery_id: i64,
        delivery_status: &str,
        attempt_status: &str,
        error_message: Option<&str>,
    ) -> AppResult<()> {
        if delivery_id <= 0 {
            return Ok(());
        }
        let mut client = self.pool.get().await?;
        let tx = client.transaction().await?;
        let now = TimeUtils::now();
        let sent_at = if delivery_status == "SENT" {
            Some(now)
        } else {
            None
        };

        let row = tx
            .query_one(
                r#"
                UPDATE deliveries
                SET status = $2,
                    attempt_count = attempt_count + 1,
                    last_error_message = $3,
                    sent_at = COALESCE($4, sent_at),
                    updated_at = $5
                WHERE id = $1
                RETURNING attempt_count
                "#,
                &[
                    &delivery_id,
                    &delivery_status,
                    &error_message,
                    &sent_at,
                    &now,
                ],
            )
            .await?;
        let attempt_no: i32 = row.get("attempt_count");

        tx.execute(
            r#"
            INSERT INTO delivery_attempts (
                delivery_id, attempt_no, status, finished_at, error_message
            ) VALUES ($1,$2,$3,$4,$5)
            "#,
            &[
                &delivery_id,
                &attempt_no,
                &attempt_status,
                &now,
                &error_message,
            ],
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
}
