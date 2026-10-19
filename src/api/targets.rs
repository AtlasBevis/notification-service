use std::sync::Arc;

use crate::models::{
    ApiQuery, ApiRequest, ApiResponse, PageData, PageRequest, TargetData, TargetRequest,
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

    pub async fn list_targets(
        State(this): State<Arc<Self>>,
        ApiQuery(page): ApiQuery<PageRequest>,
    ) -> ApiResponse<PageData<TargetData>> {
        if let Err(err) = page.validate() {
            return err.to_response();
        }

        match this.svc.list(page).await {
            Ok(data) => ApiResponse::success(data),
            Err(err) => err.to_response(),
        }
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
        ApiRequest(mut payload): ApiRequest<TargetRequest>,
    ) -> ApiResponse {
        payload.normalize();
        if let Err(err) = payload.validate(true) {
            return err.to_response();
        }

        match this.svc.create(payload).await {
            Ok(()) => ApiResponse::created(()),
            Err(err) => err.to_response(),
        }
    }

    pub async fn update_target(
        State(this): State<Arc<Self>>,
        ApiRequest(mut payload): ApiRequest<TargetRequest>,
    ) -> ApiResponse {
        payload.normalize();
        if let Err(err) = payload.validate(false) {
            return err.to_response();
        }

        match this.svc.update(payload).await {
            Ok(()) => ApiResponse::ok(),
            Err(err) => err.to_response(),
        }
    }
}
