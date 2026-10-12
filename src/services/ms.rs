use std::sync::Arc;

use axum::async_trait;

use crate::error::AppResult;
use crate::infras::http::HttpClient;

#[async_trait]
pub trait MSTrait: Send + Sync {
    async fn send_teams() -> AppResult<()>;
}

pub struct MSService {
    http: Arc<HttpClient>,
}

#[async_trait]
impl MSTrait for MSService {
    async fn send_teams() -> AppResult<()> {
        Ok(())
    }
}

impl MSService {
    pub fn new(http: Arc<HttpClient>) -> Self {
        return Self { http };
    }
}
