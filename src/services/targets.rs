use async_trait::async_trait;

use crate::enums::Status;
use crate::error::{AppError, AppResult};
use crate::models::{CreateTargetRequest, PatchMetadataRequest, TargetData};
use crate::repository::TargetsRepository;

/// Targets are always read from Postgres; they are not cached in memory.
#[async_trait]
pub trait TargetTrait: Send + Sync {
    async fn get_by_code(&self, code: &str) -> AppResult<TargetData>;
    async fn get_active_by_id(&self, id: i32) -> AppResult<TargetData>;
    async fn create(&self, req: CreateTargetRequest) -> AppResult<()>;
    async fn patch_metadata(&self, req: PatchMetadataRequest) -> AppResult<()>;
}

#[derive(Clone)]
pub struct TargetsService {
    repo: TargetsRepository,
}

impl TargetsService {
    pub fn new(repo: TargetsRepository) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl TargetTrait for TargetsService {
    async fn get_by_code(&self, code: &str) -> AppResult<TargetData> {
        let key = code.trim().to_ascii_uppercase();
        self.repo.find_by_code(&key).await
    }

    async fn get_active_by_id(&self, id: i32) -> AppResult<TargetData> {
        let t = self.repo.find_by_id(id).await?;
        active_target(t, &format!("id `{id}`"))
    }

    async fn create(&self, req: CreateTargetRequest) -> AppResult<()> {
        match self.repo.insert(&req).await {
            Ok(_) => Ok(()),
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
            Ok(()) => Ok(()),
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
