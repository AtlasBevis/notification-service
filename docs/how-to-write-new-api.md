# How to write a new explicit API

To add an endpoint (the full rule set is in [`RULES.md`](RULES.md)):

## 1. Service layer
Add a method to the domain's service trait and its implementation in `src/services/`. Keep Kafka produce/consume and HTTP dispatch out of the handler.

## 2. API controller
Each domain has one controller struct in `src/api/<domain>.rs`. It holds the service trait and its handlers are associated functions that take the controller as state:

```rust
pub struct SourceController {
    svc: Arc<dyn SourceTrait>,
}

impl SourceController {
    pub fn new(svc: Arc<dyn SourceTrait>) -> Self {
        Self { svc: svc }
    }

    pub async fn get_source(
        State(this): State<Arc<Self>>,
        Path(code): Path<String>,
    ) -> ApiResponse<SourceData> {
        match this.svc.get_by_code(&code).await {
            Ok(data) => ApiResponse::success(data),
            Err(err) => err.to_response(),
        }
    }
}
```

Add the handler to the existing controller, or add a new controller and re-export it from `src/api/mod.rs`. Return **only** `ApiResponse` / `ApiResponse<T>` (never `AppResult<…>`).

## 3. Wire and register
A new controller is built in `src/bootstrap/build.rs` (under `// controllers`) and stored in `AppState` (`src/state.rs`) as `Arc<XxxController>`.

Register the route in `src/api/routes.rs`, in the function for that controller, which ends with `.with_state(controller)`. Nest it in `protected_routes()` (mounted under `server.base_path`, `/notification`, with `api-key` + logging), unless the endpoint must be public like `/health`.

## Repository and model mapping
Keep SQL in `src/repository/`. For database results, define `map_row(&tokio_postgres::Row) -> AppResult<Self>` on the corresponding model in `src/models/`. Repository methods should handle query/not-found behavior, then call the model mapper; do not read columns or parse persisted enums in the repository.

## Error handling
Every fallible function returns `AppResult<T>` from `src/error.rs` — services, repositories, models, infrastructure, config, bootstrap, and `main`. Do not use bare `Result<T, E>`, `Result<T, String>`, or `anyhow`.

- Create errors with `AppError::bad_request`, `not_found`, `conflict`, or `internal`.
- Wrap an error from another crate with `AppError::internal_with("what failed", err)`, or add a `From` impl in `src/error.rs` so `?` converts it.
- Request `validate()` methods return `AppResult<()>`; the handler returns `err.to_response()`.
- Do not wrap a value in `Result` when the function cannot fail.
- Only signatures required by an external trait keep another result type (`FromRequest::from_request`, `std::fmt::Result`, `reqwest_middleware::Result`).

Handlers are the one place that does not return `AppResult`: they convert it to `ApiResponse`.

## Testing
Write unit tests to verify the change and run them until they pass, then delete them. Tests are not kept in the source: no `#[cfg(test)]` modules, no `tests/` files, no test-only helpers. Say what you tested when you hand the change over.

## Why do this?
- Handlers stay thin; composition root stays in `src/bootstrap/` (`build.rs` wires services; `ChannelService::new` / `SourceService::new` load their memory caches).
