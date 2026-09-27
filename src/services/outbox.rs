use crate::config;
use crate::infras::kafka::KafkaBus;
use crate::models::{NotifyMessage, OutboxData, EVENT_TYPE_SEND};
use crate::repository::OutboxRepository;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

const DEFAULT_POLL_INTERVAL_MS: u64 = 1000;
const DEFAULT_BATCH_SIZE: i64 = 100;

/// Polls `outbox` QUEUED → PROCESSING → publish Kafka → SENT (+offset) | FAILED.
#[derive(Clone)]
pub struct OutboxService {
    outbox: OutboxRepository,
    kafka: Arc<KafkaBus>,
}

impl OutboxService {
    pub fn new(outbox: OutboxRepository, kafka: Arc<KafkaBus>) -> Self {
        Self { outbox, kafka }
    }

    pub async fn run_publisher(self: Arc<Self>) {
        tracing::info!("Outbox publisher started");
        loop {
            match self.poll_once().await {
                Ok(0) => sleep(Duration::from_millis(DEFAULT_POLL_INTERVAL_MS)).await,
                Ok(n) => tracing::debug!(published = n, "Outbox poll batch processed"),
                Err(err) => {
                    tracing::error!(error = %err, "Outbox publisher poll failed; retrying in 5s");
                    sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    async fn poll_once(&self) -> anyhow::Result<usize> {
        let rows = self.outbox.claim_queued(DEFAULT_BATCH_SIZE).await?;
        let mut published = 0usize;
        for row in rows {
            if self.publish_row(&row).await {
                published += 1;
            }
        }
        Ok(published)
    }

    async fn publish_row(&self, row: &OutboxData) -> bool {
        let cfg = config::load();
        let send_topic = cfg.kafka.topics.send.as_str();

        let result = if row.topic == send_topic {
            match serde_json::from_value::<NotifyMessage>(row.payload.clone()) {
                Ok(msg) => {
                    self.kafka
                        .publish_json(
                            &row.topic,
                            &row.partition_key,
                            &msg.value.source,
                            &msg.value.kind,
                            EVENT_TYPE_SEND,
                            &msg,
                        )
                        .await
                }
                Err(err) => Err(format!("invalid notify payload: {err}")),
            }
        } else {
            Err(format!("unsupported outbox topic `{}`", row.topic))
        };

        match result {
            Ok((partition, offset)) => {
                if let Err(err) = self.outbox.mark_sent(row.id, partition, offset).await {
                    tracing::error!(outbox_id = row.id, error = %err, "Failed to mark outbox SENT");
                    return false;
                }
                tracing::info!(
                    outbox_id = row.id,
                    topic = %row.topic,
                    partition,
                    offset,
                    correlation_id = %row.correlation_id,
                    "Outbox row sent"
                );
                true
            }
            Err(err) => {
                tracing::error!(
                    outbox_id = row.id,
                    topic = %row.topic,
                    error = %err,
                    "Outbox publish failed"
                );
                let _ = self.outbox.mark_failed(row.id, &err).await;
                false
            }
        }
    }
}
