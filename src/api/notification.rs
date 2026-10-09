use crate::models::{
    ApiRequest, ApiResponse, CreateNotificationRequest, NotificationData, PageData,
    UpdateNotificationRequest,
};
use crate::services::NotificationServiceTrait;
use crate::state::SharedState;
use axum::extract::{Path, State};

pub async fn list_notifications(
    State(state): State<SharedState>,
) -> ApiResponse<PageData<NotificationData>> {
    match state.notification_service.list().await {
        Ok(data) => ApiResponse::success(PageData::from_items(data)),
        Err(err) => err.to_response(),
    }
}

pub async fn create_notification(
    State(state): State<SharedState>,
    ApiRequest(mut payload): ApiRequest<CreateNotificationRequest>,
) -> ApiResponse<NotificationData> {
    payload.normalize();
    if let Err(msg) = payload.validate() {
        return ApiResponse::bad_request(msg);
    }

    match state.notification_service.create(payload).await {
        Ok(data) => ApiResponse::created(data),
        Err(err) => err.to_response(),
    }
}

pub async fn get_notification(
    State(state): State<SharedState>,
    Path(code): Path<String>,
) -> ApiResponse<NotificationData> {
    let key = code.trim().to_ascii_uppercase();
    if key.is_empty() {
        return ApiResponse::bad_request("code is required");
    }

    match state.notification_service.get_by_code(&key).await {
        Ok(data) => ApiResponse::success(data),
        Err(err) => err.to_response(),
    }
}

pub async fn update_notification(
    State(state): State<SharedState>,
    Path(code): Path<String>,
    ApiRequest(mut payload): ApiRequest<UpdateNotificationRequest>,
) -> ApiResponse<NotificationData> {
    let key = code.trim().to_ascii_uppercase();
    if key.is_empty() {
        return ApiResponse::bad_request("code is required");
    }

    payload.normalize();
    if let Err(msg) = payload.validate() {
        return ApiResponse::bad_request(msg);
    }

    match state.notification_service.update(&key, payload).await {
        Ok(data) => ApiResponse::success(data),
        Err(err) => err.to_response(),
    }
}

pub async fn delete_notification(
    State(state): State<SharedState>,
    Path(code): Path<String>,
) -> ApiResponse {
    let key = code.trim().to_ascii_uppercase();
    if key.is_empty() {
        return ApiResponse::bad_request("code is required");
    }

    match state.notification_service.delete(&key).await {
        Ok(()) => ApiResponse::ok(),
        Err(err) => err.to_response(),
    }
}
