use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct KafkaConfig {
    pub enabled: bool,
    pub brokers: Vec<String>,
    pub client_id: String,
    pub security_protocol: KafkaSecurityProtocol,
    pub sasl: KafkaSaslConfig,
    pub topics: KafkaTopics,
    pub consumer: KafkaConsumerConfig,
}

impl KafkaConfig {
    pub fn consumer_enabled(&self) -> bool {
        self.enabled && self.consumer.enabled
    }
}

/// `PLAINTEXT` (no authentication) or `SASL_PLAINTEXT` (SASL over an unencrypted connection).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KafkaSecurityProtocol {
    Plaintext,
    SaslPlaintext,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KafkaSaslConfig {
    /// `PLAIN`, `SCRAM-SHA-256` or `SCRAM-SHA-512`; used only with `SASL_PLAINTEXT`.
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
