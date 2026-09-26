use axum::async_trait;
use axum::extract::rejection::JsonRejection;
use axum::extract::{FromRequest, Request};
use axum::Json;
use serde::de::DeserializeOwned;

use super::api_response::ApiResponse;

/// JSON body extractor. Deserialize fail → 400 `ApiResponse` (handler is not called).
pub struct ApiRequest<T>(pub T);

#[async_trait]
impl<S, T> FromRequest<S> for ApiRequest<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiResponse<()>;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(req, state).await {
            Ok(Json(value)) => Ok(ApiRequest(value)),
            Err(err) => Err(ApiResponse::bad_request(json_error(&err))),
        }
    }
}

fn json_error(err: &JsonRejection) -> String {
    match err {
        JsonRejection::JsonDataError(_) | JsonRejection::JsonSyntaxError(_) => {
            format!("Invalid JSON: {}", err.body_text())
        }
        JsonRejection::MissingJsonContentType(_) => {
            "Content-Type must be application/json".to_string()
        }
        _ => err.body_text(),
    }
}
