use axum::response::{IntoResponse, Response as AxumResponse};
use tokio_postgres::error::SqlState;

use crate::models::ApiResponse;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    NotFound(String),
    Conflict(String),
    Internal(String),
}

impl AppError {
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::BadRequest(msg.into())
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    pub fn conflict(msg: impl Into<String>) -> Self {
        Self::Conflict(msg.into())
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    pub fn to_response(self) -> ApiResponse<()> {
        match self {
            Self::BadRequest(msg) => ApiResponse::bad_request(msg),
            Self::NotFound(msg) => ApiResponse::not_found(msg),
            Self::Conflict(msg) => ApiResponse::conflict(msg),
            Self::Internal(_) => ApiResponse::internal_error(),
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadRequest(msg)
            | Self::NotFound(msg)
            | Self::Conflict(msg)
            | Self::Internal(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for AppError {}

impl From<deadpool_postgres::PoolError> for AppError {
    fn from(err: deadpool_postgres::PoolError) -> Self {
        Self::Internal(err.to_string())
    }
}

impl From<tokio_postgres::Error> for AppError {
    fn from(err: tokio_postgres::Error) -> Self {
        if let Some(db) = err.as_db_error() {
            if db.code() == &SqlState::UNIQUE_VIOLATION {
                return Self::Conflict(db.message().to_string());
            }
            if db.code() == &SqlState::FOREIGN_KEY_VIOLATION {
                return Self::Conflict(db.message().to_string());
            }
        }
        Self::Internal(err.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> AxumResponse {
        self.to_response().into_response()
    }
}
