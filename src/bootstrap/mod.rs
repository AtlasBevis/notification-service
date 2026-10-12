use anyhow::{Context, Result};
use std::sync::Arc;

use crate::config::Config;
use crate::infras::Infrastructure;
use crate::repository::{
    ChannelRepository, DefinitionRepository, DeliveriesRepository, NotificationsRepository,
    SourceRepository, TargetsRepository, TemplateRepository, WebhookRepository,
};
use crate::services::{
    ChannelService, ChannelTrait, DefinitionService, NotificationService, NotifyService,
    SourceService, SourceTrait, TargetTrait, TargetsService, TemplateService, WebhookService,
};
use crate::state::{AppState, SharedState};

/// HTTP API: load sources/targets/channels into memory, serve APIs.
pub async fn build_state(config: &Config) -> Result<SharedState> {
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
    let webhook_configs = WebhookRepository::new(pool);
    let webhook_service = Arc::new(WebhookService::new(
        reqwest::Client::builder()
            .build()
            .context("Failed to build webhook HTTP client")?,
    ));

    let channels_service = Arc::new(ChannelService::new(channels_repo.clone()));
    let sources_service = Arc::new(SourceService::new(sources_repo.clone()));
    let targets_service = Arc::new(TargetsService::new(targets_repo.clone()));
    let templates_service = Arc::new(TemplateService::new(templates_repo));

    channels_service
        .load()
        .await
        .context("Failed to load channels")?;
    sources_service
        .load()
        .await
        .context("Failed to load sources")?;
    targets_service
        .load()
        .await
        .context("Failed to load targets")?;

    let notification_service = Arc::new(NotificationService::new(
        notifications_repo,
        sources_service.clone(),
        targets_service.clone(),
        channels_service.clone(),
        templates_service.clone(),
    ));
    let notify_service = Arc::new(NotifyService::new(
        infras.kafka.clone(),
        infras.http_client.clone(),
        definitions_service,
        notification_service.clone(),
        deliveries_repo,
        webhook_configs,
        webhook_service,
        sources_service.clone(),
        targets_service.clone(),
        channels_service.clone(),
        templates_service.clone(),
    ));

    Ok(Arc::new(AppState {
        notification_service,
        notify_service,
        sources_service,
        targets_service,
        channels_service,
        templates_service,
        postgres: infras.postgres,
        kafka: infras.kafka,
    }))
}

/// Worker process `notification-consumer`.
pub async fn build_notification_consumer(config: &Config) -> Result<Arc<NotifyService>> {
    let infras = Infrastructure::new_require_kafka(config).await?;
    let kafka = infras
        .kafka
        .clone()
        .context("Kafka is required for notification-consumer")?;
    let pool = infras.postgres.pool();

    let sources_service = Arc::new(SourceService::new(SourceRepository::new(pool.clone())));
    let targets_service = Arc::new(TargetsService::new(TargetsRepository::new(pool.clone())));
    let channels_service = Arc::new(ChannelService::new(ChannelRepository::new(pool.clone())));
    let templates_service = Arc::new(TemplateService::new(TemplateRepository::new(pool.clone())));
    channels_service
        .load()
        .await
        .context("Failed to load channels")?;
    sources_service
        .load()
        .await
        .context("Failed to load sources")?;
    targets_service
        .load()
        .await
        .context("Failed to load targets")?;

    let notification_service = Arc::new(NotificationService::new(
        NotificationsRepository::new(pool.clone()),
        sources_service.clone(),
        targets_service.clone(),
        channels_service.clone(),
        templates_service.clone(),
    ));
    let webhook_service = Arc::new(WebhookService::new(
        reqwest::Client::builder()
            .build()
            .context("Failed to build webhook HTTP client")?,
    ));
    Ok(Arc::new(NotifyService::new(
        Some(kafka),
        infras.http_client,
        Arc::new(DefinitionService::new(DefinitionRepository::new(
            pool.clone(),
        ))),
        notification_service,
        DeliveriesRepository::new(pool.clone()),
        WebhookRepository::new(pool),
        webhook_service,
        sources_service,
        targets_service,
        channels_service,
        templates_service,
    )))
}
