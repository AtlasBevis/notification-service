use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::enums::Status;
use crate::error::{AppError, AppResult};
use crate::models::{CreateSourceRequest, PatchMetadataRequest, SourceData};
use crate::repository::SourceRepository;

#[async_trait]
pub trait SourceTrait: Send + Sync {
    async fn load(&self) -> anyhow::Result<()>;
    async fn get(&self, code: &str) -> Option<SourceData>;
    async fn get_by_code(&self, code: &str) -> AppResult<SourceData>;
    async fn get_active_by_code(&self, code: &str) -> AppResult<SourceData>;
    async fn get_active_by_id(&self, id: i32) -> AppResult<SourceData>;
    async fn create(&self, req: CreateSourceRequest) -> AppResult<()>;
    async fn patch_metadata(&self, req: PatchMetadataRequest) -> AppResult<()>;
}

#[derive(Clone)]
pub struct SourceService {
    repo: SourceRepository,
    cache: Arc<RwLock<HashMap<String, SourceData>>>,
}

impl SourceService {
    pub fn new(repo: SourceRepository) -> Self {
        Self {
            repo,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl SourceTrait for SourceService {
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

    async fn get_by_code(&self, code: &str) -> AppResult<SourceData> {
        let key = code.trim().to_ascii_uppercase();
        if let Some(s) = self.get(&key).await {
            return Ok(s);
        }

        match self.repo.find_by_code(&key).await {
            Ok(s) => {
                self.cache.write().await.insert(key, s.clone());
                Ok(s)
            }
            Err(err) => {
                tracing::error!(error = %err, code, "find source by code failed");
                Err(err)
            }
        }
    }

    async fn get_active_by_code(&self, code: &str) -> AppResult<SourceData> {
        let s = self.get_by_code(code).await?;
        if Status::Active.as_str() != s.status {
            return Err(AppError::bad_request(format!(
                "source `{code}` is not ACTIVE",
            )));
        }
        Ok(s)
    }

    async fn get_active_by_id(&self, id: i32) -> AppResult<SourceData> {
        let s = self.repo.find_by_id(id).await?;
        if Status::Active.as_str() != s.status {
            return Err(AppError::bad_request(format!(
                "source `{id}` is not ACTIVE",
            )));
        }
        Ok(s)
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
