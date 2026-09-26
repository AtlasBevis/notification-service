use crate::error::{AppError, AppResult};
use crate::models::{ApiRequest, ApiResponse, CreateSourceRequest, PatchMetadataRequest, SourceData};
use crate::services::SourcesServiceTrait;
use crate::state::SharedState;
use axum::extract::{Path, State};

#[utoipa::path(
    get,
    path = "/notification/sources/{code}",
    tag = "Sources",
    operation_id = "get_source",
    params(
        ("code" = String, Path, description = "Source code", example = "AIRFLOW")
    ),
    responses(
        (status = 200, description = "OK", body = SourceData),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found")
    ),
    security(
        ("api-key" = [])
    )
)]
pub async fn get_source(
    State(state): State<SharedState>,
    Path(code): Path<String>,
) -> AppResult<ApiResponse<SourceData>> {
    let key = code.trim().to_ascii_uppercase();
    if key.is_empty() {
        return Err(AppError::bad_request("code is required"));
    }
    let data = state
        .sources_service
        .get_by_code(&key)
        .await?
        .ok_or_else(|| AppError::not_found(format!("source `{key}` not found")))?;
    Ok(ApiResponse::success(data))
}

#[utoipa::path(
    post,
    path = "/notification/sources",
    tag = "Sources",
    operation_id = "create_source",
    request_body = CreateSourceRequest,
    responses(
        (status = 201, description = "Created"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 409, description = "Conflict")
    ),
    security(
        ("api-key" = [])
    )
)]
pub async fn create_source(
    State(state): State<SharedState>,
    ApiRequest(mut payload): ApiRequest<CreateSourceRequest>,
) -> AppResult<ApiResponse<()>> {
    payload.normalize();
    payload.validate().map_err(AppError::BadRequest)?;
    state.sources_service.create(payload).await?;
    Ok(ApiResponse::created(()))
}

#[utoipa::path(
    patch,
    path = "/notification/sources/metadata",
    tag = "Sources",
    operation_id = "patch_source_metadata",
    request_body = PatchMetadataRequest,
    responses(
        (status = 200, description = "OK"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found")
    ),
    security(
        ("api-key" = [])
    )
)]
pub async fn patch_metadata(
    State(state): State<SharedState>,
    ApiRequest(mut payload): ApiRequest<PatchMetadataRequest>,
) -> AppResult<ApiResponse<()>> {
    payload.normalize();
    payload.validate().map_err(AppError::BadRequest)?;
    state.sources_service.patch_metadata(payload).await?;
    Ok(ApiResponse::ok())
}
