use std::sync::Arc;

use tonic::{Request, Response, Status};

use super::convert::ProtoConvert;
use super::pb::templates_server::Templates;
use super::pb::{SaveTemplateRequest as SaveTemplateRpcRequest, Template};
use crate::enums::TemplateFormat;
use crate::error::AppError;
use crate::models::{SaveTemplateRequest, TemplateData};
use crate::services::TemplateTrait;

/// gRPC counterpart of `TemplateController`.
pub struct TemplateGrpc {
    svc: Arc<dyn TemplateTrait>,
}

impl TemplateGrpc {
    pub fn new(svc: Arc<dyn TemplateTrait>) -> Self {
        Self { svc }
    }

    fn to_reply(data: TemplateData) -> Template {
        Template {
            id: data.id,
            code: data.code,
            name: data.name,
            status: data.status.into(),
            version_id: data.version_id,
            version: data.version,
            version_status: data.version_status.into(),
            title: data.title,
            content: data.content,
            format: data.format.into(),
            created_at: Some(ProtoConvert::timestamp(data.created_at)),
            updated_at: data.updated_at.map(ProtoConvert::timestamp),
        }
    }
}

#[tonic::async_trait]
impl Templates for TemplateGrpc {
    async fn save_template(
        &self,
        request: Request<SaveTemplateRpcRequest>,
    ) -> Result<Response<Template>, Status> {
        let request = request.into_inner();
        // The HTTP body rejects an unknown `format` while deserializing; do the same here.
        let format = TemplateFormat::parse(&request.format.trim().to_ascii_uppercase())
            .ok_or_else(|| {
                AppError::bad_request("format must be TEXT, HTML, ADAPTIVE_CARD or JSON")
            })
            .map_err(ProtoConvert::status)?;
        let mut payload = SaveTemplateRequest {
            code: request.code,
            name: request.name,
            title: request.title,
            content: request.content,
            format,
        };
        payload.normalize();
        payload.validate().map_err(ProtoConvert::status)?;

        let data = self
            .svc
            .create(payload)
            .await
            .map_err(ProtoConvert::status)?;
        Ok(Response::new(Self::to_reply(data)))
    }
}
