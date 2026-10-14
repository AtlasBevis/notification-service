use std::sync::Arc;

use crate::models::{ApiRequest, ApiResponse, NotifyRequest};
use crate::services::NotifyTrait;
use axum::extract::State;

pub struct NotifyController {
    svc: Arc<dyn NotifyTrait>,
}

impl NotifyController {
    pub fn new(svc: Arc<dyn NotifyTrait>) -> Self {
        Self { svc: svc }
    }

    /// Trigger send for a notification
    pub async fn notify(
        State(this): State<Arc<Self>>,
        ApiRequest(mut payload): ApiRequest<NotifyRequest>,
    ) -> ApiResponse {
        payload.normalize();
        if let Err(err) = payload.validate() {
            return err.to_response();
        }

        let svc = this.svc.clone();
        let message = match svc.validate(payload).await {
            Ok(message) => message,
            Err(err) => return err.to_response(),
        };

        if let Err(err) = svc.notify(message).await {
            return err.to_response();
        }

        ApiResponse::accepted()
    }
}
