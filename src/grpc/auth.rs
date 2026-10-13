use tonic::{Request, Status};

use crate::config::Config;

const API_KEY_METADATA: &str = "api-key";

/// Same rule as the HTTP `api-key` middleware, read from gRPC metadata.
pub fn check_api_key(request: Request<()>) -> Result<Request<()>, Status> {
    let Some(auth) = &Config::load().auth else {
        return Ok(request);
    };
    if !auth.enabled {
        return Ok(request);
    }

    let provided = request
        .metadata()
        .get(API_KEY_METADATA)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Status::unauthenticated("Missing api-key"))?;

    if !auth.matches(provided) {
        return Err(Status::unauthenticated("Invalid api-key"));
    }
    Ok(request)
}
