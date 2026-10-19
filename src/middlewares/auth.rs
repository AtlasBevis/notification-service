use crate::config::Config;
use crate::models::ApiResponse;
use axum::response::IntoResponse;
use axum::{extract::Request, middleware::Next, response::Response as AxumResponse};

use super::RequestContext;

pub async fn auth_middleware(request: Request, next: Next) -> AxumResponse {
    let config = Config::load();
    if !config.auth.is_enabled() {
        return next.run(request).await;
    }

    let Some(ctx) = request.extensions().get::<RequestContext>() else {
        tracing::error!(path = %request.uri().path(), "Missing Request Context");
        return ApiResponse::<()>::bad_request("Unknown request").into_response();
    };

    let provided = ctx.headers.api_key.as_str();
    if provided.is_empty() {
        return ApiResponse::unauthorized("Missing x-api-key").into_response();
    }

    // is admin
    if config.auth.is_admin(provided) {
        return next.run(request).await;
    }

    if !config.auth.matches(provided) {
        return ApiResponse::unauthorized("Invalid x-api-key").into_response();
    }

    next.run(request).await
}
