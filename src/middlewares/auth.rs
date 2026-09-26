use crate::config;
use crate::models::ApiResponse;
use axum::response::IntoResponse;
use axum::{
    extract::Request, http::HeaderMap, middleware::Next, response::Response as AxumResponse,
};

const API_KEY_HEADER: &str = "api-key";

pub async fn auth_middleware(headers: HeaderMap, request: Request, next: Next) -> AxumResponse {
    let config = config::load();

    let Some(auth) = &config.auth else {
        return next.run(request).await;
    };
    if !auth.enabled {
        return next.run(request).await;
    }

    let Some(provided) = headers
        .get(API_KEY_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return ApiResponse::unauthorized("Missing api-key").into_response();
    };

    if !auth.matches(provided) {
        return ApiResponse::unauthorized("Invalid api-key").into_response();
    }

    next.run(request).await
}
