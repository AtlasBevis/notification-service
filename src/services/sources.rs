use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::enums::NotificationStatus;
use crate::error::{AppError, AppResult};
use crate::models::{CreateSourceRequest, PatchMetadataRequest, SourceData};
use crate::repository::SourcesRepository;

#[async_trait]
pub trait SourcesServiceTrait: Send + Sync {
    async fn load(&self) -> anyhow::Result<()>;
    async fn get(&self, code: &str) -> Option<SourceData>;
    async fn get_by_code(&self, code: &str) -> AppResult<Option<SourceData>>;
    async fn must_active(&self, code: &str) -> Result<SourceData, String>;
    async fn create(&self, req: CreateSourceRequest) -> AppResult<()>;
    async fn patch_metadata(&self, req: PatchMetadataRequest) -> AppResult<()>;
}

#[derive(Clone)]
pub struct SourcesService {
    repo: SourcesRepository,
    cache: Arc<RwLock<HashMap<String, SourceData>>>,
}

impl SourcesService {
    pub fn new(repo: SourcesRepository) -> Self {
        Self {
            repo,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl SourcesServiceTrait for SourcesService {
    async fn load(&self) -> anyhow::Result<()> {
        let rows = self.repo.list().await?;

        let mut map = HashMap::new();
        for row in rows {
            map.insert(row.code.clone(), row);
        }

        *self.cache.write().await = map;
        Ok(())
    }

    async fn get(&self, code: &str) -> Option<SourceData> {
        let key = code.trim().to_ascii_uppercase();
        self.cache.read().await.get(&key).cloned()
    }

    async fn get_by_code(&self, code: &str) -> AppResult<Option<SourceData>> {
        if let Some(s) = self.get(code).await {
            return Ok(Some(s));
        }

        match self.repo.find_by_code(code).await {
            Ok(Some(s)) => {
                self.cache.write().await.insert(code.to_owned(), s.clone());
                Ok(Some(s))
            }
            Ok(None) => Ok(None),
            Err(err) => {
                tracing::error!(error = %err, code, "find source by code failed");
                Err(err)
            }
        }
    }

    async fn must_active(&self, code: &str) -> Result<SourceData, String> {
        let key = code.trim().to_ascii_uppercase();
        if let Some(s) = self.get(&key).await {
            return active_source(s, code);
        }

        match self.repo.find_by_code(&key).await {
            Ok(Some(s)) => {
                self.cache.write().await.insert(key, s.clone());
                active_source(s, code)
            }
            Ok(None) => Err(format!("unknown source `{code}`")),
            Err(err) => {
                tracing::error!(error = %err, code, "load source failed");
                Err(format!("failed to load source `{code}`: {err}"))
            }
        }
    }

    async fn create(&self, req: CreateSourceRequest) -> AppResult<()> {
        match self.repo.insert(&req).await {
            Ok(row) => {
                self.cache.write().await.insert(req.code, row);
                Ok(())
            }
            Err(AppError::Conflict(_)) => Err(AppError::conflict(format!(
                "source `{}` already exists",
                req.code.trim()
            ))),
            Err(err) => {
                tracing::error!(error = %err, code = %req.code, "insert source failed");
                Err(err)
            }
        }
    }

    async fn patch_metadata(&self, req: PatchMetadataRequest) -> AppResult<()> {
        match self.repo.patch_metadata(&req.code, &req.metadata).await {
            Ok(()) => {
                self.cache.write().await.remove(&req.code);
                Ok(())
            }
            Err(AppError::NotFound(_)) => Err(AppError::not_found(format!(
                "source `{}` not found",
                req.code
            ))),
            Err(err) => {
                tracing::error!(error = %err, code = %req.code, "patch source metadata failed");
                Err(err)
            }
        }
    }
}

fn active_source(s: SourceData, code: &str) -> Result<SourceData, String> {
    if s.status
        .eq_ignore_ascii_case(NotificationStatus::Active.as_str())
    {
        Ok(s)
    } else {
        Err(format!("source `{code}` is not ACTIVE"))
    }
}
