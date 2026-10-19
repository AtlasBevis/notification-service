use std::sync::Arc;

use tonic::{Request, Response, Status};

use super::convert::ProtoConvert;
use super::pb::definitions_server::Definitions;
use super::pb::{
    CodeRequest, Definition, DefinitionList, DefinitionTarget, PageRequest as PageRpcRequest,
    SaveDefinitionRequest as SaveDefinitionRpcRequest,
};
use crate::models::{DefinitionData, DefinitionTargetRequest, PageRequest, SaveDefinitionRequest};
use crate::services::DefinitionTrait;

/// gRPC counterpart of `DefinitionController`.
pub struct DefinitionGrpc {
    svc: Arc<dyn DefinitionTrait>,
}

impl DefinitionGrpc {
    pub fn new(svc: Arc<dyn DefinitionTrait>) -> Self {
        Self { svc }
    }

    fn to_request(request: SaveDefinitionRpcRequest) -> SaveDefinitionRequest {
        SaveDefinitionRequest {
            code: request.code,
            name: request.name,
            description: request.description,
            status: request.status,
            level: request.level,
            source_code: request.source_code,
            channel_code: request.channel_code,
            targets: request
                .targets
                .into_iter()
                .map(|t| DefinitionTargetRequest {
                    target_code: t.target_code,
                    template_id: t.template_id,
                })
                .collect(),
        }
    }

    fn to_reply(data: DefinitionData) -> Definition {
        Definition {
            id: data.id,
            code: data.code,
            name: data.name,
            description: data.description,
            status: data.status.into(),
            level: data.level.into(),
            source_id: data.source_id,
            channel_id: data.channel_id,
            targets: data
                .targets
                .into_iter()
                .map(|t| DefinitionTarget {
                    target_id: t.target_id,
                    template_id: t.template_id,
                })
                .collect(),
            created_at: Some(ProtoConvert::timestamp(data.created_at)),
            updated_at: data.updated_at.map(ProtoConvert::timestamp),
        }
    }
}

#[tonic::async_trait]
impl Definitions for DefinitionGrpc {
    async fn list_definitions(
        &self,
        request: Request<PageRpcRequest>,
    ) -> Result<Response<DefinitionList>, Status> {
        let request = request.into_inner();
        let page = PageRequest {
            page: request.page,
            size: request.size,
        };
        page.validate().map_err(ProtoConvert::status)?;

        let data = self.svc.list(page).await.map_err(ProtoConvert::status)?;
        Ok(Response::new(DefinitionList {
            total: data.total,
            items: data.items.into_iter().map(Self::to_reply).collect(),
        }))
    }

    async fn get_definition(
        &self,
        request: Request<CodeRequest>,
    ) -> Result<Response<Definition>, Status> {
        let key =
            ProtoConvert::code(&request.into_inner().code, "code").map_err(ProtoConvert::status)?;
        let data = self
            .svc
            .get_by_code(&key)
            .await
            .map_err(ProtoConvert::status)?;
        Ok(Response::new(Self::to_reply(data)))
    }

    async fn create_definition(
        &self,
        request: Request<SaveDefinitionRpcRequest>,
    ) -> Result<Response<Definition>, Status> {
        let mut payload = Self::to_request(request.into_inner());
        payload.normalize();
        payload.validate().map_err(ProtoConvert::status)?;

        let data = self
            .svc
            .create(payload)
            .await
            .map_err(ProtoConvert::status)?;
        Ok(Response::new(Self::to_reply(data)))
    }

    async fn update_definition(
        &self,
        request: Request<SaveDefinitionRpcRequest>,
    ) -> Result<Response<Definition>, Status> {
        let mut payload = Self::to_request(request.into_inner());
        payload.normalize();
        payload.validate().map_err(ProtoConvert::status)?;

        let data = self
            .svc
            .update(payload)
            .await
            .map_err(ProtoConvert::status)?;
        Ok(Response::new(Self::to_reply(data)))
    }

    async fn delete_definition(
        &self,
        request: Request<CodeRequest>,
    ) -> Result<Response<()>, Status> {
        let key =
            ProtoConvert::code(&request.into_inner().code, "code").map_err(ProtoConvert::status)?;
        self.svc.delete(&key).await.map_err(ProtoConvert::status)?;
        Ok(Response::new(()))
    }
}
