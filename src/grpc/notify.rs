use std::sync::Arc;

use tonic::{Request, Response, Status};

use super::convert::ProtoConvert;
use super::pb::notification_server::Notification;
use super::pb::{NotifyReply, NotifyRequest as NotifyRpcRequest};
use crate::models::NotifyRequest;
use crate::services::NotificationTrait;

pub struct NotifyGrpc {
    svc: Arc<dyn NotificationTrait>,
}

impl NotifyGrpc {
    pub fn new(svc: Arc<dyn NotificationTrait>) -> Self {
        Self { svc }
    }
}

#[tonic::async_trait]
impl Notification for NotifyGrpc {
    /// Same flow as the HTTP handler `NotificationController::notify`.
    async fn notify(
        &self,
        request: Request<NotifyRpcRequest>,
    ) -> Result<Response<NotifyReply>, Status> {
        let request = request.into_inner();
        let mut payload = NotifyRequest {
            source: request.source,
            code: request.code,
            trace_id: request.trace_id,
            variables: ProtoConvert::map(request.variables),
        };
        payload.normalize();
        payload.validate().map_err(ProtoConvert::status)?;
        let trace_id = payload.trace_id.clone();

        let message = self
            .svc
            .validate(payload)
            .await
            .map_err(ProtoConvert::status)?;
        self.svc
            .notify(message)
            .await
            .map_err(ProtoConvert::status)?;

        Ok(Response::new(NotifyReply { trace_id }))
    }
}
