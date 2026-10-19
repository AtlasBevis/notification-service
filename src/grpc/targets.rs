use std::sync::Arc;

use tonic::{Request, Response, Status};

use super::convert::ProtoConvert;
use super::pb::targets_server::Targets;
use super::pb::{
    CodeRequest, CreateTargetRequest as CreateTargetRpcRequest, PageRequest as PageRpcRequest,
    PutTargetRequest as PutTargetRpcRequest, Target, TargetList,
};
use crate::models::{CreateTargetRequest, PageRequest, PutTargetRequest, TargetData};
use crate::services::TargetTrait;

/// gRPC counterpart of `TargetController`.
pub struct TargetGrpc {
    svc: Arc<dyn TargetTrait>,
}

impl TargetGrpc {
    pub fn new(svc: Arc<dyn TargetTrait>) -> Self {
        Self { svc }
    }

    fn to_reply(data: TargetData) -> Target {
        Target {
            id: data.id,
            code: data.code,
            channel_code: data.channel.into(),
            name: data.name,
            status: data.status.into(),
            metadata: ProtoConvert::to_struct(&data.metadata),
            created_at: Some(ProtoConvert::timestamp(data.created_at)),
            updated_at: data.updated_at.map(ProtoConvert::timestamp),
        }
    }
}

#[tonic::async_trait]
impl Targets for TargetGrpc {
    async fn list_targets(
        &self,
        request: Request<PageRpcRequest>,
    ) -> Result<Response<TargetList>, Status> {
        let request = request.into_inner();
        let page = PageRequest {
            page: request.page,
            size: request.size,
        };
        page.validate().map_err(ProtoConvert::status)?;

        let data = self.svc.list(page).await.map_err(ProtoConvert::status)?;
        Ok(Response::new(TargetList {
            total: data.total,
            items: data.items.into_iter().map(Self::to_reply).collect(),
        }))
    }

    async fn get_target(&self, request: Request<CodeRequest>) -> Result<Response<Target>, Status> {
        let key =
            ProtoConvert::code(&request.into_inner().code, "code").map_err(ProtoConvert::status)?;
        let data = self
            .svc
            .get_by_code(&key)
            .await
            .map_err(ProtoConvert::status)?;
        Ok(Response::new(Self::to_reply(data)))
    }

    async fn create_target(
        &self,
        request: Request<CreateTargetRpcRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();
        let mut payload = CreateTargetRequest {
            code: request.code,
            channel: ProtoConvert::channel(&request.channel_code).map_err(ProtoConvert::status)?,
            name: request.name,
            metadata: ProtoConvert::map(request.metadata),
        };
        payload.normalize();
        payload.validate().map_err(ProtoConvert::status)?;

        self.svc
            .create(payload)
            .await
            .map_err(ProtoConvert::status)?;
        Ok(Response::new(()))
    }

    async fn update_target(
        &self,
        request: Request<PutTargetRpcRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();
        let code = ProtoConvert::code(&request.code, "code").map_err(ProtoConvert::status)?;
        let mut payload = PutTargetRequest {
            name: request.name,
            metadata: ProtoConvert::map(request.metadata),
        };
        payload.normalize();

        self.svc
            .update(&code, payload)
            .await
            .map_err(ProtoConvert::status)?;
        Ok(Response::new(()))
    }
}
