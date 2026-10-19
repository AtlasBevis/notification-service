use tonic::{Request, Status};

use crate::config::Config;

const API_KEY_METADATA: &str = "x-api-key";

/// Same rule as the HTTP `x-api-key` middleware, read from gRPC metadata.
pub fn check_api_key(request: Request<()>) -> Result<Request<()>, Status> {
    let config = Config::load();
    if !config.auth.is_enabled() {
        return Ok(request);
    }

    let provided = request
        .metadata()
        .get(API_KEY_METADATA)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Status::unauthenticated("Missing x-api-key"))?;

    // is admin
    if config.auth.is_admin(provided) {
        return Ok(request);
    }

    if !config.auth.matches(provided) {
        return Err(Status::unauthenticated("Invalid x-api-key"));
    }
    Ok(request)
}
