use crate::config;
use crate::enums::NotificationStatus;
use crate::error::{AppError, AppResult};
use crate::infras::http::HttpClient;
use crate::infras::kafka::KafkaBus;
use crate::models::{NotificationData, NotifyMessage, NotifyRequest, NotifyValue};
use crate::repository::{DeliveriesRepository, OutboxRepository};
use crate::services::notification::NotificationService;
use crate::services::templates::wrap_teams_message;
use crate::services::{
    ChannelsService, ChannelsServiceTrait, NotificationServiceTrait, SourcesService,
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
    notifications: Arc<NotificationService>,
    deliveries: DeliveriesRepository,
    outbox: OutboxRepository,
    sources: Arc<SourcesService>,
    targets: Arc<TargetsService>,
    channels: Arc<ChannelsService>,
    templates: Arc<TemplatesService>,
}

impl NotifyService {
    pub fn new(
        kafka: Option<Arc<KafkaBus>>,
        http: Arc<HttpClient>,
        notifications: Arc<NotificationService>,
        deliveries: DeliveriesRepository,
        outbox: OutboxRepository,
        sources: Arc<SourcesService>,
        targets: Arc<TargetsService>,
        channels: Arc<ChannelsService>,
        templates: Arc<TemplatesService>,
    ) -> Self {
        Self {
            kafka,
            http,
            notifications,
            deliveries,
            outbox,
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
        self.retry_dispatch(topic, || self.dispatch_send(&msg.value))
            .await;
    }

    async fn retry_dispatch<F, Fut>(&self, topic: &str, dispatch: F)
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
                    return;
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
                    return;
                }
            }
        }
    }

    pub async fn dispatch_send(&self, request: &NotifyValue) -> Result<()> {
        let channel = request.channel.as_str();

        let result = match channel {
            "MSTEAMS" => self.send_teams(request).await,
            "EMAIL" => self.send_email(request).await,
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

    async fn build_notify_message(
        &self,
        request: &NotifyRequest,
        config: &NotificationData,
    ) -> Result<NotifyMessage, String> {
        self.sources.must_active(&config.source).await?;
        let target = self.targets.must_active(&config.target).await?;
        self.channels.must_active(&config.channel).await?;
        let template = self.templates.must_active_by_id(config.template_id).await?;

        let recipients = target.recipients();
        if config.channel.eq_ignore_ascii_case("EMAIL") && !recipients.has_to() {
            return Err(format!(
                "target `{}` metadata.to is required for EMAIL channel",
                config.target
            ));
        }

        if config.source.eq_ignore_ascii_case("AIRFLOW") {
            let team = match &config.metadata {
                Value::Object(m) => m
                    .get("team")
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .unwrap_or(""),
                _ => "",
            };
            if team.is_empty() {
                return Err("notification metadata.team is required for AIRFLOW".into());
            }
        }

        let (title, message) = template.merge_template(&request.variables);
        Ok(NotifyMessage::new(request)
            .with_notification(config, template.name, recipients)
            .with_rendered(title, message))
    }

}

#[async_trait]
impl NotifyServiceTrait for NotifyService {
    async fn validate(&self, request: NotifyRequest) -> AppResult<NotifyMessage> {
        let noti = self.notifications.get_by_code(&request.code).await?;
        if NotificationStatus::Active.as_str() != noti.status {
            return Err(AppError::bad_request(format!(
                "notification `{}` is not ACTIVE",
                noti.code
            )));
        }

        self.build_notify_message(&request, &noti)
            .await
            .map_err(AppError::bad_request)
    }

    async fn notify(&self, message: NotifyMessage) -> AppResult<()> {
        let cfg = config::load();
        let use_kafka = cfg.kafka.enabled && self.kafka.is_some();
        let value = &message.value;
        let correlation_id = value.trace_id.as_str();
        let title = value.title.clone().unwrap_or_default();
        let rendered = value.message.clone().unwrap_or_default();

        let recipients = serde_json::to_value(&value.recipients)
            .unwrap_or_else(|_| Value::Object(Default::default()));
        let metadata = Value::Object(Default::default());

        if use_kafka {
            let topic = cfg.kafka.topics.send.clone();
            let partition_key = correlation_id.to_string();

            match self
                .outbox
                .insert_delivery_with_outbox(
                    value.notification_id,
                    correlation_id,
                    &value.kind,
                    &title,
                    &rendered,
                    &recipients,
                    &value.payload,
                    &metadata,
                    &topic,
                    &partition_key,
                    |_| serde_json::to_value(&message).unwrap_or(Value::Null),
                )
                .await
            {
                Ok(_) => {}
                Err(AppError::Conflict(_)) => {
                    return Err(AppError::conflict(format!(
                        "delivery already exists for notification `{}` correlation_id `{correlation_id}`",
                        value.code
                    )));
                }
                Err(err) => {
                    tracing::error!(error = %err, code = %value.code, "insert outbox failed");
                    return Err(err);
                }
            }
        } else {
            if let Err(err) = self
                .deliveries
                .insert(
                    value.notification_id,
                    correlation_id,
                    &value.kind,
                    &title,
                    &rendered,
                    &recipients,
                    &value.payload,
                    &metadata,
                )
                .await
            {
                tracing::error!(error = %err, code = %value.code, "insert delivery failed");
                return Err(match err {
                    AppError::Conflict(_) => AppError::conflict(format!(
                        "delivery already exists for notification `{}` correlation_id `{correlation_id}`",
                        value.code
                    )),
                    err => err,
                });
            }
            self.dispatch_direct_send(value).await?;
        }

        Ok(())
    }
}
