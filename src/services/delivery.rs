use async_trait::async_trait;

use crate::error::AppResult;
use crate::repository::DeliveriesRepository;

#[async_trait]
pub trait DeliveryTrait: Send + Sync {
    async fn record_result(
        &self,
        delivery_id: i64,
        status: &str,
        error_message: Option<&str>,
    ) -> AppResult<()>;

    async fn record_attempt(
        &self,
        delivery_id: i64,
        delivery_status: &str,
        attempt_status: &str,
        error_message: Option<&str>,
    ) -> AppResult<()>;
}

#[derive(Clone)]
pub struct DeliveryService {
    repository: DeliveriesRepository,
}

impl DeliveryService {
    pub fn new(repository: DeliveriesRepository) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl DeliveryTrait for DeliveryService {
    async fn record_result(
        &self,
        delivery_id: i64,
        status: &str,
        error_message: Option<&str>,
    ) -> AppResult<()> {
        self.repository
            .record_result(delivery_id, status, error_message)
            .await
    }

    async fn record_attempt(
        &self,
        delivery_id: i64,
        delivery_status: &str,
        attempt_status: &str,
        error_message: Option<&str>,
    ) -> AppResult<()> {
        self.repository
            .record_attempt(delivery_id, delivery_status, attempt_status, error_message)
            .await
    }
}
