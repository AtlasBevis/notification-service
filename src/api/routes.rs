use crate::api::openapi::ApiDoc;
use crate::api::{health, notification, notify, sources, targets, templates};
use crate::middlewares::{auth_middleware, http_metrics_middleware, logging_middleware};
use crate::state::SharedState;
use axum::routing::patch;
use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

const TEMPLATES_API: &str = "/templates";
const SOURCES_API: &str = "/sources";
const TARGETS_API: &str = "/targets";
const PATH_BASE: &str = "/notification";

pub fn app_routes(state: SharedState) -> Router {
    let public = Router::new().route("/health", get(health::health));

    let templates = Router::new().route("/", post(templates::create_template));

    let sources_routes = Router::new()
        .route("/metadata", patch(sources::patch_metadata))
        .route("/{code}", get(sources::get_source))
        .route("/", post(sources::create_source));

    let targets_routes = Router::new()
        .route("/metadata", patch(targets::patch_metadata))
        .route("/{code}", get(targets::get_target))
        .route("/", post(targets::create_target));

    let notification = Router::new()
        .route(
            "/",
            post(notification::create_notification).get(notification::list_notifications),
        )
        .route("/notify", post(notify::notify))
        .nest(TEMPLATES_API, templates)
        .nest(SOURCES_API, sources_routes)
        .nest(TARGETS_API, targets_routes)
        .route(
            "/{code}",
            get(notification::get_notification)
                .put(notification::update_notification)
                .delete(notification::delete_notification),
        );

    let protected = Router::new()
        .nest(PATH_BASE, notification)
        .route_layer(middleware::from_fn(auth_middleware))
        .route_layer(middleware::from_fn(logging_middleware))
        .route_layer(middleware::from_fn(http_metrics_middleware));

    public
        .merge(protected)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .with_state(state)
}
