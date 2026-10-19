use async_trait::async_trait;
use std::sync::Arc;

use crate::enums::Status;
use crate::error::{AppError, AppResult};
use crate::models::{PageData, PageRequest, TargetData, TargetRequest};
use crate::repository::TargetsRepository;
use crate::services::ChannelTrait;

/// Targets are always read from Postgres; they are not cached in memory.
#[async_trait]
pub trait TargetTrait: Send + Sync {
    async fn list(&self, page: PageRequest) -> AppResult<PageData<TargetData>>;
    async fn get_by_code(&self, code: &str) -> AppResult<TargetData>;
    async fn get_active_by_id(&self, id: i32) -> AppResult<TargetData>;
    async fn create(&self, req: TargetRequest) -> AppResult<()>;
    async fn update(&self, req: TargetRequest) -> AppResult<()>;
}

#[derive(Clone)]
pub struct TargetService {
    repo: TargetsRepository,
    channels: Arc<dyn ChannelTrait>,
}

impl TargetService {
    pub fn new(repo: TargetsRepository, channels: Arc<dyn ChannelTrait>) -> Self {
        Self { repo, channels }
    }
}

#[async_trait]
impl TargetTrait for TargetService {
    async fn list(&self, page: PageRequest) -> AppResult<PageData<TargetData>> {
        self.repo.list_page(&page).await
    }

    async fn get_by_code(&self, code: &str) -> AppResult<TargetData> {
        self.repo.find_by_code(code).await
    }

    async fn get_active_by_id(&self, id: i32) -> AppResult<TargetData> {
        let t = self.repo.find_by_id(id).await?;
        if Status::Active != t.status {
            return Err(AppError::bad_request(format!("target {id} is not ACTIVE")));
        }
        Ok(t)
    }

    async fn create(&self, req: TargetRequest) -> AppResult<()> {
        let channel = match self.channels.get_by_code(req.channel).await {
            Ok(channel) => channel,
            Err(AppError::NotFound(_)) => {
                return Err(AppError::bad_request(format!(
                    "unknown channel `{}`",
                    req.channel
                )))
            }
            Err(err) => return Err(err),
        };

        match self.repo.insert(&req, channel.id).await {
            Ok(_) => Ok(()),
            Err(AppError::Conflict(_)) => Err(AppError::conflict(format!(
                "target `{}` already exists",
                req.code
            ))),
            Err(err) => {
                tracing::error!(error = %err, code = %req.code, "insert target failed");
                Err(err)
            }
        }
    }

    async fn update(&self, req: TargetRequest) -> AppResult<()> {
        match self.repo.update(&req).await {
            Ok(()) => Ok(()),
            Err(AppError::NotFound(_)) => Err(AppError::not_found(format!(
                "target `{}` not found",
                req.code
            ))),
            Err(err) => {
                tracing::error!(error = %err, code = %req.code, "update target failed");
                Err(err)
            }
        }
    }
}
