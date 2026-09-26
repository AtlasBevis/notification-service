use crate::models::{ApiRequest, ApiResponse, CreateTargetRequest, PatchMetadataRequest, TargetData};
use crate::services::TargetsServiceTrait;
use crate::state::SharedState;
use axum::extract::{Path, State};

#[utoipa::path(
    get,
    path = "/notification/targets/{code}",
    tag = "Targets",
    operation_id = "get_target",
    params(
        ("code" = String, Path, description = "Target code", example = "TEAM_MIS")
    ),
    responses(
        (status = 200, description = "OK", body = TargetData),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found")
    ),
    security(
        ("api-key" = [])
    )
)]
pub async fn get_target(
    State(state): State<SharedState>,
    Path(code): Path<String>,
) -> ApiResponse<TargetData> {
    let key = code.trim().to_ascii_uppercase();
    if key.is_empty() {
        return ApiResponse::bad_request("code is required");
    }

    match state.targets_service.get_by_code(&key).await {
        Ok(Some(data)) => ApiResponse::success(data),
        Ok(None) => ApiResponse::not_found(format!("target `{key}` not found")),
        Err(err) => err.to_response(),
    }
}

#[utoipa::path(
    post,
    path = "/notification/targets",
    tag = "Targets",
    operation_id = "create_target",
    request_body = CreateTargetRequest,
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
pub async fn create_target(
    State(state): State<SharedState>,
    ApiRequest(mut payload): ApiRequest<CreateTargetRequest>,
) -> ApiResponse {
    payload.normalize();
    if let Err(msg) = payload.validate() {
        return ApiResponse::bad_request(msg);
    }

    match state.targets_service.create(payload).await {
        Ok(()) => ApiResponse::created(()),
        Err(err) => err.to_response(),
    }
}

#[utoipa::path(
    patch,
    path = "/notification/targets/metadata",
    tag = "Targets",
    operation_id = "patch_target_metadata",
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

    match state.targets_service.patch_metadata(payload).await {
        Ok(()) => ApiResponse::ok(),
        Err(err) => err.to_response(),
    }
}
