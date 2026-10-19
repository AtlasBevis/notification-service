use crate::models::ApiResponse;
use axum::{
    body::Body,
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use http_body_util::BodyExt;
use std::time::Instant;

use super::RequestContext;
use crate::utils::StringUtils;

const MAX_LOG_BODY_BYTES: usize = 4096;

fn log_response_body_enabled() -> bool {
    match StringUtils::get_env("LOG_RESPONSE_BODY") {
        Some(v) => !(v == "0" || v.eq_ignore_ascii_case("false") || v.eq_ignore_ascii_case("off")),
        None => true,
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
    let Some(ctx) = request.extensions().get::<RequestContext>().cloned() else {
        return ApiResponse::<()>::bad_request("Unknown request").into_response();
    };
    let method = ctx.method;
    let path = ctx.path;
    let req_bytes = ctx.body.0;

    async move {
        tracing::info!(
            method = %method,
            path = %path,
            body = %if path.contains("/webhooks") {
                "<redacted: webhook config may contain secrets>".to_string()
            } else {
                read_body(&req_bytes)
            },
            "==== REQUEST INFO ===="
        );

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
    .await
}
