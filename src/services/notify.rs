use crate::config;
use crate::enums::StatusEnum;
use crate::error::{AppError, AppResult};
use crate::infras::http::HttpClient;
use crate::infras::kafka::KafkaBus;
use crate::models::{
    DeliveryDraft, NotificationData, NotifyMessage, NotifyRequest, NotifyValue, EVENT_TYPE_SEND,
};
use crate::repository::{DefinitionRepository, DeliveriesRepository};
use crate::services::definitions::{DefinitionService, DefinitionServiceTrait};
use crate::services::notification::NotificationService;
use crate::services::templates::wrap_teams_message;
use crate::services::{
    ChannelsService, ChannelsServiceTrait, NotificationServiceTrait, SourceService,
    SourcesServiceTrait, TargetsService, TargetsServiceTrait, TemplatesService,
    TemplatesServiceTrait,
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
pub trait NotifyServiceTrait: Send + Sync {
    async fn validate(&self, request: NotifyRequest) -> AppResult<NotifyMessage>;
    async fn notify(&self, message: NotifyMessage) -> AppResult<()>;
}

#[derive(Clone)]
pub struct NotifyService {
    kafka: Option<Arc<KafkaBus>>,
    http: Arc<HttpClient>,
    definitions: Arc<DefinitionService>,
    sources: Arc<SourceService>,
    channels: Arc<ChannelsService>,
    targets: Arc<TargetsService>,
    notifications: Arc<NotificationService>,
    deliveries: DeliveriesRepository,
    templates: Arc<TemplatesService>,
}

impl NotifyService {
    pub fn new(
        kafka: Option<Arc<KafkaBus>>,
        http: Arc<HttpClient>,
        definitions: Arc<DefinitionService>,
        notifications: Arc<NotificationService>,
        deliveries: DeliveriesRepository,
        sources: Arc<SourceService>,
        targets: Arc<TargetsService>,
        channels: Arc<ChannelsService>,
        templates: Arc<TemplatesService>,
    ) -> Self {
        Self {
            kafka,
            http,
            definitions,
            notifications,
            deliveries,
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
        match self
            .retry_dispatch(topic, || self.dispatch_send(&msg.value))
            .await
        {
            Ok(()) => {
                if let Err(err) = self
                    .deliveries
                    .record_result(delivery_id, "SENT", None)
                    .await
                {
                    tracing::error!(error = %err, delivery_id, "record SENT failed");
                }
            }
            Err(err) => {
                if let Err(db) = self
                    .deliveries
                    .record_result(delivery_id, "FAILED", Some(&err))
                    .await
                {
                    tracing::error!(error = %db, delivery_id, "record FAILED failed");
                }
            }
        }
    }

    async fn retry_dispatch<F, Fut>(&self, topic: &str, dispatch: F) -> Result<(), String>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        for attempt in 1..=MAX_ATTEMPTS {
            match dispatch().await {
                Ok(()) => {
                    metrics::counter!(
                        "notification_consumed_total",
                        "topic" => topic.to_owned(),
                        "status" => "ok"
                    )
                    .increment(1);
                    return Ok(());
                }
                Err(err) if attempt < MAX_ATTEMPTS => {
                    tracing::warn!(attempt, error = %err, "Dispatch failed; retrying");
                    sleep(Duration::from_millis(200 * u64::from(attempt))).await;
                }
                Err(err) => {
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
        let target = self
            .targets
            .must_active(&request.target)
            .await
            .map_err(|msg| anyhow::anyhow!(msg))?;
        let url = target.teams_webhook_url().ok_or_else(|| {
            anyhow::anyhow!(
                "target `{}` metadata.url is required for WEBHOOK",
                request.target
            )
        })?;
        let body = serde_json::json!({
            "code": request.code,
            "source": request.source,
            "trace_id": request.trace_id,
            "level": request.kind,
            "title": request.title,
            "data": request.payload,
        });
        self.http.post_json(&url, &HashMap::new(), &body).await?;
        Ok(())
    }

    async fn build_notify_message(
        &self,
        request: &NotifyRequest,
        definition: &NotificationData,
    ) -> Result<NotifyMessage, String> {
        let template = self
            .templates
            .must_active_by_id(definition.template_id)
            .await?;

        let recipients = target.recipients();
        if definition.channel_code.eq_ignore_ascii_case("EMAIL") && !recipients.has_to() {
            return Err(format!(
                "target `{}` metadata.to is required for EMAIL channel",
                definition.target_code
            ));
        }

        let (title, message) = template.merge_template(&request.variables);
        Ok(NotifyMessage::new(request)
            .with_definition(definition, template.name, recipients)
            .with_rendered(title, message))
    }
}

#[async_trait]
impl NotifyServiceTrait for NotifyService {
    async fn validate(&self, request: NotifyRequest) -> AppResult<NotifyMessage> {
        // definitions
        let def = self.definitions.get_active_by_code(&request.code).await?;

        // sources
        let source = self.sources.get_active_by_id(def.source_id).await?;

        // channels
        let channels = self.channels.get_active_by_id(def.channel_id).await?;

        // target
        let target = self.targets.get_active_by_id(def.target_id).await?;

        let templates = self.templates.get_active_by_id(def.template_id).await?;

        // templates
        self.build_notify_message(&request, &definition)
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
