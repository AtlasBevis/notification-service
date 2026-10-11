use crate::error::{AppError, AppResult};
use crate::models::{
    AcceptedNotification, CreateNotificationRequest, DeliveryDraft, NotificationData,
    UpdateNotificationRequest,
};
use crate::repository::NotificationsRepository;
use crate::services::{
    ChannelsService, ChannelsServiceTrait, SourceService, SourcesServiceTrait, TargetsService,
    TargetsServiceTrait, TemplatesService, TemplatesServiceTrait,
};
use async_trait::async_trait;
use serde_json::Value;
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
    sources: Arc<SourceService>,
    targets: Arc<TargetsService>,
    channels: Arc<ChannelsService>,
    templates: Arc<TemplatesService>,
}

impl NotificationService {
    pub fn new(
        notifications: NotificationsRepository,
        sources: Arc<SourceService>,
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

    pub async fn accept(
        &self,
        definition_id: i64,
        source_code: &str,
        code: &str,
        idempotency_key: &str,
        variables: &Value,
        drafts: &[DeliveryDraft],
    ) -> AppResult<AcceptedNotification> {
        self.notifications
            .accept(
                definition_id,
                source_code,
                code,
                idempotency_key,
                variables,
                drafts,
            )
            .await
    }

    async fn ensure_destination(
        &self,
        source_code: &str,
        channel_code: &str,
        target_code: &str,
        template_id: i64,
    ) -> AppResult<()> {
        self.sources
            .get_active_by_id(source_code)
            .await
            .map_err(AppError::bad_request)?;
        self.channels
            .must_active(channel_code)
            .await
            .map_err(AppError::bad_request)?;
        let target = self
            .targets
            .must_active(target_code)
            .await
            .map_err(AppError::bad_request)?;
        if !target.channel_code.eq_ignore_ascii_case(channel_code) {
            return Err(AppError::bad_request(format!(
                "target `{target_code}` belongs to channel `{}`",
                target.channel_code
            )));
        }
        self.templates
            .must_active_by_id(template_id)
            .await
            .map_err(AppError::bad_request)?;
        Ok(())
    }
}

#[async_trait]
impl NotificationServiceTrait for NotificationService {
    async fn create(&self, req: CreateNotificationRequest) -> AppResult<NotificationData> {
        self.ensure_destination(
            &req.source_code,
            &req.channel_code,
            &req.target_code,
            req.template_id,
        )
        .await?;

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
        self.ensure_destination(
            &req.source_code,
            &req.channel_code,
            &req.target_code,
            req.template_id,
        )
        .await?;

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
