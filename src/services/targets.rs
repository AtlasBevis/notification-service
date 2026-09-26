use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::enums::NotificationStatus;
use crate::error::{AppError, AppResult};
use crate::models::{CreateTargetRequest, PatchMetadataRequest, TargetData};
use crate::repository::TargetsRepository;

#[async_trait]
pub trait TargetsServiceTrait: Send + Sync {
    async fn load(&self) -> anyhow::Result<()>;
    async fn get(&self, code: &str) -> Option<TargetData>;
    async fn get_by_code(&self, code: &str) -> AppResult<Option<TargetData>>;
    async fn must_active(&self, code: &str) -> Result<TargetData, String>;
    async fn create(&self, req: CreateTargetRequest) -> AppResult<()>;
    async fn patch_metadata(&self, req: PatchMetadataRequest) -> AppResult<()>;
}

#[derive(Clone)]
pub struct TargetsService {
    repo: TargetsRepository,
    cache: Arc<RwLock<HashMap<String, TargetData>>>,
}

impl TargetsService {
    pub fn new(repo: TargetsRepository) -> Self {
        Self {
            repo,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl TargetsServiceTrait for TargetsService {
    async fn load(&self) -> anyhow::Result<()> {
        let rows = self.repo.list().await?;

        let mut map = HashMap::new();
        for row in rows {
            map.insert(row.code.clone(), row);
        }

        *self.cache.write().await = map;
        Ok(())
    }

    async fn get(&self, code: &str) -> Option<TargetData> {
        let key = code.trim().to_ascii_uppercase();
        self.cache.read().await.get(&key).cloned()
    }

    async fn get_by_code(&self, code: &str) -> AppResult<Option<TargetData>> {
        if let Some(t) = self.get(code).await {
            return Ok(Some(t));
        }

        match self.repo.find_by_code(code).await {
            Ok(Some(t)) => {
                self.cache.write().await.insert(code.to_owned(), t.clone());
                Ok(Some(t))
            }
            Ok(None) => Ok(None),
            Err(err) => {
                tracing::error!(error = %err, code, "find target by code failed");
                Err(err)
            }
        }
    }

    async fn must_active(&self, code: &str) -> Result<TargetData, String> {
        let key = code.trim().to_ascii_uppercase();
        if let Some(t) = self.get(&key).await {
            return active_target(t, code);
        }

        match self.repo.find_by_code(&key).await {
            Ok(Some(t)) => {
                self.cache.write().await.insert(key, t.clone());
                active_target(t, code)
            }
            Ok(None) => Err(format!("unknown target `{code}`")),
            Err(err) => {
                tracing::error!(error = %err, code, "load target failed");
                Err(format!("failed to load target `{code}`: {err}"))
            }
        }
    }

    async fn create(&self, req: CreateTargetRequest) -> AppResult<()> {
        match self.repo.insert(&req).await {
            Ok(row) => {
                self.cache.write().await.insert(req.code, row);
                Ok(())
            }
            Err(AppError::Conflict(_)) => Err(AppError::conflict(format!(
                "target `{}` already exists",
                req.code.trim()
            ))),
            Err(err) => {
                tracing::error!(error = %err, code = %req.code, "insert target failed");
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
                "target `{}` not found",
                req.code
            ))),
            Err(err) => {
                tracing::error!(error = %err, code = %req.code, "patch target metadata failed");
                Err(err)
            }
        }
    }
}

fn active_target(t: TargetData, code: &str) -> Result<TargetData, String> {
    if t.status
        .eq_ignore_ascii_case(NotificationStatus::Active.as_str())
    {
        Ok(t)
    } else {
        Err(format!("target `{code}` is not ACTIVE"))
    }
}
