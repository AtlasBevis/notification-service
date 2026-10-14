use crate::api::{
    DefinitionController, HealthController, NotifyController, SourceController, TargetController,
    TemplateController, WebhookController,
};
use crate::infras::kafka::KafkaBus;
use crate::infras::postgres::PostgresManager;
use crate::services::{ChannelTrait, ConsumerTrait, NotifyTrait};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub notify_service: Arc<dyn NotifyTrait>,
    pub consumer_service: Option<Arc<dyn ConsumerTrait>>,
    pub channels_service: Arc<dyn ChannelTrait>,
    pub health_controller: Arc<HealthController>,
    pub notify_controller: Arc<NotifyController>,
    pub source_controller: Arc<SourceController>,
    pub target_controller: Arc<TargetController>,
    pub template_controller: Arc<TemplateController>,
    pub definition_controller: Arc<DefinitionController>,
    pub webhook_controller: Arc<WebhookController>,
    pub postgres: Arc<PostgresManager>,
    pub kafka: Option<Arc<KafkaBus>>,
}

pub type SharedState = Arc<AppState>;
