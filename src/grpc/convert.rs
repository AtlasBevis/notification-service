use chrono::{DateTime, Utc};
use prost_types::value::Kind;
use prost_types::{ListValue, Struct, Timestamp};
use serde_json::{Map, Number, Value};
use tonic::Status;

use crate::error::{AppError, AppResult};

/// Conversions between protobuf well-known types and the service's models.
pub struct ProtoConvert;

impl ProtoConvert {
    /// Internal errors are logged and returned without detail, like the HTTP API.
    pub fn status(err: AppError) -> Status {
        match err {
            AppError::BadRequest(msg) => Status::invalid_argument(msg),
            AppError::NotFound(msg) => Status::not_found(msg),
            AppError::Conflict(msg) => Status::already_exists(msg),
            AppError::Internal(msg) => {
                tracing::error!(error = %msg, "gRPC request failed");
                Status::internal("Internal server error")
            }
        }
    }

    /// Trim and upper-case a code addressed by the request, like a path `{code}`.
    pub fn code(value: &str, field: &str) -> AppResult<String> {
        let key = value.trim().to_ascii_uppercase();
        if key.is_empty() {
            return Err(AppError::bad_request(format!("{field} is required")));
        }
        Ok(key)
    }

    pub fn map(value: Option<Struct>) -> Map<String, Value> {
        value
            .map(|s| {
                s.fields
                    .into_iter()
                    .map(|(key, value)| (key, Self::to_json(value)))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// A JSON object as `Struct`; any other value has no `Struct` form.
    pub fn to_struct(value: &Value) -> Option<Struct> {
        value.as_object().map(Self::map_to_struct)
    }

    pub fn timestamp(value: DateTime<Utc>) -> Timestamp {
        Timestamp {
            seconds: value.timestamp(),
            nanos: value.timestamp_subsec_nanos() as i32,
        }
    }

    fn map_to_struct(map: &Map<String, Value>) -> Struct {
        Struct {
            fields: map
                .iter()
                .map(|(key, value)| (key.clone(), Self::to_proto(value)))
                .collect(),
        }
    }

    fn to_json(value: prost_types::Value) -> Value {
        match value.kind {
            None | Some(Kind::NullValue(_)) => Value::Null,
            Some(Kind::BoolValue(b)) => Value::Bool(b),
            Some(Kind::StringValue(s)) => Value::String(s),
            Some(Kind::NumberValue(n)) => Self::number_to_json(n),
            Some(Kind::StructValue(s)) => Value::Object(Self::map(Some(s))),
            Some(Kind::ListValue(list)) => {
                Value::Array(list.values.into_iter().map(Self::to_json).collect())
            }
        }
    }

    fn to_proto(value: &Value) -> prost_types::Value {
        let kind = match value {
            Value::Null => Kind::NullValue(0),
            Value::Bool(b) => Kind::BoolValue(*b),
            Value::Number(n) => Kind::NumberValue(n.as_f64().unwrap_or_default()),
            Value::String(s) => Kind::StringValue(s.clone()),
            Value::Array(items) => Kind::ListValue(ListValue {
                values: items.iter().map(Self::to_proto).collect(),
            }),
            Value::Object(map) => Kind::StructValue(Self::map_to_struct(map)),
        };
        prost_types::Value { kind: Some(kind) }
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
}
