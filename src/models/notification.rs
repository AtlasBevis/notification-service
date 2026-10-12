use serde_json::Value;
use uuid::Uuid;

/// Runtime row in `notifications`, created when a caller triggers a definition.
#[derive(Debug, Clone)]
pub struct AcceptedNotification {
    pub id: i64,
    pub delivery_id: i64,
}

#[derive(Debug, Clone)]
pub struct DeliveryDraft {
    pub target_code: String,
    pub channel_code: String,
    pub title: String,
    pub body: String,
    pub payload: Value,
    pub destination: Value,
}
