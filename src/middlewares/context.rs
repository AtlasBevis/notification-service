use axum::{
    body::Body,
    extract::Request,
    http::HeaderMap,
    middleware::Next,
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use http_body_util::BodyExt;
use tracing::{info_span, Instrument};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

// header keys
const X_API_KEY: &str = "x-api-key";
const X_REQUEST_ID: &str = "x-request-id";

#[derive(Clone, Debug)]
pub struct BufferedBody(pub Bytes);

#[derive(Debug, Clone)]
pub struct RequestHeaders {
    pub request_id: String,
    pub api_key: String,
}

#[derive(Debug, Clone)]
pub struct RequestContext {
    pub headers: RequestHeaders,
    pub path: String,
    pub method: String,
    pub body: BufferedBody,
}

impl RequestContext {
    /// Buffers the request body, stores the context in the request extensions,
    /// and returns the context with a rebuilt request.
    pub async fn from_request(request: Request) -> AppResult<(Self, Request)> {
        let method = request.method().as_str().to_owned();
        let path = request.uri().path().to_owned();
        let headers = Self::parse_headers(request.headers());

        let (mut parts, body) = request.into_parts();
        let bytes = body
            .collect()
            .await
            .map_err(|err| AppError::internal_with("failed to read request body", err))?
            .to_bytes();

        let ctx = Self {
            headers,
            path,
            method,
            body: BufferedBody(bytes.clone()),
        };
        parts.extensions.insert(ctx.clone());
        Ok((ctx, Request::from_parts(parts, Body::from(bytes))))
    }

    pub async fn middleware(request: Request, next: Next) -> Response {
        let (ctx, request) = match Self::from_request(request).await {
            Ok(pair) => pair,
            Err(err) => {
                tracing::error!(error = %err, "failed to build request context");
                return err.into_response();
            }
        };
        let span = info_span!(
            "trace",
            request_id = %ctx.headers.request_id,
        );
        next.run(request).instrument(span).await
    }

    fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    }

    fn parse_headers(headers: &HeaderMap) -> RequestHeaders {
        RequestHeaders {
            api_key: Self::header_value(headers, X_API_KEY).unwrap_or_else(String::new),
            request_id: Self::header_value(headers, X_REQUEST_ID)
                .unwrap_or_else(|| Uuid::new_v4().to_string()),
        }
    }
}

pub async fn request_context_middleware(request: Request, next: Next) -> Response {
    RequestContext::middleware(request, next).await
}
