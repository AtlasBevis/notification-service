use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::enums::Status;
use crate::error::{AppError, AppResult};
use crate::models::ChannelData;
use crate::repository::ChannelRepository;

#[async_trait]
pub trait ChannelTrait: Send + Sync {
    async fn load(&self) -> AppResult<()>;
    async fn get(&self, code: &str) -> Option<ChannelData>;
    async fn get_active_by_id(&self, id: i32) -> AppResult<ChannelData>;
}

#[derive(Clone)]
pub struct ChannelService {
    repo: ChannelRepository,
    cache: Arc<RwLock<HashMap<String, ChannelData>>>,
}

impl ChannelService {
    pub fn new(repo: ChannelRepository) -> Self {
        Self {
            repo,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl ChannelTrait for ChannelService {
    async fn load(&self) -> AppResult<()> {
        let rows = self.repo.list().await?;

        let mut map = HashMap::new();
        for row in rows {
            map.insert(row.code.as_str().to_string(), row);
        }

        *self.cache.write().await = map;
        Ok(())
    }

    async fn get(&self, code: &str) -> Option<ChannelData> {
        let key = code.trim().to_ascii_uppercase();
        self.cache.read().await.get(&key).cloned()
    }

    async fn get_active_by_id(&self, id: i32) -> AppResult<ChannelData> {
        let c = self.repo.find_by_id(id).await?;
        self.cache
            .write()
            .await
            .insert(c.code.as_str().to_string(), c.clone());
        active_channel(c, &format!("id `{id}`"))
    }
}

fn active_channel(c: ChannelData, label: &str) -> AppResult<ChannelData> {
    if c.status == Status::Active {
        Ok(c)
    } else {
        Err(AppError::bad_request(format!(
            "channel {label} is not ACTIVE"
        )))
    }
}
