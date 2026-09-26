use crate::models::{
    ApiRequest, ApiResponse, CreateNotificationRequest, NotificationData, PageData,
    UpdateNotificationRequest,
};
use crate::services::NotificationServiceTrait;
use crate::state::SharedState;
use axum::extract::{Path, State};

#[utoipa::path(
    get,
    path = "/notification",
    tag = "Notification",
    operation_id = "list_notifications",
    responses(
        (status = 200, description = "OK", body = PageData<NotificationData>),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("api-key" = [])
    )
)]
pub async fn list_notifications(
    State(state): State<SharedState>,
) -> ApiResponse<PageData<NotificationData>> {
    match state.notification_service.list().await {
        Ok(data) => ApiResponse::success(PageData::from_items(data)),
        Err(err) => err.to_response(),
    }
}

#[utoipa::path(
    post,
    path = "/notification",
    tag = "Notification",
    operation_id = "create_notification",
    request_body = CreateNotificationRequest,
    responses(
        (status = 201, description = "Created", body = NotificationData),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 409, description = "Conflict")
    ),
    security(
        ("api-key" = [])
    )
)]
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

#[utoipa::path(
    get,
    path = "/notification/{code}",
    tag = "Notification",
    operation_id = "get_notification",
    params(
        ("code" = String, Path, description = "Notification code", example = "AIRFLOW_FAIL")
    ),
    responses(
        (status = 200, description = "OK", body = NotificationData),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found")
    ),
    security(
        ("api-key" = [])
    )
)]
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

#[utoipa::path(
    put,
    path = "/notification/{code}",
    tag = "Notification",
    operation_id = "update_notification",
    params(
        ("code" = String, Path, description = "Notification code", example = "AIRFLOW_FAIL")
    ),
    request_body = UpdateNotificationRequest,
    responses(
        (status = 200, description = "OK", body = NotificationData),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found")
    ),
    security(
        ("api-key" = [])
    )
)]
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

#[utoipa::path(
    delete,
    path = "/notification/{code}",
    tag = "Notification",
    operation_id = "delete_notification",
    params(
        ("code" = String, Path, description = "Notification code", example = "AIRFLOW_FAIL")
    ),
    responses(
        (status = 200, description = "OK"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Conflict")
    ),
    security(
        ("api-key" = [])
    )
)]
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
