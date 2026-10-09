use crate::models::{ApiRequest, ApiResponse, CreateTemplateRequest, TemplateData};
use crate::services::TemplatesServiceTrait;
use crate::state::SharedState;
use axum::extract::State;

pub async fn create_template(
    State(state): State<SharedState>,
    ApiRequest(payload): ApiRequest<CreateTemplateRequest>,
) -> ApiResponse<TemplateData> {
    if let Err(msg) = payload.validate() {
        return ApiResponse::bad_request(msg);
    }

    match state.templates_service.create(payload).await {
        Ok(data) => ApiResponse::created(data),
        Err(err) => err.to_response(),
    }
}
