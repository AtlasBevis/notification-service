use crate::models::{ApiRequest, ApiResponse, CreateTemplateRequest, TemplateData};
use crate::services::TemplatesServiceTrait;
use crate::state::SharedState;
use axum::extract::State;

#[utoipa::path(
    post,
    path = "/notification/templates",
    tag = "Templates",
    operation_id = "create_template",
    request_body = CreateTemplateRequest,
    responses(
        (status = 201, description = "Created", body = TemplateData),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 409, description = "Conflict")
    ),
    security(
        ("api-key" = [])
    )
)]
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
