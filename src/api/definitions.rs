use std::sync::Arc;

use crate::models::{
    ApiQuery, ApiRequest, ApiResponse, CreateDefinitionRequest, DefinitionData, PageData,
    PageRequest, PutDefinitionRequest,
};
use crate::services::DefinitionTrait;
use axum::extract::{Path, State};

pub struct DefinitionController {
    svc: Arc<dyn DefinitionTrait>,
}

impl DefinitionController {
    pub fn new(svc: Arc<dyn DefinitionTrait>) -> Self {
        Self { svc: svc }
    }

    pub async fn list_definitions(
        State(this): State<Arc<Self>>,
        ApiQuery(page): ApiQuery<PageRequest>,
    ) -> ApiResponse<PageData<DefinitionData>> {
        if let Err(err) = page.validate() {
            return err.to_response();
        }

        match this.svc.list(page).await {
            Ok(data) => ApiResponse::success(data),
            Err(err) => err.to_response(),
        }
    }

    pub async fn get_definition(
        State(this): State<Arc<Self>>,
        Path(code): Path<String>,
    ) -> ApiResponse<DefinitionData> {
        let key = code.trim().to_ascii_uppercase();
        if key.is_empty() {
            return ApiResponse::bad_request("code is required");
        }

        match this.svc.get_by_code(&key).await {
            Ok(data) => ApiResponse::success(data),
            Err(err) => err.to_response(),
        }
    }

    pub async fn create_definition(
        State(this): State<Arc<Self>>,
        ApiRequest(mut payload): ApiRequest<CreateDefinitionRequest>,
    ) -> ApiResponse<DefinitionData> {
        payload.normalize();
        if let Err(err) = payload.validate() {
            return err.to_response();
        }

        match this.svc.create(payload).await {
            Ok(data) => ApiResponse::created(data),
            Err(err) => err.to_response(),
        }
    }

    pub async fn update_definition(
        State(this): State<Arc<Self>>,
        Path(code): Path<String>,
        ApiRequest(mut payload): ApiRequest<PutDefinitionRequest>,
    ) -> ApiResponse<DefinitionData> {
        let code = code.trim().to_ascii_uppercase();
        if code.is_empty() {
            return ApiResponse::bad_request("code is required");
        }
        payload.normalize();
        if let Err(err) = payload.validate() {
            return err.to_response();
        }

        match this.svc.update(&code, payload).await {
            Ok(data) => ApiResponse::success(data),
            Err(err) => err.to_response(),
        }
    }

    pub async fn delete_definition(
        State(this): State<Arc<Self>>,
        Path(code): Path<String>,
    ) -> ApiResponse {
        let key = code.trim().to_ascii_uppercase();
        if key.is_empty() {
            return ApiResponse::bad_request("code is required");
        }

        match this.svc.delete(&key).await {
            Ok(()) => ApiResponse::ok(),
            Err(err) => err.to_response(),
        }
    }
}
