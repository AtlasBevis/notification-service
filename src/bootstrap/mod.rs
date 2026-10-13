use crate::error::{AppError, AppResult};
use std::sync::Arc;

use crate::config::Config;
use crate::infras::Infrastructure;
use crate::repository::{
    ChannelRepository, DefinitionRepository, DeliveriesRepository, NotificationsRepository,
    SourceRepository, TargetsRepository, TemplateRepository, WebhookRepository,
};
use crate::services::{
    ChannelService, ChannelTrait, DefinitionService, DeliveryService, EmailService, MSService,
    NotificationService, NotifyService, SourceService, SourceTrait, TargetTrait, TargetsService,
    TemplateService, WebhookService,
};
use crate::state::{AppState, SharedState};

/// HTTP API: load sources/targets/channels into memory, serve APIs.
pub async fn build_state(config: &Config) -> AppResult<SharedState> {
    let infras = Infrastructure::new(config).await?;
    let pool = infras.postgres.pool();

    let channels_repo = ChannelRepository::new(pool.clone());
    let sources_repo = SourceRepository::new(pool.clone());
    let targets_repo = TargetsRepository::new(pool.clone());
    let templates_repo = TemplateRepository::new(pool.clone());
    let notifications_repo = NotificationsRepository::new(pool.clone());
    let deliveries_repo = DeliveriesRepository::new(pool.clone());
    let definitions_service = Arc::new(DefinitionService::new(DefinitionRepository::new(
        pool.clone(),
    )));

    let channels_service = Arc::new(ChannelService::new(channels_repo.clone()));
    let sources_service = Arc::new(SourceService::new(sources_repo.clone()));
    let targets_service = Arc::new(TargetsService::new(targets_repo.clone()));
    let templates_service = Arc::new(TemplateService::new(templates_repo));
    let webhook_service = Arc::new(WebhookService::new(
        infras.http_client.clone(),
        WebhookRepository::new(pool),
    ));
    let email_service = Arc::new(EmailService::new(channels_service.clone()));
    let teams_service = Arc::new(MSService::new(
        infras.http_client.clone(),
        channels_service.clone(),
        targets_service.clone(),
    ));

    channels_service
        .load()
        .await
        .map_err(|e| AppError::internal_with("Failed to load channels", e))?;
    sources_service
        .load()
        .await
        .map_err(|e| AppError::internal_with("Failed to load sources", e))?;
    targets_service
        .load()
        .await
        .map_err(|e| AppError::internal_with("Failed to load targets", e))?;

    let notification_service = Arc::new(NotificationService::new(notifications_repo));
    let delivery_service = Arc::new(DeliveryService::new(deliveries_repo));
    let notify_service = Arc::new(NotifyService::new(
        infras.kafka.clone(),
        definitions_service,
        notification_service.clone(),
        delivery_service,
        sources_service.clone(),
        targets_service.clone(),
        channels_service.clone(),
        templates_service.clone(),
        email_service,
        teams_service,
        webhook_service,
    ));

    Ok(Arc::new(AppState {
        notify_service,
        sources_service,
        targets_service,
        channels_service,
        templates_service,
        postgres: infras.postgres,
        kafka: infras.kafka,
    }))
}

/// Worker process `notification-job`.
pub async fn build_notification_job(config: &Config) -> AppResult<Arc<NotifyService>> {
    let infras = Infrastructure::new_require_kafka(config).await?;
    let kafka = infras
        .kafka
        .clone()
        .ok_or_else(|| AppError::internal("Kafka is required for notification-job"))?;
    let pool = infras.postgres.pool();

    let sources_service = Arc::new(SourceService::new(SourceRepository::new(pool.clone())));
    let targets_service = Arc::new(TargetsService::new(TargetsRepository::new(pool.clone())));
    let channels_service = Arc::new(ChannelService::new(ChannelRepository::new(pool.clone())));
    let templates_service = Arc::new(TemplateService::new(TemplateRepository::new(pool.clone())));
    channels_service
        .load()
        .await
        .map_err(|e| AppError::internal_with("Failed to load channels", e))?;
    sources_service
        .load()
        .await
        .map_err(|e| AppError::internal_with("Failed to load sources", e))?;
    targets_service
        .load()
        .await
        .map_err(|e| AppError::internal_with("Failed to load targets", e))?;

    let notification_service = Arc::new(NotificationService::new(NotificationsRepository::new(
        pool.clone(),
    )));
    let delivery_service = Arc::new(DeliveryService::new(DeliveriesRepository::new(
        pool.clone(),
    )));
    let webhook_service = Arc::new(WebhookService::new(
        infras.http_client.clone(),
        WebhookRepository::new(pool.clone()),
    ));
    let email_service = Arc::new(EmailService::new(channels_service.clone()));
    let teams_service = Arc::new(MSService::new(
        infras.http_client,
        channels_service.clone(),
        targets_service.clone(),
    ));
    Ok(Arc::new(NotifyService::new(
        Some(kafka),
        Arc::new(DefinitionService::new(DefinitionRepository::new(
            pool.clone(),
        ))),
        notification_service,
        delivery_service,
        sources_service,
        targets_service,
        channels_service,
        templates_service,
        email_service,
        teams_service,
        webhook_service,
    )))
}
