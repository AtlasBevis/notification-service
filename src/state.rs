use crate::infras::kafka::KafkaBus;
use crate::infras::postgres::PostgresManager;
use crate::services::{
    ChannelService, NotificationService, NotifyService, SourceService, TargetsService,
    TemplateService,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub notification_service: Arc<NotificationService>,
    pub notify_service: Arc<NotifyService>,
    pub sources_service: Arc<SourceService>,
    pub targets_service: Arc<TargetsService>,
    pub channels_service: Arc<ChannelService>,
    pub templates_service: Arc<TemplateService>,
    pub postgres: Arc<PostgresManager>,
    pub kafka: Option<Arc<KafkaBus>>,
}

pub type SharedState = Arc<AppState>;
