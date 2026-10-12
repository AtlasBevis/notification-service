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
    async fn load(&self) -> anyhow::Result<()>;
    async fn get(&self, code: &str) -> Option<ChannelData>;
    async fn must_active(&self, code: &str) -> Result<ChannelData, String>;
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
    async fn load(&self) -> anyhow::Result<()> {
        let rows = self.repo.list().await?;

        let mut map = HashMap::new();
        for row in rows {
            map.insert(row.code.clone(), row);
        }

        *self.cache.write().await = map;
        Ok(())
    }

    async fn get(&self, code: &str) -> Option<ChannelData> {
        let key = code.trim().to_ascii_uppercase();
        self.cache.read().await.get(&key).cloned()
    }

    async fn must_active(&self, code: &str) -> Result<ChannelData, String> {
        let key = code.trim().to_ascii_uppercase();
        if let Some(channel) = self.get(&key).await {
            return active_channel(channel, &format!("`{code}`"));
        }
        match self.repo.find_by_code(&key).await {
            Ok(Some(channel)) => {
                self.cache.write().await.insert(key, channel.clone());
                active_channel(channel, &format!("`{code}`"))
            }
            Ok(None) => Err(format!("unknown channel `{code}`")),
            Err(err) => Err(format!("failed to load channel `{code}`: {err}")),
        }
    }

    async fn get_active_by_id(&self, id: i32) -> AppResult<ChannelData> {
        match self.repo.find_by_id(id).await {
            Ok(Some(c)) => {
                self.cache.write().await.insert(c.code.clone(), c.clone());
                active_channel(c, &format!("id `{id}`")).map_err(AppError::bad_request)
            }
            Ok(None) => Err(AppError::not_found(format!("unknown channel id `{id}`"))),
            Err(err) => {
                tracing::error!(error = %err, id, "load channel failed");
                Err(err)
            }
        }
    }
}

fn active_channel(c: ChannelData, label: &str) -> Result<ChannelData, String> {
    if c.status.eq_ignore_ascii_case(Status::Active.as_str()) {
        Ok(c)
    } else {
        Err(format!("channel {label} is not ACTIVE"))
    }
}
