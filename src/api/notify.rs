use crate::models::{ApiRequest, ApiResponse, NotifyRequest};
use crate::services::NotifyServiceTrait;
use crate::state::SharedState;
use axum::extract::State;

/// Trigger send for a notification
pub async fn notify(
    State(state): State<SharedState>,
    ApiRequest(mut payload): ApiRequest<NotifyRequest>,
) -> ApiResponse {
    payload.normalize();
    if let Err(err) = payload.validate() {
        return err.to_response();
    }

    let svc = state.notify_service.clone();
    let message = match svc.validate(payload).await {
        Ok(message) => message,
        Err(err) => return err.to_response(),
    };

    if let Err(err) = svc.notify(message).await {
        return err.to_response();
    }

    ApiResponse::accepted()
}
