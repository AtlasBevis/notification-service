use std::sync::Arc;

use crate::models::{
    ApiRequest, ApiResponse, CreateTargetRequest, PatchMetadataRequest, TargetData,
};
use crate::services::TargetTrait;
use axum::extract::{Path, State};

pub struct TargetController {
    svc: Arc<dyn TargetTrait>,
}

impl TargetController {
    pub fn new(svc: Arc<dyn TargetTrait>) -> Self {
        Self { svc: svc }
    }

    pub async fn get_target(
        State(this): State<Arc<Self>>,
        Path(code): Path<String>,
    ) -> ApiResponse<TargetData> {
        let key = code.trim().to_ascii_uppercase();
        if key.is_empty() {
            return ApiResponse::bad_request("code is required");
        }

        match this.svc.get_by_code(&key).await {
            Ok(data) => ApiResponse::success(data),
            Err(err) => err.to_response(),
        }
    }

    pub async fn create_target(
        State(this): State<Arc<Self>>,
        ApiRequest(mut payload): ApiRequest<CreateTargetRequest>,
    ) -> ApiResponse {
        payload.normalize();
        if let Err(err) = payload.validate() {
            return err.to_response();
        }

        match this.svc.create(payload).await {
            Ok(()) => ApiResponse::created(()),
            Err(err) => err.to_response(),
        }
    }

    pub async fn patch_metadata(
        State(this): State<Arc<Self>>,
        ApiRequest(mut payload): ApiRequest<PatchMetadataRequest>,
    ) -> ApiResponse {
        payload.normalize();
        if let Err(err) = payload.validate() {
            return err.to_response();
        }

        match this.svc.patch_metadata(payload).await {
            Ok(()) => ApiResponse::ok(),
            Err(err) => err.to_response(),
        }
    }
}
