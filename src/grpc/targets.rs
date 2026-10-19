use std::sync::Arc;

use tonic::{Request, Response, Status};

use super::convert::ProtoConvert;
use super::pb::targets_server::Targets;
use super::pb::{
    CodeRequest, PageRequest as PageRpcRequest, Target, TargetList,
    TargetRequest as TargetRpcRequest,
};
use crate::models::{PageRequest, TargetData, TargetRequest};
use crate::services::TargetTrait;

/// gRPC counterpart of `TargetController`.
pub struct TargetGrpc {
    svc: Arc<dyn TargetTrait>,
}

impl TargetGrpc {
    pub fn new(svc: Arc<dyn TargetTrait>) -> Self {
        Self { svc }
    }

    fn to_request(request: TargetRpcRequest) -> TargetRequest {
        TargetRequest {
            code: request.code,
            channel: request.channel_code,
            name: request.name,
            metadata: ProtoConvert::map(request.metadata),
        }
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
        request: Request<TargetRpcRequest>,
    ) -> Result<Response<()>, Status> {
        let mut payload = Self::to_request(request.into_inner());
        payload.normalize();
        payload.validate(true).map_err(ProtoConvert::status)?;

        self.svc
            .create(payload)
            .await
            .map_err(ProtoConvert::status)?;
        Ok(Response::new(()))
    }

    async fn update_target(
        &self,
        request: Request<TargetRpcRequest>,
    ) -> Result<Response<()>, Status> {
        let mut payload = Self::to_request(request.into_inner());
        payload.normalize();
        payload.validate(false).map_err(ProtoConvert::status)?;

        self.svc
            .update(payload)
            .await
            .map_err(ProtoConvert::status)?;
        Ok(Response::new(()))
    }
}
