mod auth;
mod convert;
mod definitions;
mod notify;
mod sources;
mod targets;
mod templates;
mod webhooks;

use std::net::SocketAddr;

use tonic::service::interceptor::InterceptedService;
use tonic::transport::Server;

use crate::error::{AppError, AppResult};
use crate::state::SharedState;

/// Code generated from `proto/notification.proto`.
pub mod pb {
    tonic::include_proto!("notification.v1");
}

pub use definitions::DefinitionGrpc;
pub use notify::NotifyGrpc;
pub use sources::SourceGrpc;
pub use targets::TargetGrpc;
pub use templates::TemplateGrpc;
pub use webhooks::WebhookGrpc;

use pb::definitions_server::DefinitionsServer;
use pb::notification_server::NotificationServer;
use pb::sources_server::SourcesServer;
use pb::targets_server::TargetsServer;
use pb::templates_server::TemplatesServer;
use pb::webhooks_server::WebhooksServer;

/// Serve the internal gRPC API until the server stops. Every service checks `api-key`.
pub async fn serve(state: SharedState, addr: SocketAddr) -> AppResult<()> {
    tracing::info!("gRPC listening on {}", addr);
    Server::builder()
        .add_service(InterceptedService::new(
            NotificationServer::from_arc(state.notify_grpc.clone()),
            auth::check_api_key,
        ))
        .add_service(InterceptedService::new(
            SourcesServer::from_arc(state.source_grpc.clone()),
            auth::check_api_key,
        ))
        .add_service(InterceptedService::new(
            TargetsServer::from_arc(state.target_grpc.clone()),
            auth::check_api_key,
        ))
        .add_service(InterceptedService::new(
            TemplatesServer::from_arc(state.template_grpc.clone()),
            auth::check_api_key,
        ))
        .add_service(InterceptedService::new(
            DefinitionsServer::from_arc(state.definition_grpc.clone()),
            auth::check_api_key,
        ))
        .add_service(InterceptedService::new(
            WebhooksServer::from_arc(state.webhook_grpc.clone()),
            auth::check_api_key,
        ))
        .serve(addr)
        .await
        .map_err(|e| AppError::internal_with("gRPC server failed", e))
}
