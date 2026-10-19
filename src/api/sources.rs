use std::sync::Arc;

use crate::models::{
    ApiQuery, ApiRequest, ApiResponse, PageData, PageRequest, SourceData, SourceRequest,
};
use crate::services::SourceTrait;
use axum::extract::{Path, State};

pub struct SourceController {
    svc: Arc<dyn SourceTrait>,
}

impl SourceController {
    pub fn new(svc: Arc<dyn SourceTrait>) -> Self {
        Self { svc: svc }
    }

    pub async fn list_sources(
        State(this): State<Arc<Self>>,
        ApiQuery(page): ApiQuery<PageRequest>,
    ) -> ApiResponse<PageData<SourceData>> {
        if let Err(err) = page.validate() {
            return err.to_response();
        }

        match this.svc.list(page).await {
            Ok(data) => ApiResponse::success(data),
            Err(err) => err.to_response(),
        }
    }

    pub async fn get_source(
        State(this): State<Arc<Self>>,
        Path(code): Path<String>,
    ) -> ApiResponse<SourceData> {
        let key = code.trim().to_ascii_uppercase();
        if key.is_empty() {
            return ApiResponse::bad_request("code is required");
        }

        match this.svc.get_by_code(&key).await {
            Ok(data) => ApiResponse::success(data),
            Err(err) => err.to_response(),
        }
    }

    pub async fn create_source(
        State(this): State<Arc<Self>>,
        ApiRequest(mut payload): ApiRequest<SourceRequest>,
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

    pub async fn update_source(
        State(this): State<Arc<Self>>,
        ApiRequest(mut payload): ApiRequest<SourceRequest>,
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
