use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{
    enums::{Channel, Level},
    models::{ChannelData, DefinitionData, NotifyRequest, Recipients},
};

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
    pub source: String,
    pub notification_id: i64,
    pub code: String,
    pub trace_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NotifyValue {
    pub source: String,
    pub trace_id: String,
    pub code: String,
    pub variables: Map<String, Value>,
    pub definition_id: i64,
    pub channel_id: i32,
    pub channel: Channel,
    pub notification_id: i64,
    pub level: Level,

    pub delivery_id: i64,
    pub target: String,
    pub target_ids: Vec<i32>,
    #[serde(default)]
    pub target_payloads: Vec<TargetPayload>,
    pub template: String,

    pub title: Option<String>,
    pub message: Option<String>,
    pub recipients: Recipients,
}

/// Webhook body template resolved for one target when the send was accepted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetPayload {
    pub target_id: i32,
    pub payload: Value,
}

impl NotifyMessage {
    pub fn with_request(req: &NotifyRequest) -> Self {
        Self {
            key: NotifyKey {
                source: req.source.clone(),
                notification_id: 0,
                code: req.code.clone(),
                trace_id: req.trace_id.clone(),
            },
            value: NotifyValue {
                source: req.source.clone(),
                trace_id: req.trace_id.clone(),
                code: req.code.clone(),
                variables: req.variables.clone(),
                notification_id: 0,
                definition_id: 0,
                channel_id: 0,
                delivery_id: 0,
                target: String::new(),
                target_ids: Vec::new(),
                target_payloads: Vec::new(),
                channel: Channel::EMAIL,
                level: Level::Info,
                template: String::new(),
                title: None,
                message: None,
                recipients: Recipients::default(),
            },
        }
    }

    pub fn with_definition(mut self, definition: &DefinitionData) -> Result<Self, String> {
        self.value.definition_id = definition.id;
        self.value.level = definition.level;
        self.value.code = definition.code.clone();
        self.key.code = definition.code.clone();
        Ok(self)
    }

    pub fn with_channel(mut self, channel: &ChannelData) -> Result<Self, String> {
        self.value.channel = channel.code;
        self.value.channel_id = channel.id;
        Ok(self)
    }

    pub fn with_targets(mut self, targets: &[crate::models::TargetData]) -> Self {
        self.value.target = targets
            .iter()
            .map(|target| target.code.as_str())
            .collect::<Vec<_>>()
            .join(",");
        self.value.target_ids = targets.iter().map(|target| target.id).collect();
        self
    }

    pub fn with_target_payloads(mut self, payloads: Vec<TargetPayload>) -> Self {
        self.value.target_payloads = payloads;
        self
    }

    pub fn with_template(mut self, template: impl Into<String>) -> Self {
        self.value.template = template.into();
        self
    }

    pub fn with_recipients(mut self, recipients: Recipients) -> Self {
        self.value.recipients = recipients;
        self
    }

    pub fn with_rendered(mut self, title: String, message: String) -> Self {
        self.value.title = Some(title);
        self.value.message = Some(message);
        self
    }
}
