use anyhow::{Context, Result};
use std::sync::Arc;

use crate::config::Config;
use crate::infras::Infrastructure;
use crate::repository::{
    ChannelsRepository, DeliveriesRepository, NotificationsRepository, OutboxRepository,
    RequestsRepository, SourcesRepository, TargetsRepository, TemplatesRepository,
};
use crate::services::{
    ChannelsService, ChannelsServiceTrait, NotificationService, NotifyService, OutboxService,
    SourcesService, SourcesServiceTrait, TargetsService, TargetsServiceTrait, TemplatesService,
};
use crate::state::{AppState, SharedState};

/// HTTP API: load sources/targets/channels into memory, serve APIs.
pub async fn build_state(config: &Config) -> Result<SharedState> {
    let infras = Infrastructure::new(config).await?;
    let pool = infras.postgres.pool();

    let channels_repo = ChannelsRepository::new(pool.clone());
    let sources_repo = SourcesRepository::new(pool.clone());
    let targets_repo = TargetsRepository::new(pool.clone());
    let templates_repo = TemplatesRepository::new(pool.clone());
    let notifications_repo = NotificationsRepository::new(pool.clone());
    let requests_repo = RequestsRepository::new(pool.clone());
    let deliveries_repo = DeliveriesRepository::new(pool.clone());
    let outbox_repo = OutboxRepository::new(pool);

    let channels_service = Arc::new(ChannelsService::new(channels_repo.clone()));
    let sources_service = Arc::new(SourcesService::new(sources_repo.clone()));
    let targets_service = Arc::new(TargetsService::new(targets_repo.clone()));
    let templates_service = Arc::new(TemplatesService::new(templates_repo));

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
        notification_service.clone(),
        requests_repo,
        deliveries_repo,
        outbox_repo.clone(),
        sources_service.clone(),
        targets_service.clone(),
        channels_service.clone(),
        templates_service.clone(),
    ));

    if let Some(kafka) = infras.kafka.clone() {
        let publisher = Arc::new(OutboxService::new(outbox_repo, kafka));
        tokio::spawn(async move {
            publisher.run_publisher().await;
        });
    }

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

    let sources_service = Arc::new(SourcesService::new(SourcesRepository::new(pool.clone())));
    let targets_service = Arc::new(TargetsService::new(TargetsRepository::new(pool.clone())));
    let channels_service = Arc::new(ChannelsService::new(ChannelsRepository::new(pool.clone())));
    let templates_service = Arc::new(TemplatesService::new(TemplatesRepository::new(
        pool.clone(),
    )));
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
    Ok(Arc::new(NotifyService::new(
        Some(kafka),
        infras.http_client,
        notification_service,
        RequestsRepository::new(pool.clone()),
        DeliveriesRepository::new(pool.clone()),
        OutboxRepository::new(pool),
        sources_service,
        targets_service,
        channels_service,
        templates_service,
    )))
}
