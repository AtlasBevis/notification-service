use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    host: String,
    pub profile: String,
    port: u16,
    admin: String,
    pub version: String,
    pub metrics: MetricsConfig,
    pub grpc: GrpcConfig,
    pub http: HttpConfig,
}

impl ServerConfig {
    pub fn http_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
    pub fn metrics_addr(&self) -> String {
        format!("{}:{}", self.host, self.metrics.port)
    }
    pub fn grpc_addr(&self) -> String {
        format!("{}:{}", self.host, self.grpc.port)
    }
    pub fn is_admin(&self, client: &str) -> bool {
        self.admin == client
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetricsConfig {
    pub enabled: bool,
    port: u16,
}

/// Internal gRPC API (`data-notification` only).
#[derive(Debug, Clone, Deserialize)]
pub struct GrpcConfig {
    pub enabled: bool,
    port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HttpConfig {
    pub timeout: u64,
}
