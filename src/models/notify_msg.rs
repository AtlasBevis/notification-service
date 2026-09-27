use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::models::{NotificationData, NotifyRequest, Recipients};

pub const EVENT_TYPE_SEND: &str = "notification.send";

/// Kafka / outbox payload. `Serialize` + `Deserialize` for JSON.
/// `Debug` for tracing. No `Clone`: pass `&NotifyMessage` or move it.
#[derive(Debug, Serialize, Deserialize)]
pub struct NotifyMessage {
    pub key: NotifyKey,
    pub value: NotifyValue,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NotifyKey {
    pub service: String,
    pub notification_code: String,
    pub trace_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NotifyValue {
    pub trace_id: String,
    pub code: String,
    pub variables: Map<String, Value>,

    pub notification_id: i64,
    pub source: String,
    pub target: String,
    pub channel: String,
    pub template: String,

    pub kind: String,
    pub title: Option<String>,
    pub message: Option<String>,
    pub payload: Value,
    pub recipients: Recipients,
}

impl NotifyMessage {
    pub fn new(req: &NotifyRequest) -> Self {
        Self {
            key: NotifyKey {
                service: req.service.clone(),
                notification_code: req.code.clone(),
                trace_id: req.trace_id.clone(),
            },
            value: NotifyValue {
                trace_id: req.trace_id.clone(),
                code: req.code.clone(),
                variables: req.variables.clone(),
                notification_id: 0,
                source: String::new(),
                target: String::new(),
                channel: String::new(),
                template: String::new(),
                kind: "INFO".to_string(),
                title: None,
                message: None,
                payload: Value::Object(req.variables.clone()),
                recipients: Recipients::default(),
            },
        }
    }

    pub fn with_notification(
        mut self,
        notification: &NotificationData,
        template: impl Into<String>,
        recipients: Recipients,
    ) -> Self {
        self.value.notification_id = notification.id;
        self.value.code = notification.code.clone();
        self.value.source = notification.source.clone();
        self.value.target = notification.target.clone();
        self.value.channel = notification.channel.clone();
        self.value.template = template.into();
        self.value.recipients = recipients;
        self
    }

    pub fn with_rendered(mut self, title: String, message: String) -> Self {
        self.value.title = Some(title);
        self.value.message = Some(message);
        self
    }
}
