use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct KafkaConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub brokers: Vec<String>,

    #[serde(default)]
    pub client_id: String,

    #[serde(default)]
    pub sasl: KafkaSaslConfig,

    #[serde(default)]
    pub topics: KafkaTopics,

    #[serde(default)]
    pub consumer: KafkaConsumerConfig,
}

impl Default for KafkaConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            brokers: Vec::new(),
            client_id: String::default(),
            sasl: KafkaSaslConfig::default(),
            topics: KafkaTopics::default(),
            consumer: KafkaConsumerConfig::default(),
        }
    }
}

impl KafkaConfig {
    pub fn consumer_enabled(&self) -> bool {
        self.enabled && self.consumer.enabled
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct KafkaSaslConfig {
    #[serde(default)]
    pub mechanism: String,

    #[serde(default)]
    pub username: String,

    #[serde(default)]
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KafkaTopics {
    #[serde(default = "default_send_topic")]
    pub send: String,
}

impl Default for KafkaTopics {
    fn default() -> Self {
        Self {
            send: default_send_topic(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct KafkaConsumerConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(default = "default_max_wait_ms")]
    pub max_wait_ms: i32,

    /// Max records to process per poll cycle, per partition.
    #[serde(default = "default_max_poll_records")]
    pub max_poll_records: i32,

    /// `latest` (default) or `earliest`.
    #[serde(default = "default_start_offset")]
    pub start_offset: String,
}

impl Default for KafkaConsumerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_wait_ms: default_max_wait_ms(),
            max_poll_records: default_max_poll_records(),
            start_offset: default_start_offset(),
        }
    }
}

fn default_send_topic() -> String {
    "notification.send".to_string()
}

fn default_true() -> bool {
    true
}

fn default_max_wait_ms() -> i32 {
    1000
}

fn default_max_poll_records() -> i32 {
    100
}

fn default_start_offset() -> String {
    "latest".to_string()
}
