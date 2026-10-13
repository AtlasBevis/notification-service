mod auth;
mod notify;

use std::net::SocketAddr;

use tonic::transport::Server;

use crate::error::{AppError, AppResult};
use crate::state::SharedState;

/// Code generated from `proto/notification.proto`.
pub mod pb {
    tonic::include_proto!("notification.v1");
}

use notify::NotifyGrpc;
use pb::notification_server::NotificationServer;

/// Serve the internal gRPC API until the server stops.
pub async fn serve(state: SharedState, addr: SocketAddr) -> AppResult<()> {
    tracing::info!("gRPC listening on {}", addr);
    Server::builder()
        .add_service(NotificationServer::with_interceptor(
            NotifyGrpc::new(state),
            auth::check_api_key,
        ))
        .serve(addr)
        .await
        .map_err(|e| AppError::internal_with("gRPC server failed", e))
}
