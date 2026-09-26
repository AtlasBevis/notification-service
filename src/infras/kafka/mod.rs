use anyhow::{Context, Result};
use chrono::Utc;
use futures::StreamExt;
use rskafka::client::consumer::{StartOffset, StreamConsumer, StreamConsumerBuilder};
use rskafka::client::partition::{Compression, PartitionClient, UnknownTopicHandling};
use rskafka::client::{Client, ClientBuilder, Credentials, SaslConfig};
use rskafka::record::Record;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::Instrument;

use crate::config;
use crate::config::KafkaConfig;

/// Kafka produce/consume helper (rskafka, partition-aware, key = correlation_id).
#[derive(Clone)]
pub struct KafkaBus {
    client: Arc<Client>,
    partitions: Arc<Mutex<HashMap<(String, i32), Arc<PartitionClient>>>>,
    topic_partition_count: Arc<Mutex<HashMap<String, i32>>>,
}

impl KafkaBus {
    pub async fn connect(config: &KafkaConfig) -> Result<Self> {
        if config.brokers.is_empty() {
            anyhow::bail!("kafka.brokers is empty");
        }

        let mut builder =
            ClientBuilder::new(config.brokers.clone()).client_id(config.client_id.clone());

        if !config.sasl.username.is_empty() {
            let creds = Credentials {
                username: config.sasl.username.clone(),
                password: config.sasl.password.clone(),
            };
            let sasl = match config.sasl.mechanism.to_ascii_uppercase().as_str() {
                "" | "PLAIN" => SaslConfig::Plain(creds),
                "SCRAM-SHA-256" | "SCRAMSHA256" => SaslConfig::ScramSha256(creds),
                "SCRAM-SHA-512" | "SCRAMSHA512" => SaslConfig::ScramSha512(creds),
                other => anyhow::bail!("Unsupported kafka SASL mechanism `{other}`"),
            };
            builder = builder.sasl_config(sasl);
        }

        let client = builder
            .build()
            .await
            .context("Failed to connect to Kafka")?;

        Ok(Self {
            client: Arc::new(client),
            partitions: Arc::new(Mutex::new(HashMap::new())),
            topic_partition_count: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub async fn ping(&self) -> Result<()> {
        self.client
            .list_topics()
            .await
            .map(|_| ())
            .context("Kafka ping failed")
    }

    /// Produce one record; returns `(partition, offset)`.
    pub async fn publish(
        &self,
        topic: &str,
        key: &str,
        headers: BTreeMap<String, Vec<u8>>,
        payload: &[u8],
    ) -> Result<(i32, i64)> {
        let count = self.partition_count(topic).await?;
        let partition = kafka_partition(key.as_bytes(), count);
        let client = self.partition_client(topic, partition).await?;
        let record = Record {
            key: Some(key.as_bytes().to_vec()),
            value: Some(payload.to_vec()),
            headers,
            timestamp: Utc::now(),
        };
        let offsets = client
            .produce(vec![record], Compression::default())
            .await
            .with_context(|| format!("Failed to produce to {topic}[{partition}]"))?;
        let offset = offsets.first().copied().ok_or_else(|| {
            anyhow::anyhow!("Kafka produce returned no offset for {topic}[{partition}]")
        })?;
        Ok((partition, offset))
    }

    pub async fn partition_ids(&self, topic: &str) -> Result<Vec<i32>> {
        let count = self.partition_count(topic).await?;
        Ok((0..count).collect())
    }

    pub async fn partition_client(
        &self,
        topic: &str,
        partition: i32,
    ) -> Result<Arc<PartitionClient>> {
        let key = (topic.to_string(), partition);
        {
            let guard = self.partitions.lock().await;
            if let Some(existing) = guard.get(&key) {
                return Ok(existing.clone());
            }
        }

        let client = self
            .client
            .partition_client(topic.to_string(), partition, UnknownTopicHandling::Retry)
            .await
            .with_context(|| format!("Failed to open partition {topic}[{partition}]"))?;
        let client = Arc::new(client);

        let mut guard = self.partitions.lock().await;
        guard.insert(key, client.clone());
        Ok(client)
    }

    pub fn stream(
        partition_client: Arc<PartitionClient>,
        start: StartOffset,
        max_wait_ms: i32,
    ) -> StreamConsumer {
        StreamConsumerBuilder::new(partition_client, start)
            .with_max_wait_ms(max_wait_ms)
            .build()
    }

    async fn partition_count(&self, topic: &str) -> Result<i32> {
        {
            let guard = self.topic_partition_count.lock().await;
            if let Some(count) = guard.get(topic) {
                return Ok(*count);
            }
        }

        let topics = self
            .client
            .list_topics()
            .await
            .context("Failed to list Kafka topics")?;
        let count = topics
            .iter()
            .find(|t| t.name == topic)
            .map(|t| t.partitions.len() as i32)
            .filter(|n| *n > 0)
            .unwrap_or(1);

        let mut guard = self.topic_partition_count.lock().await;
        guard.insert(topic.to_string(), count);
        Ok(count)
    }

    /// Publish JSON with Kafka message key = `key` (correlation_id); returns `(partition, offset)`.
    pub async fn publish_json(
        &self,
        topic: &str,
        key: &str,
        source: &str,
        kind: &str,
        event_type: &str,
        body: &impl Serialize,
    ) -> Result<(i32, i64), String> {
        let payload = serde_json::to_vec(body).map_err(|e| e.to_string())?;

        let mut headers = BTreeMap::new();
        headers.insert("source".to_string(), source.as_bytes().to_vec());
        headers.insert("correlation_id".to_string(), key.as_bytes().to_vec());
        headers.insert("kind".to_string(), kind.as_bytes().to_vec());
        headers.insert("type".to_string(), event_type.as_bytes().to_vec());
        headers.insert("content-type".to_string(), b"application/json".to_vec());

        let (partition, offset) = self
            .publish(topic, key, headers, &payload)
            .await
            .map_err(|e| e.to_string())?;

        metrics::counter!(
            "notification_produced_total",
            "topic" => topic.to_owned(),
            "source" => source.to_owned()
        )
        .increment(1);
        Ok((partition, offset))
    }

    /// Consume `topics` forever (restart on crash). Calls `handler` per record.
    pub async fn run_consumers<H, Fut>(self: Arc<Self>, topics: Vec<String>, handler: H)
    where
        H: Fn(String, i32, i64, Option<Vec<u8>>) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        loop {
            if let Err(err) = self.run_once(topics.clone(), handler.clone()).await {
                tracing::error!(error = %err, "[CRASHED] Kafka consumers restarting in 5s...");
                sleep(Duration::from_secs(5)).await;
            }
        }
    }

    async fn run_once<H, Fut>(&self, topics: Vec<String>, handler: H) -> Result<()>
    where
        H: Fn(String, i32, i64, Option<Vec<u8>>) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let mut join_set = tokio::task::JoinSet::new();
        for topic in topics {
            self.spawn_topic(&mut join_set, topic, handler.clone())
                .await?;
        }

        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(Ok(())) => {}
                Ok(Err(err)) => return Err(err),
                Err(err) => anyhow::bail!("consumer task join error: {err}"),
            }
        }
        Ok(())
    }

    async fn spawn_topic<H, Fut>(
        &self,
        join_set: &mut tokio::task::JoinSet<Result<()>>,
        topic: String,
        handler: H,
    ) -> Result<()>
    where
        H: Fn(String, i32, i64, Option<Vec<u8>>) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let partitions = loop {
            match self.partition_ids(&topic).await {
                Ok(ids) if !ids.is_empty() => break ids,
                Ok(_) => {
                    tracing::warn!(%topic, "Topic has no partitions yet; retrying");
                }
                Err(err) => {
                    tracing::warn!(%topic, error = %err, "Waiting for Kafka topic");
                }
            }
            sleep(Duration::from_secs(5)).await;
        };

        for partition in partitions {
            let bus = Arc::new(self.clone());
            let topic = topic.clone();
            let handler = handler.clone();
            join_set.spawn(async move { consume_partition(bus, topic, partition, handler).await });
        }
        Ok(())
    }
}

async fn consume_partition<H, Fut>(
    bus: Arc<KafkaBus>,
    topic: String,
    partition: i32,
    handler: H,
) -> Result<()>
where
    H: Fn(String, i32, i64, Option<Vec<u8>>) -> Fut,
    Fut: Future<Output = ()>,
{
    let cfg = config::load();
    let max_wait_ms = cfg.kafka.consumer.max_wait_ms.max(100);
    let max_poll_records = cfg.kafka.consumer.max_poll_records.max(1) as usize;
    let start = start_offset();
    let client = bus.partition_client(&topic, partition).await?;
    let mut stream = KafkaBus::stream(client, start, max_wait_ms);
    let span = tracing::info_span!("kafka_consumer", %topic, partition);

    async {
        tracing::info!(%topic, partition, max_poll_records, "Kafka consumer started");
        loop {
            let mut polled = 0usize;
            while polled < max_poll_records {
                match stream.next().await {
                    Some(Ok((record_and_offset, _hwm))) => {
                        polled += 1;
                        handler(
                            topic.clone(),
                            partition,
                            record_and_offset.offset,
                            record_and_offset.record.value,
                        )
                        .await;
                    }
                    Some(Err(err)) => {
                        tracing::error!(%topic, partition, error = %err, "Kafka fetch error");
                        sleep(Duration::from_secs(1)).await;
                        break;
                    }
                    None => {
                        anyhow::bail!("Kafka consumer stream ended for {topic}[{partition}]")
                    }
                }
            }
        }
    }
    .instrument(span)
    .await
}

fn start_offset() -> StartOffset {
    if config::load()
        .kafka
        .consumer
        .start_offset
        .eq_ignore_ascii_case("earliest")
    {
        StartOffset::Earliest
    } else {
        StartOffset::Latest
    }
}

/// Kafka default partitioner: `toPositive(murmur2(key)) % numPartitions`.
pub fn kafka_partition(key: &[u8], num_partitions: i32) -> i32 {
    let n = num_partitions.max(1);
    let hash = murmur2(key) & 0x7fff_ffff;
    (hash as i32).rem_euclid(n)
}

fn murmur2(data: &[u8]) -> i32 {
    const SEED: u32 = 0x9747b28c;
    const M: u32 = 0x5bd1e995;
    const R: u32 = 24;

    let length = data.len();
    let mut h = SEED ^ length as u32;
    let mut i = 0;
    while i + 4 <= length {
        let mut k = u32::from_le_bytes(data[i..i + 4].try_into().unwrap());
        k = k.wrapping_mul(M);
        k ^= k >> R;
        k = k.wrapping_mul(M);
        h = h.wrapping_mul(M);
        h ^= k;
        i += 4;
    }

    match length - i {
        3 => {
            h ^= (data[i + 2] as u32) << 16;
            h ^= (data[i + 1] as u32) << 8;
            h ^= data[i] as u32;
            h = h.wrapping_mul(M);
        }
        2 => {
            h ^= (data[i + 1] as u32) << 8;
            h ^= data[i] as u32;
            h = h.wrapping_mul(M);
        }
        1 => {
            h ^= data[i] as u32;
            h = h.wrapping_mul(M);
        }
        _ => {}
    }

    h ^= h >> 13;
    h = h.wrapping_mul(M);
    h ^= h >> 15;
    h as i32
}
