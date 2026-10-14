use std::sync::Arc;

use crate::models::{ApiRequest, ApiResponse, SaveTemplateRequest, TemplateData};
use crate::services::TemplateTrait;
use axum::extract::State;

pub struct TemplateController {
    svc: Arc<dyn TemplateTrait>,
}

impl TemplateController {
    pub fn new(svc: Arc<dyn TemplateTrait>) -> Self {
        Self { svc: svc }
    }

    pub async fn save_template(
        State(this): State<Arc<Self>>,
        ApiRequest(mut payload): ApiRequest<SaveTemplateRequest>,
    ) -> ApiResponse<TemplateData> {
        payload.normalize();
        if let Err(err) = payload.validate() {
            return err.to_response();
        }

        match this.svc.create(payload).await {
            Ok(data) => ApiResponse::created(data),
            Err(err) => err.to_response(),
        }
    }
}
