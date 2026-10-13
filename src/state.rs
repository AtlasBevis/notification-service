use crate::infras::kafka::KafkaBus;
use crate::infras::postgres::PostgresManager;
use crate::services::{
    ChannelTrait, ConsumerTrait, NotifyTrait, SourceTrait, TargetTrait, TemplateTrait,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    /// API: validate, record and publish.
    pub notify_service: Arc<dyn NotifyTrait>,
    /// `notification-job`: consume and dispatch. `None` when Kafka is not connected.
    pub consumer_service: Option<Arc<dyn ConsumerTrait>>,
    pub sources_service: Arc<dyn SourceTrait>,
    pub targets_service: Arc<dyn TargetTrait>,
    pub channels_service: Arc<dyn ChannelTrait>,
    pub templates_service: Arc<dyn TemplateTrait>,
    pub postgres: Arc<PostgresManager>,
    pub kafka: Option<Arc<KafkaBus>>,
}

pub type SharedState = Arc<AppState>;
