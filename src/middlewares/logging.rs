use crate::models::ApiResponse as ApiResponse;
use axum::{
    body::Body,
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use http_body_util::BodyExt;
use std::time::Instant;
use tracing::{info_span, Instrument};

use super::RequestContext;

const MAX_LOG_BODY_BYTES: usize = 4096;

fn log_response_body_enabled() -> bool {
    match std::env::var("LOG_RESPONSE_BODY") {
        Ok(v) => {
            let v = v.trim();
            !(v == "0" || v.eq_ignore_ascii_case("false") || v.eq_ignore_ascii_case("off"))
        }
        Err(_) => true,
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

pub async fn logging_middleware(request: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = request.method().clone();
    let path = request.uri().path().to_owned();

    let headers = request.headers();
    let ctx = RequestContext::from_headers(
        headers.get("x-request-id").and_then(|v| v.to_str().ok()),
    );
    let request_id = ctx.request_id.clone();
    let span = info_span!(
        "trace",
        request_id = %request_id,
    );

    async move {
        let (mut parts, body) = request.into_parts();
        let req_bytes = match body.collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(err) => {
                tracing::error!(error = %err, "failed to read request body");
                return ApiResponse::<()>::internal_error().into_response();
            }
        };

        tracing::info!(
            method = %method,
            path = %path,
            body = %read_body(&req_bytes),
            "==== REQUEST INFO ===="
        );

        parts.extensions.insert(ctx);
        let request = Request::from_parts(parts, Body::from(req_bytes));

        let response = next.run(request).await;
        let status = response.status().as_u16();
        let (parts, body) = response.into_parts();
        let res_bytes = match body.collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(err) => {
                tracing::error!(
                    status,
                    error = %err,
                    "failed to read response body"
                );
                return Response::from_parts(parts, Body::empty());
            }
        };

        let duration_ms = start.elapsed().as_millis();
        if log_response_body_enabled() {
            tracing::info!(
                status,
                duration_ms,
                body = %read_body(&res_bytes),
                "==== RESPONSE INFO ===="
            );
        } else {
            tracing::info!(status, duration_ms, "==== RESPONSE INFO ====");
        }

        Response::from_parts(parts, Body::from(res_bytes))
    }
    .instrument(span)
    .await
}
