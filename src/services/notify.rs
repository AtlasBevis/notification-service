use crate::config;
use crate::enums::Channel;
use crate::error::{AppError, AppResult};
use crate::infras::kafka::KafkaBus;
use crate::models::{
    DefinitionData, DeliveryDraft, NotifyMessage, NotifyRequest, NotifyValue, TargetData,
    TargetPayload, TemplateData, EVENT_TYPE_SEND,
};
use crate::services::{
    ChannelTrait, DefinitionTrait, DeliveryTrait, EmailTrait, MSTrait, NotificationTrait,
    SourceTrait, TargetTrait, TemplateTrait, WebhookServiceTrait,
};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

const MAX_ATTEMPTS: u32 = 3;

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
    emails: Arc<dyn EmailTrait>,
    teams: Arc<dyn MSTrait>,
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
        emails: Arc<dyn EmailTrait>,
        teams: Arc<dyn MSTrait>,
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
            emails,
            teams,
            webhooks,
        }
    }

    async fn dispatch_direct_send(&self, request: &NotifyValue) -> AppResult<()> {
        match self.dispatch_send(request).await {
            Ok(()) => Ok(()),
            Err(err) => {
                tracing::error!(error = %err, "Direct channel dispatch failed");
                Err(AppError::internal(err.to_string()))
            }
        }
    }

    pub async fn run_consumers(self: Arc<Self>) {
        let Some(kafka) = self.kafka.clone() else {
            tracing::warn!("[SKIPPED]: Kafka is not connected");
            return;
        };

        let cfg = config::load();
        let send_topic = cfg.kafka.topics.send.clone();
        let topics = vec![send_topic.clone()];
        let svc = self.clone();

        kafka
            .run_consumers(topics, move |topic, partition, offset, value| {
                let svc = svc.clone();
                async move {
                    svc.handle_record(&topic, partition, offset, &value).await;
                }
            })
            .await;
    }

    async fn handle_record(
        &self,
        topic: &str,
        partition: i32,
        offset: i64,
        value: &Option<Vec<u8>>,
    ) {
        let Some(bytes) = value else {
            tracing::warn!(%topic, partition, offset, "Empty Kafka record");
            return;
        };

        let msg = match serde_json::from_slice::<NotifyMessage>(bytes) {
            Ok(msg) => msg,
            Err(err) => {
                tracing::error!(%topic, partition, offset, error = %err, "Invalid notify JSON; skipping");
                return;
            }
        };
        let delivery_id = msg.value.delivery_id;
        let max_attempts = self.max_attempts_for(&msg.value).await;
        match self
            .retry_dispatch(topic, delivery_id, max_attempts, || {
                self.dispatch_send(&msg.value)
            })
            .await
        {
            Ok(()) => {}
            Err(err) => tracing::error!(error = %err, delivery_id, "delivery failed"),
        }
    }

    async fn retry_dispatch<F, Fut>(
        &self,
        topic: &str,
        delivery_id: i64,
        max_attempts: u32,
        dispatch: F,
    ) -> AppResult<()>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = AppResult<()>>,
    {
        for attempt in 1..=max_attempts.max(1) {
            match dispatch().await {
                Ok(()) => {
                    if let Err(err) = self
                        .deliveries
                        .record_attempt(delivery_id, "SENT", "SUCCESS", None)
                        .await
                    {
                        tracing::error!(error = %err, delivery_id, "record successful attempt failed");
                    }
                    metrics::counter!(
                        "notification_consumed_total",
                        "topic" => topic.to_owned(),
                        "status" => "ok"
                    )
                    .increment(1);
                    return Ok(());
                }
                Err(err) if attempt < max_attempts => {
                    let error = err.to_string();
                    let attempt_status = if error.contains("timed out") {
                        "TIMEOUT"
                    } else {
                        "FAILED"
                    };
                    if let Err(db) = self
                        .deliveries
                        .record_attempt(delivery_id, "RETRY_WAIT", attempt_status, Some(&error))
                        .await
                    {
                        tracing::error!(error = %db, delivery_id, "record retry attempt failed");
                    }
                    tracing::warn!(attempt, error = %err, "Dispatch failed; retrying");
                    sleep(Duration::from_millis(200 * u64::from(attempt))).await;
                }
                Err(err) => {
                    let error = err.to_string();
                    let attempt_status = if error.contains("timed out") {
                        "TIMEOUT"
                    } else {
                        "FAILED"
                    };
                    if let Err(db) = self
                        .deliveries
                        .record_attempt(delivery_id, "FAILED", attempt_status, Some(&error))
                        .await
                    {
                        tracing::error!(error = %db, delivery_id, "record failed attempt failed");
                    }
                    tracing::error!(error = %err, "Dispatch exhausted retries");
                    metrics::counter!(
                        "notification_consumed_total",
                        "topic" => topic.to_owned(),
                        "status" => "failed"
                    )
                    .increment(1);
                    return Err(err);
                }
            }
        }
        Err(AppError::internal("dispatch failed"))
    }

    async fn max_attempts_for(&self, request: &NotifyValue) -> u32 {
        if request.channel != Channel::WEBHOOK {
            return MAX_ATTEMPTS;
        }
        match self.webhooks.max_attempts(request.definition_id).await {
            Ok(attempts) => attempts.max(1),
            Err(err) => {
                tracing::warn!(error = %err, "Could not resolve webhook retry policy");
                1
            }
        }
    }

    pub async fn dispatch_send(&self, request: &NotifyValue) -> AppResult<()> {
        let channel = request.channel.as_str();
        let result = match Channel::parse(channel) {
            Some(Channel::MSTEAMS) => self.teams.send_teams(request).await,
            Some(Channel::EMAIL) => self.emails.send_email(request).await,
            Some(Channel::WEBHOOK) => self.webhooks.send_webhook(request).await,
            None => Err(AppError::bad_request(format!(
                "unsupported channel `{channel}`"
            ))),
        };

        match result {
            Ok(()) => {
                tracing::info!(source = %request.source, channel, "Channel dispatched");
                metrics::counter!(
                    "notification_dispatch_total",
                    "channel" => channel.to_owned(),
                    "status" => "ok"
                )
                .increment(1);
                Ok(())
            }
            Err(err) => {
                tracing::error!(source = %request.source, channel, error = %err, "Channel dispatch failed");
                metrics::counter!(
                    "notification_dispatch_total",
                    "channel" => channel.to_owned(),
                    "status" => "error"
                )
                .increment(1);
                Err(err)
            }
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
        let source = self.sources.get_active_by_code(&request.source).await?;
        let definition = self.definitions.get_active_by_code(&request.code).await?;
        if definition.source_id != source.id {
            return Err(AppError::bad_request(format!(
                "code `{}` is not valid for source `{}`",
                definition.code, source.code,
            )));
        }

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

        let cfg = config::load();
        let use_kafka = cfg.kafka.enabled && self.kafka.is_some();
        {
            let value = &msg.value;
            if use_kafka {
                let topic = cfg.kafka.topics.send.clone();
                let kafka = self
                    .kafka
                    .as_ref()
                    .expect("kafka client present when kafka is enabled");
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
                    }
                    Err(err) => {
                        tracing::error!(error = %err, code = %value.code, "Kafka publish failed");
                        let _ = self
                            .deliveries
                            .record_result(value.delivery_id, "FAILED", Some(&err.to_string()))
                            .await;
                        return Err(err);
                    }
                }
            } else {
                match self.dispatch_direct_send(value).await {
                    Ok(()) => {
                        self.deliveries
                            .record_result(value.delivery_id, "SENT", None)
                            .await?;
                    }
                    Err(err) => {
                        let message = err.to_string();
                        let _ = self
                            .deliveries
                            .record_result(value.delivery_id, "FAILED", Some(&message))
                            .await;
                        return Err(err);
                    }
                }
            }
        }

        Ok(())
    }
}
