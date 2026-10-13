use crate::config;
use crate::enums::Channel;
use crate::error::{AppError, AppResult};
use crate::infras::kafka::KafkaBus;
use crate::models::{NotifyMessage, NotifyValue};
use crate::services::{DeliveryTrait, EmailTrait, MSTrait, WebhookServiceTrait};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

const MAX_ATTEMPTS: u32 = 3;

/// Job side: consume published notifications and dispatch them to their channel.
#[async_trait]
pub trait ConsumerTrait: Send + Sync {
    async fn consume(self: Arc<Self>);
}

#[derive(Clone)]
pub struct ConsumerService {
    kafka: Arc<KafkaBus>,
    deliveries: Arc<dyn DeliveryTrait>,
    emails: Arc<dyn EmailTrait>,
    teams: Arc<dyn MSTrait>,
    webhooks: Arc<dyn WebhookServiceTrait>,
}

impl ConsumerService {
    pub fn new(
        kafka: Arc<KafkaBus>,
        deliveries: Arc<dyn DeliveryTrait>,
        emails: Arc<dyn EmailTrait>,
        teams: Arc<dyn MSTrait>,
        webhooks: Arc<dyn WebhookServiceTrait>,
    ) -> Self {
        Self {
            kafka,
            deliveries,
            emails,
            teams,
            webhooks,
        }
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

    async fn dispatch_send(&self, request: &NotifyValue) -> AppResult<()> {
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
}

#[async_trait]
impl ConsumerTrait for ConsumerService {
    async fn consume(self: Arc<Self>) {
        let topics = vec![config::load().kafka.topics.send.clone()];
        let svc = self.clone();

        self.kafka
            .clone()
            .run_consumers(topics, move |topic, partition, offset, value| {
                let svc = svc.clone();
                async move {
                    svc.handle_record(&topic, partition, offset, &value).await;
                }
            })
            .await;
    }
}
