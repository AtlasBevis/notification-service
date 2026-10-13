use crate::api::{health, notify, sources, targets, templates};
use crate::config::Config;
use crate::middlewares::{auth_middleware, http_metrics_middleware, logging_middleware};
use crate::state::SharedState;
use axum::{
    middleware,
    routing::{get, patch, post},
    Router,
};

const HEALTH_API: &str = "/health";
const NOTI_API: &str = "/noti";
const SOURCES_API: &str = "/sources";
const TARGETS_API: &str = "/targets";
const TEMPLATES_API: &str = "/templates";

/// `/health` is public at the root; everything else is protected under `server.base_path`.
pub fn app_routes(state: SharedState) -> Router {
    let base_path = Config::load().server.base_path.as_str();

    Router::new()
        .route(HEALTH_API, get(health::health))
        .nest(base_path, protected_routes())
        .with_state(state)
}

fn protected_routes() -> Router<SharedState> {
    Router::new()
        .nest(NOTI_API, noti_routes())
        .nest(SOURCES_API, source_routes())
        .nest(TARGETS_API, target_routes())
        .nest(TEMPLATES_API, template_routes())
        .route_layer(middleware::from_fn(auth_middleware))
        .route_layer(middleware::from_fn(logging_middleware))
        .route_layer(middleware::from_fn(http_metrics_middleware))
}

/// Routes that work on the `notifications` table.
fn noti_routes() -> Router<SharedState> {
    Router::new().route("/notify", post(notify::notify))
}

fn source_routes() -> Router<SharedState> {
    Router::new()
        .route("/", post(sources::create_source))
        .route("/metadata", patch(sources::patch_metadata))
        .route("/:code", get(sources::get_source))
}

fn target_routes() -> Router<SharedState> {
    Router::new()
        .route("/", post(targets::create_target))
        .route("/metadata", patch(targets::patch_metadata))
        .route("/:code", get(targets::get_target))
}

fn template_routes() -> Router<SharedState> {
    Router::new().route("/", post(templates::create_template))
}
