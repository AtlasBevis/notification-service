use crate::error::AppResult;
use crate::models::{AcceptedNotification, DeliveryDraft};
use crate::repository::NotificationsRepository;
use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
pub trait NotificationTrait: Send + Sync {
    async fn accept(
        &self,
        definition_id: i64,
        source_code: &str,
        code: &str,
        trace_id: &str,
        variables: &Value,
        drafts: &[DeliveryDraft],
    ) -> AppResult<AcceptedNotification>;
}

/// Persists a notification accepted by the `/notification/noti/notify` flow.
#[derive(Clone)]
pub struct NotificationService {
    notifications: NotificationsRepository,
}

impl NotificationService {
    pub fn new(notifications: NotificationsRepository) -> Self {
        Self { notifications }
    }
}

#[async_trait]
impl NotificationTrait for NotificationService {
    async fn accept(
        &self,
        definition_id: i64,
        source_code: &str,
        code: &str,
        trace_id: &str,
        variables: &Value,
        drafts: &[DeliveryDraft],
    ) -> AppResult<AcceptedNotification> {
        self.notifications
            .accept(
                definition_id,
                source_code,
                code,
                trace_id,
                variables,
                drafts,
            )
            .await
    }
}
