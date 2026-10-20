use crate::api::{
    DefinitionController, HealthController, NotificationController, SourceController,
    TargetController, TemplateController, WebhookController,
};
use crate::config::Config;
use crate::middlewares::{
    auth_middleware, http_metrics_middleware, logging_middleware, request_context_middleware,
};
use crate::models::ApiResponse;
use crate::state::SharedState;
use axum::{
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use std::any::Any;
use std::sync::Arc;
use tower_http::catch_panic::CatchPanicLayer;

const HEALTH_API: &str = "/health";
const NOTI_API: &str = "/noti";
const SOURCES_API: &str = "/sources";
const TARGETS_API: &str = "/targets";
const TEMPLATES_API: &str = "/templates";
const DEFINITIONS_API: &str = "/definitions";
const WEBHOOKS_API: &str = "/webhooks";

/// `/health` is public at the root; everything else is protected under `server.base_path`.
/// Handlers are controller methods; each group of routes takes its controller as state.
/// `CatchPanicLayer` is the outermost layer: a panic anywhere below it (handlers and
/// the route middlewares) becomes a 500 `ApiResponse` instead of a dropped connection.
pub fn app_routes(state: SharedState) -> Router {
    let base_path = Config::load().server.base_path.as_str();

    Router::new()
        .merge(health_routes(state.health_controller.clone()))
        .nest(base_path, internal_routes(&state))
        .layer(CatchPanicLayer::custom(panic_response))
}

/// Turns a panic in a handler or route middleware into a 500 `ApiResponse`.
fn panic_response(err: Box<dyn Any + Send + 'static>) -> Response {
    let detail = if let Some(s) = err.downcast_ref::<String>() {
        s.as_str()
    } else if let Some(s) = err.downcast_ref::<&str>() {
        s
    } else {
        "unknown panic"
    };
    tracing::error!(panic = %detail, "request handler panicked");
    ApiResponse::<()>::internal_error().into_response()
}

fn internal_routes(state: &SharedState) -> Router {
    Router::new()
        .nest(NOTI_API, noti_routes(state.notification_controller.clone()))
        .nest(SOURCES_API, source_routes(state.source_controller.clone()))
        .nest(TARGETS_API, target_routes(state.target_controller.clone()))
        .nest(
            TEMPLATES_API,
            template_routes(state.template_controller.clone()),
        )
        .nest(
            DEFINITIONS_API,
            definition_routes(state.definition_controller.clone()),
        )
        .nest(
            WEBHOOKS_API,
            webhook_routes(state.webhook_controller.clone()),
        )
        .route_layer(middleware::from_fn(auth_middleware))
        .route_layer(middleware::from_fn(logging_middleware))
        .route_layer(middleware::from_fn(request_context_middleware))
        .route_layer(middleware::from_fn(http_metrics_middleware))
}

fn health_routes(controller: Arc<HealthController>) -> Router {
    Router::new()
        .route(HEALTH_API, get(HealthController::health))
        .with_state(controller)
}

/// Routes that work on the `notifications` table.
fn noti_routes(controller: Arc<NotificationController>) -> Router {
    Router::new()
        .route("/notify", post(NotificationController::notify))
        .with_state(controller)
}

fn source_routes(controller: Arc<SourceController>) -> Router {
    Router::new()
        .route(
            "/",
            get(SourceController::list_sources)
                .post(SourceController::create_source)
                .put(SourceController::update_source),
        )
        .route("/{code}", get(SourceController::get_source))
        .with_state(controller)
}

fn target_routes(controller: Arc<TargetController>) -> Router {
    Router::new()
        .route(
            "/",
            get(TargetController::list_targets).post(TargetController::create_target),
        )
        .route(
            "/{code}",
            get(TargetController::get_target).put(TargetController::update_target),
        )
        .with_state(controller)
}

fn definition_routes(controller: Arc<DefinitionController>) -> Router {
    Router::new()
        .route(
            "/",
            get(DefinitionController::list_definitions)
                .post(DefinitionController::create_definition),
        )
        .route(
            "/{code}",
            get(DefinitionController::get_definition)
                .put(DefinitionController::update_definition)
                .delete(DefinitionController::delete_definition),
        )
        .with_state(controller)
}

fn template_routes(controller: Arc<TemplateController>) -> Router {
    Router::new()
        .route("/", post(TemplateController::save_template))
        .with_state(controller)
}

fn webhook_routes(controller: Arc<WebhookController>) -> Router {
    Router::new()
        .route(
            "/",
            get(WebhookController::list_webhooks).post(WebhookController::create_webhook),
        )
        .route(
            "/{code}",
            get(WebhookController::get_webhook)
                .put(WebhookController::update_webhook)
                .delete(WebhookController::delete_webhook),
        )
        .with_state(controller)
}
