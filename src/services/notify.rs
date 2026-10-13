use crate::config::Config;
use crate::enums::Channel;
use crate::error::{AppError, AppResult};
use crate::infras::kafka::KafkaBus;
use crate::models::{
    DefinitionData, DeliveryDraft, NotifyMessage, NotifyRequest, TargetData, TargetPayload,
    TemplateData, EVENT_TYPE_SEND,
};
use crate::services::{
    ChannelTrait, DefinitionTrait, DeliveryTrait, NotificationTrait, SourceTrait, TargetTrait,
    TemplateTrait, WebhookServiceTrait,
};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// API side: validate a send request, record it, and publish it to Kafka.
/// Channels are dispatched by `ConsumerService` in `notification-job`.
#[async_trait]
pub trait NotifyTrait: Send + Sync {
    async fn validate(&self, request: NotifyRequest) -> AppResult<NotifyMessage>;
    async fn notify(&self, message: NotifyMessage) -> AppResult<()>;
}

#[derive(Clone)]
pub struct NotifyService {
    kafka: Option<Arc<KafkaBus>>,
    definitions: Arc<dyn DefinitionTrait>,
    sources: Arc<dyn SourceTrait>,
    channels: Arc<dyn ChannelTrait>,
    targets: Arc<dyn TargetTrait>,
    notifications: Arc<dyn NotificationTrait>,
    deliveries: Arc<dyn DeliveryTrait>,
    templates: Arc<dyn TemplateTrait>,
    webhooks: Arc<dyn WebhookServiceTrait>,
}

impl NotifyService {
    pub fn new(
        kafka: Option<Arc<KafkaBus>>,
        definitions: Arc<dyn DefinitionTrait>,
        notifications: Arc<dyn NotificationTrait>,
        deliveries: Arc<dyn DeliveryTrait>,
        sources: Arc<dyn SourceTrait>,
        targets: Arc<dyn TargetTrait>,
        channels: Arc<dyn ChannelTrait>,
        templates: Arc<dyn TemplateTrait>,
        webhooks: Arc<dyn WebhookServiceTrait>,
    ) -> Self {
        Self {
            kafka,
            definitions,
            notifications,
            deliveries,
            sources,
            targets,
            channels,
            templates,
            webhooks,
        }
    }

    async fn build_notify_message(
        &self,
        request: &NotifyRequest,
        definition: &DefinitionData,
        channel: &crate::models::ChannelData,
        targets: &[TargetData],
    ) -> AppResult<NotifyMessage> {
        let channel_code = channel.code.as_str();
        let is_webhook = channel.code == Channel::WEBHOOK;

        // A WEBHOOK target sends its own template as the request body; a target
        // without a template sends the request's variables as the body.
        let mut resolved: HashMap<i32, TemplateData> = HashMap::new();
        let mut target_payloads = Vec::new();
        if is_webhook {
            for target in &definition.targets {
                let Some(template_id) = target.template_id else {
                    continue;
                };
                if !resolved.contains_key(&template_id) {
                    let template = self.templates.get_active_by_id(template_id).await?;
                    resolved.insert(template_id, template);
                }
                target_payloads.push(TargetPayload {
                    target_id: target.target_id,
                    payload: resolved[&template_id].json_payload()?,
                });
            }
        }

        // Title and body recorded on the delivery come from the first target
        // that has a template. A WEBHOOK definition without any template
        // records the definition name and the variables it sends.
        let first_template_id = definition
            .targets
            .iter()
            .find_map(|target| target.template_id);
        let template = match first_template_id {
            Some(template_id) => {
                let template = match resolved.remove(&template_id) {
                    Some(template) => template,
                    None => self.templates.get_active_by_id(template_id).await?,
                };
                if template.is_json() != is_webhook {
                    return Err(AppError::bad_request(format!(
                        "template `{}` format `{}` is not valid for channel `{channel_code}`",
                        template.name, template.format
                    )));
                }
                Some(template)
            }
            None if is_webhook => None,
            None => {
                return Err(AppError::bad_request(format!(
                    "definition `{}` has no template",
                    definition.code
                )))
            }
        };

        let mut recipients = crate::models::Recipients::default();
        for target in targets {
            let group = target.recipients();
            recipients.to.extend(group.to);
            recipients.cc.extend(group.cc);
            recipients.bcc.extend(group.bcc);
        }
        if channel_code.eq_ignore_ascii_case(Channel::EMAIL.as_str()) && !recipients.has_to() {
            return Err(AppError::bad_request(format!(
                "target `{}` metadata.to is required for EMAIL channel",
                targets
                    .iter()
                    .map(|item| item.code.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            )));
        }

        let (template_name, title, message) = match template {
            Some(template) => {
                let (title, message) = template.merge_template(&request.variables);
                (template.name, title, message)
            }
            None => (
                String::new(),
                definition.name.clone(),
                Value::Object(request.variables.clone()).to_string(),
            ),
        };
        Ok(NotifyMessage::with_request(request)
            .with_definition(definition)
            .with_channel(channel)
            .with_targets(targets)
            .with_target_payloads(target_payloads)
            .with_template(template_name)
            .with_recipients(recipients)
            .with_rendered(title, message))
    }
}

#[async_trait]
impl NotifyTrait for NotifyService {
    async fn validate(&self, request: NotifyRequest) -> AppResult<NotifyMessage> {
        // check source
        let source = self.sources.get_active_by_code(&request.source).await?;

        // check definition
        let definition = self.definitions.get_active_by_code(&request.code).await?;
        if definition.source_id != source.id {
            return Err(AppError::bad_request(format!(
                "code `{}` is not valid for source `{}`",
                definition.code, source.code,
            )));
        }

        // check channel
        let channel = self
            .channels
            .get_active_by_id(definition.channel_id)
            .await?;
        let kind = channel.code;
        if definition.targets.is_empty() {
            return Err(AppError::bad_request(format!(
                "definition `{}` has no targets",
                definition.code
            )));
        }
        let mut targets = Vec::with_capacity(definition.targets.len());
        for definition_target in &definition.targets {
            let target = self
                .targets
                .get_active_by_id(definition_target.target_id)
                .await?;
            if target.channel_code != channel.code {
                return Err(AppError::bad_request(format!(
                    "target `{}` does not belong to channel `{}`",
                    target.code, channel.code
                )));
            }
            targets.push(target);
        }
        if kind == Channel::WEBHOOK {
            self.webhooks.validate(&definition).await?;
        }

        self.build_notify_message(&request, &definition, &channel, &targets)
            .await
    }

    async fn notify(&self, mut msg: NotifyMessage) -> AppResult<()> {
        let kafka = self
            .kafka
            .as_ref()
            .ok_or_else(|| AppError::internal("Kafka is not connected; cannot publish"))?;

        let source_code = msg.value.source.clone();
        let code = msg.value.code.clone();
        let definition_id = msg.value.definition_id;
        let trace_id = msg.value.trace_id.clone();
        let variables = Value::Object(msg.value.variables.clone());
        let drafts = vec![DeliveryDraft {
            target_code: msg.value.target.clone(),
            channel_code: msg.value.channel.as_str().to_string(),
            title: msg.value.title.clone().unwrap_or_default(),
            body: msg.value.message.clone().unwrap_or_default(),
            payload: variables.clone(),
            destination: serde_json::to_value(&msg.value.recipients)
                .unwrap_or_else(|_| Value::Object(Default::default())),
        }];

        let accepted = match self
            .notifications
            .accept(
                definition_id,
                &source_code,
                &code,
                &trace_id,
                &variables,
                &drafts,
            )
            .await
        {
            Ok(row) => row,
            Err(AppError::Conflict(_)) => {
                return Err(AppError::conflict(format!(
                    "notification `{code}` already accepted for trace `{trace_id}`"
                )));
            }
            Err(err) => {
                tracing::error!(error = %err, code, "accept notification failed");
                return Err(err);
            }
        };

        msg.value.notification_id = accepted.id;
        msg.value.delivery_id = accepted.delivery_id;

        let topic = Config::load().kafka.topics.send.clone();
        let value = &msg.value;
        match kafka
            .publish_json(
                &topic,
                &value.trace_id,
                &value.source,
                value.level.as_str(),
                EVENT_TYPE_SEND,
                &msg,
            )
            .await
        {
            Ok((partition, offset)) => {
                tracing::info!(
                    topic,
                    partition,
                    offset,
                    trace_id = %value.trace_id,
                    code = %value.code,
                    delivery_id = value.delivery_id,
                    "Notification published"
                );
                Ok(())
            }
            Err(err) => {
                tracing::error!(error = %err, code = %value.code, "Kafka publish failed");
                let _ = self
                    .deliveries
                    .record_result(value.delivery_id, "FAILED", Some(&err.to_string()))
                    .await;
                Err(err)
            }
        }
    }
}
