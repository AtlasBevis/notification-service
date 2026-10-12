use axum::async_trait;

use crate::error::AppResult;

#[async_trait]
pub trait EmailTrait: Send + Sync {
    async fn send_email() -> AppResult<()>;
}

pub struct EmailService {}

#[async_trait]
impl EmailTrait for EmailService {
    async fn send_email() -> AppResult<()> {
        Ok(())
    }
}
