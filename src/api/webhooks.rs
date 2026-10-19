use std::sync::Arc;

use crate::models::{ApiRequest, ApiResponse, PageData, SaveWebhookRequest, WebhookData};
use crate::services::WebhookServiceTrait;
use axum::extract::{Path, State};

/// Webhook configs are addressed by their target code (one config per target).
pub struct WebhookController {
    svc: Arc<dyn WebhookServiceTrait>,
}

impl WebhookController {
    pub fn new(svc: Arc<dyn WebhookServiceTrait>) -> Self {
        Self { svc: svc }
    }

    pub async fn list_webhooks(
        State(this): State<Arc<Self>>,
    ) -> ApiResponse<PageData<WebhookData>> {
        match this.svc.list().await {
            Ok(data) => ApiResponse::success(data),
            Err(err) => err.to_response(),
        }
    }

    pub async fn get_webhook(
        State(this): State<Arc<Self>>,
        Path(target_code): Path<String>,
    ) -> ApiResponse<WebhookData> {
        let key = target_code.trim().to_ascii_uppercase();
        if key.is_empty() {
            return ApiResponse::bad_request("target_code is required");
        }

        match this.svc.get_by_target_code(&key).await {
            Ok(data) => ApiResponse::success(data),
            Err(err) => err.to_response(),
        }
    }

    pub async fn create_webhook(
        State(this): State<Arc<Self>>,
        ApiRequest(mut payload): ApiRequest<SaveWebhookRequest>,
    ) -> ApiResponse<WebhookData> {
        payload.normalize();
        if let Err(err) = payload.validate() {
            return err.to_response();
        }

        match this.svc.create(payload).await {
            Ok(data) => ApiResponse::created(data),
            Err(err) => err.to_response(),
        }
    }

    /// The path `target_code` selects the webhook; a `target_code` in the body is ignored.
    pub async fn update_webhook(
        State(this): State<Arc<Self>>,
        Path(target_code): Path<String>,
        ApiRequest(mut payload): ApiRequest<SaveWebhookRequest>,
    ) -> ApiResponse<WebhookData> {
        payload.target_code = target_code;
        payload.normalize();
        if let Err(err) = payload.validate() {
            return err.to_response();
        }

        match this.svc.update(payload).await {
            Ok(data) => ApiResponse::success(data),
            Err(err) => err.to_response(),
        }
    }

    pub async fn delete_webhook(
        State(this): State<Arc<Self>>,
        Path(target_code): Path<String>,
    ) -> ApiResponse {
        let key = target_code.trim().to_ascii_uppercase();
        if key.is_empty() {
            return ApiResponse::bad_request("target_code is required");
        }

        match this.svc.delete(&key).await {
            Ok(()) => ApiResponse::ok(),
            Err(err) => err.to_response(),
        }
    }
}
