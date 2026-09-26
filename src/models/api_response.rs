use axum::{
    http::StatusCode,
    response::{IntoResponse, Response as AxumResponse},
    Json,
};
use serde::Serialize;

/// Standard API response envelope (`Code`, `Message`, `Data`).
#[derive(Debug, Clone, Serialize)]
#[serde(bound = "T: Serialize")]
#[serde(rename_all = "PascalCase")]
pub struct ApiResponse<T = ()> {
    #[serde(skip_serializing)]
    http_status: StatusCode,
    pub code: u16, // business code
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T> ApiResponse<T>
where
    T: Serialize,
{
    pub fn new(
        http_status: StatusCode,
        code: u16,
        message: impl Into<String>,
        data: Option<T>,
    ) -> Self {
        Self {
            http_status,
            code,
            message: message.into(),
            data,
        }
    }

    pub fn without_data(http_status: StatusCode, code: u16, message: impl Into<String>) -> Self {
        Self {
            http_status,
            code,
            message: message.into(),
            data: None,
        }
    }

    pub fn success(data: T) -> Self {
        Self::new(
            StatusCode::OK,
            StatusCode::OK.as_u16(),
            "Success",
            Some(data),
        )
    }

    pub fn created(data: T) -> Self {
        Self::new(
            StatusCode::CREATED,
            StatusCode::CREATED.as_u16(),
            "Created",
            Some(data),
        )
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::without_data(
            StatusCode::BAD_REQUEST,
            StatusCode::BAD_REQUEST.as_u16(),
            message,
        )
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::without_data(StatusCode::CONFLICT, StatusCode::CONFLICT.as_u16(), message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::without_data(
            StatusCode::NOT_FOUND,
            StatusCode::NOT_FOUND.as_u16(),
            message,
        )
    }

    pub fn internal_error() -> Self {
        Self::without_data(
            StatusCode::INTERNAL_SERVER_ERROR,
            StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
            "Internal Server Error",
        )
    }

    pub fn service_unavailable() -> Self {
        Self::without_data(
            StatusCode::SERVICE_UNAVAILABLE,
            StatusCode::SERVICE_UNAVAILABLE.as_u16(),
            "Service Unavailable",
        )
    }
}

impl<T> IntoResponse for ApiResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> AxumResponse {
        (self.http_status, Json(self)).into_response()
    }
}

impl ApiResponse<()> {
    pub fn ok() -> Self {
        Self::without_data(StatusCode::OK, StatusCode::OK.as_u16(), "Success")
    }

    pub fn accepted() -> Self {
        Self::without_data(
            StatusCode::ACCEPTED,
            StatusCode::ACCEPTED.as_u16(),
            "The request has been accepted",
        )
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::without_data(
            StatusCode::UNAUTHORIZED,
            StatusCode::UNAUTHORIZED.as_u16(),
            message,
        )
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::without_data(
            StatusCode::FORBIDDEN,
            StatusCode::FORBIDDEN.as_u16(),
            message,
        )
    }
}
