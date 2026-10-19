use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::enums::{Channel, Status};
use crate::error::{AppError, AppResult};
use crate::models::ChannelData;
use crate::repository::ChannelRepository;

#[async_trait]
pub trait ChannelTrait: Send + Sync {
    async fn get_by_code(&self, code: Channel) -> AppResult<ChannelData>;
    async fn get_active_by_id(&self, id: i32) -> AppResult<ChannelData>;
}

#[derive(Clone)]
pub struct ChannelService {
    repo: ChannelRepository,
    cache: Arc<RwLock<HashMap<Channel, ChannelData>>>,
}

impl ChannelService {
    pub async fn new(repo: ChannelRepository) -> AppResult<Self> {
        let rows = repo.list().await?;

        let mut map = HashMap::new();
        for row in rows {
            map.insert(row.code, row);
        }

        Ok(Self {
            repo,
            cache: Arc::new(RwLock::new(map)),
        })
    }

    async fn get(&self, code: &Channel) -> Option<ChannelData> {
        self.cache.read().await.get(code).cloned()
    }

    async fn set(&self, code: Channel, data: &ChannelData) {
        self.cache.write().await.insert(code, data.clone());
    }
}

#[async_trait]
impl ChannelTrait for ChannelService {
    async fn get_by_code(&self, code: Channel) -> AppResult<ChannelData> {
        if let Some(c) = self.get(&code).await {
            return Ok(c);
        }

        match self.repo.find_by_code(code).await {
            Ok(c) => {
                self.set(code, &c).await;
                Ok(c)
            }
            Err(err) => {
                tracing::error!(error = %err, code = %code, "find channel by code failed");
                Err(err)
            }
        }
    }

    async fn get_active_by_id(&self, id: i32) -> AppResult<ChannelData> {
        let c = self.repo.find_by_id(id).await?;
        self.set(c.code, &c).await;
        if Status::Active != c.status {
            return Err(AppError::bad_request(format!(
                "channel {} is not ACTIVE",
                c.code,
            )));
        }
        Ok(c)
    }
}
