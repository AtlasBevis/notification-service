use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct KafkaConfig {
    pub enabled: bool,
    pub brokers: Vec<String>,
    pub client_id: String,
    pub sasl: KafkaSaslConfig,
    pub topics: KafkaTopics,
    pub consumer: KafkaConsumerConfig,
}

impl KafkaConfig {
    pub fn consumer_enabled(&self) -> bool {
        self.enabled && self.consumer.enabled
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct KafkaSaslConfig {
    pub mechanism: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KafkaTopics {
    pub send: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KafkaConsumerConfig {
    pub enabled: bool,
    pub max_wait_ms: i32,
    /// Max records to process per poll cycle, per partition.
    pub max_poll_records: i32,
    /// `latest` or `earliest`.
    pub start_offset: String,
}
