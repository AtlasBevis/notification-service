use crate::infras::kafka::KafkaBus;
use crate::infras::postgres::PostgresManager;
use crate::services::{
    ChannelsService, NotificationService, NotifyService, SourcesService, TargetsService,
    TemplatesService,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub notification_service: Arc<NotificationService>,
    pub notify_service: Arc<NotifyService>,
    pub sources_service: Arc<SourcesService>,
    pub targets_service: Arc<TargetsService>,
    pub channels_service: Arc<ChannelsService>,
    pub templates_service: Arc<TemplatesService>,
    pub postgres: Arc<PostgresManager>,
    pub kafka: Option<Arc<KafkaBus>>,
}

pub type SharedState = Arc<AppState>;
