use crate::models::{ApiRequest, ApiResponse, NotifyRequest};
use crate::services::NotifyServiceTrait;
use crate::state::SharedState;
use axum::extract::State;

/// Trigger send for a notification
#[utoipa::path(
    post,
    path = "/notification/notify",
    tag = "Notify",
    operation_id = "notification_notify",
    request_body = NotifyRequest,
    responses(
        (status = 202, description = "Accepted"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Conflict"),
        (status = 503, description = "Service unavailable")
    ),
    security(
        ("api-key" = [])
    )
)]
pub async fn notify(
    State(state): State<SharedState>,
    ApiRequest(mut payload): ApiRequest<NotifyRequest>,
) -> ApiResponse {
    payload.normalize();
    if let Err(err) = payload.validate() {
        return err.to_response();
    }

    let svc = state.notify_service.clone();
    if let Err(err) = svc.validate(payload.clone()).await {
        return err.to_response();
    }

    if let Err(err) = svc.notify(payload).await {
        return err.to_response();
    }

    ApiResponse::accepted()
}
