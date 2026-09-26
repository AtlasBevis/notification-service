use anyhow::{Context, Result};
use async_trait::async_trait;
use http::Extensions;
use reqwest::{Method, Request, Response};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware, Middleware, Next};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, Instant};

const MAX_LOG_BODY_BYTES: usize = 4096;

/// App-facing HTTP client (reqwest + logging middleware) with base verbs.
#[derive(Clone)]
pub struct HttpClient {
    inner: ClientWithMiddleware,
}

/// Build a process-wide HTTP client with external call logging.
pub fn build_http_client(timeout_secs: u64) -> Result<HttpClient> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs.max(1)))
        .build()
        .context("Failed to build http client")?;

    Ok(HttpClient {
        inner: ClientBuilder::new(client).with(ExternalHttpLogging).build(),
    })
}

impl HttpClient {
    pub async fn get(&self, url: &str, headers: &HashMap<String, String>) -> Result<Response> {
        self.request(Method::GET, url, headers, None).await
    }

    pub async fn post(
        &self,
        url: &str,
        headers: &HashMap<String, String>,
        body: Option<Vec<u8>>,
    ) -> Result<Response> {
        self.request(Method::POST, url, headers, body).await
    }

    pub async fn put(
        &self,
        url: &str,
        headers: &HashMap<String, String>,
        body: Option<Vec<u8>>,
    ) -> Result<Response> {
        self.request(Method::PUT, url, headers, body).await
    }

    pub async fn patch(
        &self,
        url: &str,
        headers: &HashMap<String, String>,
        body: Option<Vec<u8>>,
    ) -> Result<Response> {
        self.request(Method::PATCH, url, headers, body).await
    }

    pub async fn delete(&self, url: &str, headers: &HashMap<String, String>) -> Result<Response> {
        self.request(Method::DELETE, url, headers, None).await
    }

    /// POST JSON body; fails if status is not 2xx.
    pub async fn post_json(
        &self,
        url: &str,
        headers: &HashMap<String, String>,
        body: &impl Serialize,
    ) -> Result<()> {
        let payload = serde_json::to_vec(body).context("Failed to serialize JSON body")?;
        let mut headers = headers.clone();
        headers
            .entry("Content-Type".to_string())
            .or_insert_with(|| "application/json".to_string());
        let response = self.post(url, &headers, Some(payload)).await?;
        ensure_success(url, response).await
    }

    /// PUT JSON body; fails if status is not 2xx.
    pub async fn put_json(
        &self,
        url: &str,
        headers: &HashMap<String, String>,
        body: &impl Serialize,
    ) -> Result<()> {
        let payload = serde_json::to_vec(body).context("Failed to serialize JSON body")?;
        let mut headers = headers.clone();
        headers
            .entry("Content-Type".to_string())
            .or_insert_with(|| "application/json".to_string());
        let response = self.put(url, &headers, Some(payload)).await?;
        ensure_success(url, response).await
    }

    /// PATCH JSON body; fails if status is not 2xx.
    pub async fn patch_json(
        &self,
        url: &str,
        headers: &HashMap<String, String>,
        body: &impl Serialize,
    ) -> Result<()> {
        let payload = serde_json::to_vec(body).context("Failed to serialize JSON body")?;
        let mut headers = headers.clone();
        headers
            .entry("Content-Type".to_string())
            .or_insert_with(|| "application/json".to_string());
        let response = self.patch(url, &headers, Some(payload)).await?;
        ensure_success(url, response).await
    }

    /// GET and parse JSON response body.
    pub async fn get_json(&self, url: &str, headers: &HashMap<String, String>) -> Result<Value> {
        let response = self.get(url, headers).await?;
        let status = response.status();
        let text = response
            .text()
            .await
            .with_context(|| format!("HTTP GET {url} read body failed"))?;
        if !status.is_success() {
            anyhow::bail!("HTTP {status} from {url}: {text}");
        }
        serde_json::from_str(&text).with_context(|| format!("HTTP GET {url} invalid JSON"))
    }

    async fn request(
        &self,
        method: Method,
        url: &str,
        headers: &HashMap<String, String>,
        body: Option<Vec<u8>>,
    ) -> Result<Response> {
        let mut req = self.inner.request(method.clone(), url);
        for (name, value) in headers {
            if !name.trim().is_empty() {
                req = req.header(name, value);
            }
        }
        if let Some(bytes) = body {
            req = req.body(bytes);
        }
        req.send()
            .await
            .with_context(|| format!("HTTP {method} {url} failed"))
    }
}

async fn ensure_success(url: &str, response: Response) -> Result<()> {
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }
    let text = response.text().await.unwrap_or_default();
    anyhow::bail!("HTTP {status} from {url}: {text}")
}

struct ExternalHttpLogging;

#[async_trait]
impl Middleware for ExternalHttpLogging {
    async fn handle(
        &self,
        req: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> reqwest_middleware::Result<Response> {
        let method = req.method().clone();
        let url = req.url().clone();
        let bytes = req.body().and_then(|b| b.as_bytes()).unwrap_or_default();
        let request_body = read_body(bytes);

        tracing::info!(
            %method,
            %url,
            body = %request_body,
            "==== EXTERNAL REQUEST INFO ===="
        );

        let started = Instant::now();
        let response = next.run(req, extensions).await?;
        let elapsed_ms = started.elapsed().as_millis();

        let status = response.status();
        let headers = response.headers().clone();
        let version = response.version();
        let bytes = response.bytes().await?;
        let response_body = read_body(&bytes);

        tracing::info!(
            %method,
            %url,
            %status,
            elapsed_ms,
            body = %response_body,
            "==== EXTERNAL RESPONSE INFO ===="
        );

        let mut builder = http::Response::builder().status(status).version(version);
        for (name, value) in headers.iter() {
            builder = builder.header(name, value);
        }
        let http_response = builder
            .body(bytes)
            .map_err(|e| reqwest_middleware::Error::Middleware(e.into()))?;

        Ok(Response::from(http_response))
    }
}

fn read_body(bytes: &[u8]) -> String {
    let full = String::from_utf8_lossy(bytes);
    if full.len() <= MAX_LOG_BODY_BYTES {
        return full.into_owned();
    }
    let mut end = MAX_LOG_BODY_BYTES;
    while end > 0 && !full.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...[truncated,{}B]", &full[..end], bytes.len())
}
