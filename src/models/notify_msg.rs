use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::models::{DefinitionData, NotifyRequest, Recipients};

pub const EVENT_TYPE_SEND: &str = "notification.send";

/// Kafka payload. `Serialize` + `Deserialize` for JSON.
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
    pub definition_id: i64,
    pub delivery_id: i64,
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
                service: String::new(),
                notification_code: req.code.clone(),
                trace_id: req.trace_id.clone(),
            },
            value: NotifyValue {
                trace_id: req.trace_id.clone(),
                code: req.code.clone(),
                variables: req.variables.clone(),
                notification_id: 0,
                definition_id: 0,
                delivery_id: 0,
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

    pub fn with_definition(
        mut self,
        definition: &DefinitionData,
        source_code: impl Into<String>,
        channel_code: impl Into<String>,
        target_code: impl Into<String>,
        template: impl Into<String>,
        recipients: Recipients,
    ) -> Self {
        let source_code = source_code.into();
        self.key.service = source_code.clone();
        self.key.notification_code = definition.code.clone();
        self.value.definition_id = definition.id;
        self.value.code = definition.code.clone();
        self.value.source = source_code;
        self.value.target = target_code.into();
        self.value.channel = channel_code.into();
        self.value.kind = definition.level.clone();
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
