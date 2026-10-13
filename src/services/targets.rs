use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::enums::Status;
use crate::error::{AppError, AppResult};
use crate::models::{CreateTargetRequest, PatchMetadataRequest, TargetData};
use crate::repository::TargetsRepository;

#[async_trait]
pub trait TargetTrait: Send + Sync {
    async fn load(&self) -> AppResult<()>;
    async fn get(&self, code: &str) -> Option<TargetData>;
    async fn get_by_code(&self, code: &str) -> AppResult<TargetData>;
    async fn get_active_by_id(&self, id: i32) -> AppResult<TargetData>;
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
impl TargetTrait for TargetsService {
    async fn load(&self) -> AppResult<()> {
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

    async fn get_by_code(&self, code: &str) -> AppResult<TargetData> {
        if let Some(t) = self.get(code).await {
            return Ok(t);
        }
        let t = self.repo.find_by_code(code).await?;
        self.cache.write().await.insert(t.code.clone(), t.clone());
        Ok(t)
    }

    async fn get_active_by_id(&self, id: i32) -> AppResult<TargetData> {
        let t = self.repo.find_by_id(id).await?;
        self.cache.write().await.insert(t.code.clone(), t.clone());
        active_target(t, &format!("id `{id}`"))
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

fn active_target(t: TargetData, label: &str) -> AppResult<TargetData> {
    if t.status == Status::Active {
        Ok(t)
    } else {
        Err(AppError::bad_request(format!(
            "target {label} is not ACTIVE"
        )))
    }
}
