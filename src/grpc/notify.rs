use prost_types::value::Kind;
use serde_json::{Map, Number, Value};
use tonic::{Request, Response, Status};

use super::pb::notification_server::Notification;
use super::pb::{NotifyReply, NotifyRequest as NotifyRpcRequest};
use crate::error::AppError;
use crate::models::NotifyRequest;
use crate::state::SharedState;

pub struct NotifyGrpc {
    state: SharedState,
}

impl NotifyGrpc {
    pub fn new(state: SharedState) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl Notification for NotifyGrpc {
    /// Same flow as the HTTP handler `api::notify::notify`.
    async fn notify(
        &self,
        request: Request<NotifyRpcRequest>,
    ) -> Result<Response<NotifyReply>, Status> {
        let request = request.into_inner();
        let mut payload = NotifyRequest {
            source: request.source,
            code: request.code,
            trace_id: request.trace_id,
            variables: request.variables.map(struct_to_map).unwrap_or_default(),
        };
        payload.normalize();
        payload.validate().map_err(to_status)?;
        let trace_id = payload.trace_id.clone();

        let svc = self.state.notify_service.clone();
        let message = svc.validate(payload).await.map_err(to_status)?;
        svc.notify(message).await.map_err(to_status)?;

        Ok(Response::new(NotifyReply { trace_id }))
    }
}

/// Internal errors are logged and returned without detail, like the HTTP API.
fn to_status(err: AppError) -> Status {
    match err {
        AppError::BadRequest(msg) => Status::invalid_argument(msg),
        AppError::NotFound(msg) => Status::not_found(msg),
        AppError::Conflict(msg) => Status::already_exists(msg),
        AppError::Internal(msg) => {
            tracing::error!(error = %msg, "gRPC notify failed");
            Status::internal("Internal server error")
        }
    }
}

fn struct_to_map(value: prost_types::Struct) -> Map<String, Value> {
    value
        .fields
        .into_iter()
        .map(|(key, value)| (key, to_json(value)))
        .collect()
}

fn to_json(value: prost_types::Value) -> Value {
    match value.kind {
        None | Some(Kind::NullValue(_)) => Value::Null,
        Some(Kind::BoolValue(b)) => Value::Bool(b),
        Some(Kind::StringValue(s)) => Value::String(s),
        Some(Kind::NumberValue(n)) => number_to_json(n),
        Some(Kind::StructValue(s)) => Value::Object(struct_to_map(s)),
        Some(Kind::ListValue(list)) => Value::Array(list.values.into_iter().map(to_json).collect()),
    }
}

/// `Struct` numbers are always doubles; keep whole numbers as integers so a
/// template renders `42`, not `42.0`.
fn number_to_json(n: f64) -> Value {
    const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_992.0;
    if n.fract() == 0.0 && n.abs() < MAX_SAFE_INTEGER {
        return Value::Number(Number::from(n as i64));
    }
    Number::from_f64(n)
        .map(Value::Number)
        .unwrap_or(Value::Null)
}
