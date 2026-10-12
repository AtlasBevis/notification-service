use crate::config;
use crate::error::{AppError, AppResult};
use crate::infras::http::HttpClient;
use crate::infras::kafka::KafkaBus;
use crate::models::{
    DefinitionData, DeliveryDraft, NotifyMessage, NotifyRequest, NotifyValue, TargetData,
    EVENT_TYPE_SEND,
};
use crate::repository::{DeliveriesRepository, WebhookRepository};
use crate::services::definition::{DefinitionService, DefinitionTrait};
use crate::services::notification::NotificationService;
use crate::services::template::wrap_teams_message;
use crate::services::{
    ChannelService, ChannelTrait, SourceService, SourceTrait, TargetTrait, TargetsService,
    TemplateService, TemplateTrait, WebhookServiceTrait,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
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
    http: Arc<HttpClient>,
    definitions: Arc<DefinitionService>,
    sources: Arc<SourceService>,
    channels: Arc<ChannelService>,
    targets: Arc<TargetsService>,
    notifications: Arc<NotificationService>,
    deliveries: DeliveriesRepository,
    webhook_configs: WebhookRepository,
    webhooks: Arc<dyn WebhookServiceTrait>,
    templates: Arc<TemplateService>,
}

impl NotifyService {
    pub fn new(
        kafka: Option<Arc<KafkaBus>>,
        http: Arc<HttpClient>,
        definitions: Arc<DefinitionService>,
        notifications: Arc<NotificationService>,
        deliveries: DeliveriesRepository,
        webhook_configs: WebhookRepository,
        webhooks: Arc<dyn WebhookServiceTrait>,
        sources: Arc<SourceService>,
        targets: Arc<TargetsService>,
        channels: Arc<ChannelService>,
        templates: Arc<TemplateService>,
    ) -> Self {
        Self {
            kafka,
            http,
            definitions,
            notifications,
            deliveries,
            webhook_configs,
            webhooks,
            sources,
            targets,
            channels,
            templates,
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
    ) -> Result<(), String>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
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
                    return Err(err.to_string());
                }
            }
        }
        Err("dispatch failed".into())
    }

    async fn max_attempts_for(&self, request: &NotifyValue) -> u32 {
        if !request.channel.eq_ignore_ascii_case("WEBHOOK") {
            return MAX_ATTEMPTS;
        }
        let result = async {
            let channel = self
                .channels
                .must_active(&request.channel)
                .await
                .map_err(AppError::bad_request)?;
            self.webhook_configs
                .find_by_channel_id(channel.id)
                .await
                .map(|config| config.max_retries.saturating_add(1))
        }
        .await;
        match result {
            Ok(attempts) => attempts.max(1),
            Err(err) => {
                tracing::warn!(error = %err, "Could not resolve webhook retry policy");
                1
            }
        }
    }

    pub async fn dispatch_send(&self, request: &NotifyValue) -> Result<()> {
        let channel = request.channel.as_str();

        let result = match channel {
            "MSTEAMS" => self.send_teams(request).await,
            "EMAIL" => self.send_email(request).await,
            "WEBHOOK" => self.send_webhook(request).await,
            other => anyhow::bail!("Unsupported channel `{other}`"),
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

    async fn send_teams(&self, request: &NotifyValue) -> Result<()> {
        let channel = self
            .channels
            .must_active(&request.channel)
            .await
            .map_err(|msg| anyhow::anyhow!(msg))?;
        let target = self
            .targets
            .must_active(&request.target)
            .await
            .map_err(|msg| anyhow::anyhow!(msg))?;

        let url = target
            .teams_webhook_url()
            .or_else(|| channel.teams_webhook_url().ok())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Teams webhook url missing on target `{}` and channel `{}`",
                    request.target,
                    request.channel
                )
            })?;

        let raw = request
            .message
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow::anyhow!("rendered Teams card (message) is empty"))?;
        let card: Value = serde_json::from_str(raw)
            .context("rendered Teams template is not valid Adaptive Card JSON")?;
        let body = wrap_teams_message(card);
        self.http.post_json(&url, &HashMap::new(), &body).await?;
        Ok(())
    }

    async fn send_email(&self, request: &NotifyValue) -> Result<()> {
        let channel = self
            .channels
            .must_active(&request.channel)
            .await
            .map_err(|msg| anyhow::anyhow!(msg))?;
        let smtp = channel.smtp_config().map_err(|msg| anyhow::anyhow!(msg))?;

        let to = &request.recipients.to;
        if to.is_empty() {
            anyhow::bail!("No email recipients (target.metadata.to is required for EMAIL)");
        }

        let from: lettre::message::Mailbox = smtp.username.parse().with_context(|| {
            format!(
                "Invalid channel `{}` metadata.username `{}`",
                channel.code, smtp.username
            )
        })?;
        let title = request
            .title
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(request.kind.as_str());
        let subject = format!("[{}][{}] {}", request.source, request.kind, title);
        let text = request
            .message
            .clone()
            .unwrap_or_else(|| request.payload.to_string());

        let mut builder = Message::builder().from(from).subject(subject);
        for addr in to {
            let mailbox: lettre::message::Mailbox = addr
                .parse()
                .with_context(|| format!("Invalid email to `{addr}`"))?;
            builder = builder.to(mailbox);
        }
        for addr in &request.recipients.cc {
            let mailbox: lettre::message::Mailbox = addr
                .parse()
                .with_context(|| format!("Invalid email cc `{addr}`"))?;
            builder = builder.cc(mailbox);
        }
        for addr in &request.recipients.bcc {
            let mailbox: lettre::message::Mailbox = addr
                .parse()
                .with_context(|| format!("Invalid email bcc `{addr}`"))?;
            builder = builder.bcc(mailbox);
        }
        let email = builder
            .body(text)
            .context("Failed to build email message")?;

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&smtp.host)
            .with_context(|| format!("Invalid SMTP host `{}`", smtp.host))?
            .credentials(Credentials::new(smtp.username.clone(), smtp.password))
            .build();
        mailer
            .send(email)
            .await
            .with_context(|| format!("SMTP send via {} failed", smtp.host))?;

        tracing::debug!(source = %request.source, "Email sent");
        Ok(())
    }

    async fn send_webhook(&self, request: &NotifyValue) -> Result<()> {
        self.targets
            .must_active(&request.target)
            .await
            .map_err(|msg| anyhow::anyhow!(msg))?;
        let channel = self
            .channels
            .must_active(&request.channel)
            .await
            .map_err(|msg| anyhow::anyhow!(msg))?;
        let config = self.webhook_configs.find_by_channel_id(channel.id).await?;
        self.webhooks
            .send_webhook(&config, &request.payload)
            .await?;
        Ok(())
    }

    async fn build_notify_message(
        &self,
        request: &NotifyRequest,
        definition: &DefinitionData,
        source_code: &str,
        channel_code: &str,
        target: &TargetData,
    ) -> Result<NotifyMessage, String> {
        let template = self
            .templates
            .must_active_by_id(i64::from(definition.template_id))
            .await?;

        let recipients = target.recipients();
        if channel_code.eq_ignore_ascii_case("EMAIL") && !recipients.has_to() {
            return Err(format!(
                "target `{}` metadata.to is required for EMAIL channel",
                target.code
            ));
        }

        let (title, message) = template.merge_template(&request.variables);
        Ok(NotifyMessage::new(request)
            .with_definition(
                definition,
                source_code,
                channel_code,
                &target.code,
                template.name,
                recipients,
            )
            .with_rendered(title, message))
    }
}

#[async_trait]
impl NotifyTrait for NotifyService {
    async fn validate(&self, request: NotifyRequest) -> AppResult<NotifyMessage> {
        // definitions
        let definition = self.definitions.get_active_by_code(&request.code).await?;

        // sources
        let source = self.sources.get_active_by_id(definition.source_id).await?;

        // channels
        let channel = self
            .channels
            .get_active_by_id(definition.channel_id)
            .await?;

        // target
        let target = self.targets.get_active_by_id(definition.target_id).await?;
        if !target.channel_code.eq_ignore_ascii_case(&channel.code) {
            return Err(AppError::bad_request(format!(
                "target `{}` does not belong to channel `{}`",
                target.code, channel.code
            )));
        }

        self.build_notify_message(&request, &definition, &source.code, &channel.code, &target)
            .await
            .map_err(AppError::bad_request)
    }

    async fn notify(&self, mut message: NotifyMessage) -> AppResult<()> {
        let source_code = message.value.source.clone();
        let code = message.value.code.clone();
        let definition_id = message.value.definition_id;
        let idempotency_key = message.value.trace_id.clone();
        let variables = Value::Object(message.value.variables.clone());
        let drafts = vec![DeliveryDraft {
            target_code: message.value.target.clone(),
            channel_code: message.value.channel.clone(),
            title: message.value.title.clone().unwrap_or_default(),
            body: message.value.message.clone().unwrap_or_default(),
            payload: message.value.payload.clone(),
            destination: serde_json::to_value(&message.value.recipients)
                .unwrap_or_else(|_| Value::Object(Default::default())),
        }];

        let accepted = match self
            .notifications
            .accept(
                definition_id,
                &source_code,
                &code,
                &idempotency_key,
                &variables,
                &drafts,
            )
            .await
        {
            Ok(row) => row,
            Err(AppError::Conflict(_)) => {
                return Err(AppError::conflict(format!(
                    "notification `{code}` already accepted for trace `{idempotency_key}`"
                )));
            }
            Err(err) => {
                tracing::error!(error = %err, code, "accept notification failed");
                return Err(err);
            }
        };

        message.value.notification_id = accepted.id;
        message.value.delivery_id = accepted.delivery_id;

        let cfg = config::load();
        let use_kafka = cfg.kafka.enabled && self.kafka.is_some();
        {
            let value = &message.value;
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
                        &value.kind,
                        EVENT_TYPE_SEND,
                        &message,
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
                            .record_result(value.delivery_id, "FAILED", Some(&err))
                            .await;
                        return Err(AppError::internal(err));
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
