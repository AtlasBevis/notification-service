use crate::error::{AppError, AppResult};
use crate::models::{CreateNotificationRequest, NotificationData, UpdateNotificationRequest};
use crate::repository::NotificationsRepository;
use crate::services::{
    ChannelsService, ChannelsServiceTrait, SourcesService, SourcesServiceTrait, TargetsService,
    TargetsServiceTrait, TemplatesService, TemplatesServiceTrait,
};
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait NotificationServiceTrait: Send + Sync {
    async fn create(&self, req: CreateNotificationRequest) -> AppResult<NotificationData>;
    async fn list(&self) -> AppResult<Vec<NotificationData>>;
    async fn get_by_code(&self, code: &str) -> AppResult<NotificationData>;
    async fn update(
        &self,
        code: &str,
        req: UpdateNotificationRequest,
    ) -> AppResult<NotificationData>;
    async fn delete(&self, code: &str) -> AppResult<()>;
}

#[derive(Clone)]
pub struct NotificationService {
    notifications: NotificationsRepository,
    sources: Arc<SourcesService>,
    targets: Arc<TargetsService>,
    channels: Arc<ChannelsService>,
    templates: Arc<TemplatesService>,
}

impl NotificationService {
    pub fn new(
        notifications: NotificationsRepository,
        sources: Arc<SourcesService>,
        targets: Arc<TargetsService>,
        channels: Arc<ChannelsService>,
        templates: Arc<TemplatesService>,
    ) -> Self {
        Self {
            notifications,
            sources,
            targets,
            channels,
            templates,
        }
    }
}

#[async_trait]
impl NotificationServiceTrait for NotificationService {
    async fn create(&self, req: CreateNotificationRequest) -> AppResult<NotificationData> {
        self.sources
            .must_active(&req.source)
            .await
            .map_err(AppError::bad_request)?;
        self.targets
            .must_active(&req.target)
            .await
            .map_err(AppError::bad_request)?;
        self.channels
            .must_active(&req.channel)
            .await
            .map_err(AppError::bad_request)?;
        self.templates
            .must_active_by_id(req.template_id)
            .await
            .map_err(AppError::bad_request)?;

        match self.notifications.insert(&req).await {
            Ok(row) => Ok(row),
            Err(AppError::Conflict(_)) => Err(AppError::conflict(format!(
                "notification `{}` already exists",
                req.code
            ))),
            Err(err) => {
                tracing::error!(error = %err, code = %req.code, "insert notification failed");
                Err(err)
            }
        }
    }

    async fn list(&self) -> AppResult<Vec<NotificationData>> {
        self.notifications.list().await.map_err(|err| {
            tracing::error!(error = %err, "list notifications failed");
            err
        })
    }

    async fn get_by_code(&self, code: &str) -> AppResult<NotificationData> {
        match self.notifications.find_by_code(code).await {
            Ok(noti) => Ok(noti),
            Err(err) => {
                tracing::error!(error = %err, code, "notification get_by_code err");
                Err(err)
            }
        }
    }

    async fn update(
        &self,
        code: &str,
        req: UpdateNotificationRequest,
    ) -> AppResult<NotificationData> {
        self.sources
            .must_active(&req.source)
            .await
            .map_err(AppError::bad_request)?;
        self.targets
            .must_active(&req.target)
            .await
            .map_err(AppError::bad_request)?;
        self.channels
            .must_active(&req.channel)
            .await
            .map_err(AppError::bad_request)?;
        self.templates
            .must_active_by_id(req.template_id)
            .await
            .map_err(AppError::bad_request)?;

        match self.notifications.update(code, &req).await {
            Ok(row) => Ok(row),
            Err(AppError::NotFound(_)) => Err(AppError::not_found(format!(
                "notification `{code}` not found"
            ))),
            Err(err) => {
                tracing::error!(error = %err, code, "update notification failed");
                Err(err)
            }
        }
    }

    async fn delete(&self, code: &str) -> AppResult<()> {
        match self.notifications.delete(code).await {
            Ok(()) => Ok(()),
            Err(AppError::NotFound(_)) => Err(AppError::not_found(format!(
                "notification `{code}` not found"
            ))),
            Err(AppError::Conflict(msg)) => Err(AppError::conflict(msg)),
            Err(err) => {
                tracing::error!(error = %err, code, "delete notification failed");
                Err(err)
            }
        }
    }
}
