use crate::error::{AppError, AppResult};
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
) -> AppResult<ApiResponse<TemplateData>> {
    payload.validate().map_err(AppError::BadRequest)?;
    let data = state.templates_service.create(payload).await?;
    Ok(ApiResponse::created(data))
}
