use std::sync::Arc;

use tonic::{Request, Response, Status};

use super::convert::ProtoConvert;
use super::pb::sources_server::Sources;
use super::pb::{
    CodeRequest, CreateSourceRequest as CreateSourceRpcRequest,
    PatchMetadataRequest as PatchMetadataRpcRequest, Source,
};
use crate::models::{CreateSourceRequest, PatchMetadataRequest, SourceData};
use crate::services::SourceTrait;

/// gRPC counterpart of `SourceController`.
pub struct SourceGrpc {
    svc: Arc<dyn SourceTrait>,
}

impl SourceGrpc {
    pub fn new(svc: Arc<dyn SourceTrait>) -> Self {
        Self { svc }
    }

    fn to_reply(data: SourceData) -> Source {
        Source {
            id: data.id,
            code: data.code,
            name: data.name,
            status: data.status.into(),
            metadata: ProtoConvert::to_struct(&data.metadata),
            created_at: Some(ProtoConvert::timestamp(data.created_at)),
            updated_at: data.updated_at.map(ProtoConvert::timestamp),
        }
    }
}

#[tonic::async_trait]
impl Sources for SourceGrpc {
    async fn get_source(&self, request: Request<CodeRequest>) -> Result<Response<Source>, Status> {
        let key =
            ProtoConvert::code(&request.into_inner().code, "code").map_err(ProtoConvert::status)?;
        let data = self
            .svc
            .get_by_code(&key)
            .await
            .map_err(ProtoConvert::status)?;
        Ok(Response::new(Self::to_reply(data)))
    }

    async fn create_source(
        &self,
        request: Request<CreateSourceRpcRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();
        let mut payload = CreateSourceRequest {
            code: request.code,
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

    async fn patch_source_metadata(
        &self,
        request: Request<PatchMetadataRpcRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.into_inner();
        let mut payload = PatchMetadataRequest {
            code: request.code,
            metadata: ProtoConvert::map(request.metadata),
        };
        payload.normalize();
        payload.validate().map_err(ProtoConvert::status)?;

        self.svc
            .patch_metadata(payload)
            .await
            .map_err(ProtoConvert::status)?;
        Ok(Response::new(()))
    }
}
