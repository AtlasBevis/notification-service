use crate::api::{
    DefinitionController, HealthController, NotifyController, SourceController, TargetController,
    TemplateController, WebhookController,
};
use crate::grpc::{DefinitionGrpc, NotifyGrpc, SourceGrpc, TargetGrpc, TemplateGrpc, WebhookGrpc};
use crate::infras::kafka::KafkaBus;
use crate::infras::postgres::PostgresManager;
use crate::services::{ChannelTrait, ConsumerTrait, NotifyTrait};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    // services
    pub notify_service: Arc<dyn NotifyTrait>,
    pub consumer_service: Option<Arc<dyn ConsumerTrait>>,
    pub channels_service: Arc<dyn ChannelTrait>,

    // controllers
    pub health_controller: Arc<HealthController>,
    pub notify_controller: Arc<NotifyController>,
    pub source_controller: Arc<SourceController>,
    pub target_controller: Arc<TargetController>,
    pub template_controller: Arc<TemplateController>,
    pub definition_controller: Arc<DefinitionController>,
    pub webhook_controller: Arc<WebhookController>,

    // grpc
    pub notify_grpc: Arc<NotifyGrpc>,
    pub source_grpc: Arc<SourceGrpc>,
    pub target_grpc: Arc<TargetGrpc>,
    pub template_grpc: Arc<TemplateGrpc>,
    pub definition_grpc: Arc<DefinitionGrpc>,
    pub webhook_grpc: Arc<WebhookGrpc>,

    // infas
    pub postgres: Arc<PostgresManager>,
    pub kafka: Option<Arc<KafkaBus>>,
}

pub type SharedState = Arc<AppState>;
