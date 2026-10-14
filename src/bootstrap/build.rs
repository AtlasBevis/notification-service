use crate::api::{
    DefinitionController, HealthController, NotifyController, SourceController, TargetController,
    TemplateController, WebhookController,
};
use crate::error::AppResult;
use std::sync::Arc;

use crate::config::Config;
use crate::infras::Infrastructure;
use crate::repository::{
    ChannelRepository, DefinitionRepository, DeliveriesRepository, NotificationsRepository,
    SourceRepository, TargetsRepository, TemplateRepository, WebhookRepository,
};
use crate::services::{
    ChannelService, ConsumerService, ConsumerTrait, DefinitionService, DeliveryService,
    EmailService, MSService, NotificationService, NotifyService, SourceService, TargetsService,
    TemplateService, WebhookService,
};
use crate::state::{AppState, SharedState};

use super::load::load_memory;

/// Wire repositories and services into the shared `AppState`, used by both the API and `notification-job`.
pub async fn build_state(config: &Config) -> AppResult<SharedState> {
    let infras = Infrastructure::new(config).await?;
    let pool = infras.postgres.pool();

    // repository
    let channels_repo = ChannelRepository::new(pool.clone());
    let sources_repo = SourceRepository::new(pool.clone());
    let targets_repo = TargetsRepository::new(pool.clone());
    let templates_repo = TemplateRepository::new(pool.clone());
    let notifications_repo = NotificationsRepository::new(pool.clone());
    let deliveries_repo = DeliveriesRepository::new(pool.clone());
    let definitions_repo = DefinitionRepository::new(pool.clone());

    // services
    let channels_service = Arc::new(ChannelService::new(channels_repo.clone()));
    let sources_service = Arc::new(SourceService::new(sources_repo.clone()));
    let targets_service = Arc::new(TargetsService::new(targets_repo.clone()));
    let templates_service = Arc::new(TemplateService::new(templates_repo));
    let definitions_service = Arc::new(DefinitionService::new(
        definitions_repo,
        sources_service.clone(),
        channels_service.clone(),
        targets_service.clone(),
        templates_service.clone(),
    ));
    let webhook_service = Arc::new(WebhookService::new(
        infras.http_client.clone(),
        WebhookRepository::new(pool),
        targets_service.clone(),
    ));
    let email_service = Arc::new(EmailService::new(channels_service.clone()));
    let teams_service = Arc::new(MSService::new(
        infras.http_client.clone(),
        channels_service.clone(),
        targets_service.clone(),
    ));

    load_memory(channels_service.as_ref(), sources_service.as_ref()).await?;

    let notification_service = Arc::new(NotificationService::new(notifications_repo));
    let delivery_service = Arc::new(DeliveryService::new(deliveries_repo));
    let notify_service = Arc::new(NotifyService::new(
        infras.kafka.clone(),
        definitions_service.clone(),
        notification_service.clone(),
        delivery_service.clone(),
        sources_service.clone(),
        targets_service.clone(),
        channels_service.clone(),
        templates_service.clone(),
        webhook_service.clone(),
    ));
    let consumer_service = infras.kafka.clone().map(|kafka| {
        Arc::new(ConsumerService::new(
            kafka,
            delivery_service,
            email_service,
            teams_service,
            webhook_service.clone(),
        )) as Arc<dyn ConsumerTrait>
    });

    // controllers
    let webhook_controller = Arc::new(WebhookController::new(webhook_service.clone()));
    let health_controller = Arc::new(HealthController::new(
        infras.postgres.clone(),
        infras.kafka.clone(),
    ));
    let notify_controller = Arc::new(NotifyController::new(notify_service.clone()));
    let source_controller = Arc::new(SourceController::new(sources_service));
    let target_controller = Arc::new(TargetController::new(targets_service));
    let template_controller = Arc::new(TemplateController::new(templates_service));
    let definition_controller = Arc::new(DefinitionController::new(definitions_service));

    Ok(Arc::new(AppState {
        notify_service,
        consumer_service,
        channels_service,
        health_controller,
        notify_controller,
        source_controller,
        target_controller,
        template_controller,
        definition_controller,
        webhook_controller,
        postgres: infras.postgres,
        kafka: infras.kafka,
    }))
}
