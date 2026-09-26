use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::enums::NotificationStatus;
use crate::models::ChannelData;
use crate::repository::ChannelsRepository;

#[async_trait]
pub trait ChannelsServiceTrait: Send + Sync {
    async fn load(&self) -> anyhow::Result<()>;
    async fn get(&self, code: &str) -> Option<ChannelData>;
    async fn must_active(&self, code: &str) -> Result<ChannelData, String>;
}

#[derive(Clone)]
pub struct ChannelsService {
    repo: ChannelsRepository,
    cache: Arc<RwLock<HashMap<String, ChannelData>>>,
}

impl ChannelsService {
    pub fn new(repo: ChannelsRepository) -> Self {
        Self {
            repo,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl ChannelsServiceTrait for ChannelsService {
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
        self.cache.read().await.get(code).cloned()
    }

    async fn must_active(&self, code: &str) -> Result<ChannelData, String> {
        let key = code.trim().to_ascii_uppercase();
        if let Some(c) = self.get(&key).await {
            return active_channel(c, code);
        }

        match self.repo.find_by_code(&key).await {
            Ok(Some(c)) => {
                self.cache.write().await.insert(key, c.clone());
                active_channel(c, code)
            }
            Ok(None) => Err(format!("unknown channel `{code}`")),
            Err(err) => {
                tracing::error!(error = %err, code, "load channel failed");
                Err(format!("failed to load channel `{code}`: {err}"))
            }
        }
    }
}

fn active_channel(c: ChannelData, code: &str) -> Result<ChannelData, String> {
    if c.status
        .eq_ignore_ascii_case(NotificationStatus::Active.as_str())
    {
        Ok(c)
    } else {
        Err(format!("channel `{code}` is not ACTIVE"))
    }
}
