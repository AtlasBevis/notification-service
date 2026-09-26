use crate::error::{AppError, AppResult};
use crate::models::{
    ApiRequest, ApiResponse, CreateNotificationRequest, NotificationData, UpdateNotificationRequest,
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
        (status = 200, description = "OK", body = Vec<NotificationData>),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("api-key" = [])
    )
)]
pub async fn list_notifications(
    State(state): State<SharedState>,
) -> AppResult<ApiResponse<Vec<NotificationData>>> {
    let data = state.notification_service.list().await?;
    Ok(ApiResponse::success(data))
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
) -> AppResult<ApiResponse<NotificationData>> {
    payload.normalize();
    payload.validate().map_err(AppError::BadRequest)?;
    let data = state.notification_service.create(payload).await?;
    Ok(ApiResponse::created(data))
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
) -> AppResult<ApiResponse<NotificationData>> {
    let key = code.trim().to_ascii_uppercase();
    if key.is_empty() {
        return Err(AppError::bad_request("code is required"));
    }
    let data = state.notification_service.get_by_code(&key).await?;
    Ok(ApiResponse::success(data))
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
) -> AppResult<ApiResponse<NotificationData>> {
    let key = code.trim().to_ascii_uppercase();
    if key.is_empty() {
        return Err(AppError::bad_request("code is required"));
    }
    payload.normalize();
    payload.validate().map_err(AppError::BadRequest)?;
    let data = state.notification_service.update(&key, payload).await?;
    Ok(ApiResponse::success(data))
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
) -> AppResult<ApiResponse<()>> {
    let key = code.trim().to_ascii_uppercase();
    if key.is_empty() {
        return Err(AppError::bad_request("code is required"));
    }
    state.notification_service.delete(&key).await?;
    Ok(ApiResponse::ok())
}
