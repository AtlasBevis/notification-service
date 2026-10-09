use crate::models::{ApiRequest, ApiResponse, CreateTargetRequest, PatchMetadataRequest, TargetData};
use crate::services::TargetsServiceTrait;
use crate::state::SharedState;
use axum::extract::{Path, State};

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
