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
) -> ApiResponse<SourceData> {
    let key = code.trim().to_ascii_uppercase();
    if key.is_empty() {
        return ApiResponse::bad_request("code is required");
    }

    match state.sources_service.get_by_code(&key).await {
        Ok(Some(data)) => ApiResponse::success(data),
        Ok(None) => ApiResponse::not_found(format!("source `{key}` not found")),
        Err(err) => err.to_response(),
    }
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
) -> ApiResponse {
    payload.normalize();
    if let Err(msg) = payload.validate() {
        return ApiResponse::bad_request(msg);
    }

    match state.sources_service.create(payload).await {
        Ok(()) => ApiResponse::created(()),
        Err(err) => err.to_response(),
    }
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
) -> ApiResponse {
    payload.normalize();
    if let Err(msg) = payload.validate() {
        return ApiResponse::bad_request(msg);
    }

    match state.sources_service.patch_metadata(payload).await {
        Ok(()) => ApiResponse::ok(),
        Err(err) => err.to_response(),
    }
}
