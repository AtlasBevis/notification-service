use crate::enums::Status;
use crate::error::{AppError, AppResult};
use crate::models::DefinitionData;
use crate::repository::DefinitionRepository;
use async_trait::async_trait;

#[async_trait]
pub trait DefinitionTrait: Send + Sync {
    // async fn create(&self, req: CreateNotificationRequest) -> AppResult<NotificationData>;
    // async fn list(&self) -> AppResult<Vec<NotificationData>>;
    async fn get_by_code(&self, code: &str) -> AppResult<DefinitionData>;
    async fn get_active_by_code(&self, code: &str) -> AppResult<DefinitionData>;
    // async fn update(
    //     &self,
    //     code: &str,
    //     req: UpdateNotificationRequest,
    // ) -> AppResult<NotificationData>;
    // async fn delete(&self, code: &str) -> AppResult<()>;
}

#[derive(Clone)]
pub struct DefinitionService {
    repo: DefinitionRepository,
}

impl DefinitionService {
    pub fn new(repo: DefinitionRepository) -> Self {
        Self { repo: repo }
    }
}

#[async_trait]
impl DefinitionTrait for DefinitionService {
    async fn get_by_code(&self, code: &str) -> AppResult<DefinitionData> {
        match self.repo.find_by_code(code).await {
            Ok(d) => Ok(d),
            Err(err) => {
                tracing::error!(error = %err, code, "definition get_by_code err");
                Err(err)
            }
        }
    }

    async fn get_active_by_code(&self, code: &str) -> AppResult<DefinitionData> {
        let d = self.get_by_code(code).await?;
        if Status::Active.as_str() != d.status {
            return Err(AppError::bad_request(format!(
                "definition `{code}` is not ACTIVE",
            )));
        }
        Ok(d)
    }
}
