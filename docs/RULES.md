# RULES.md — Data Notification

Coding rules for this repository. `CLAUDE.md` imports this file; what the service does and how it is laid out is described there.

## Code organization

- Prefer grouping related functions into a struct and its `impl` block over free functions: controllers, services, repositories, and utils (`JsonUtils`, `StringUtils`) all follow this.
- Use a unit struct (`pub struct JsonUtils;`) when the functions need no state, and call them as associated functions (`JsonUtils::get_string(...)`).
- Private helpers that only serve one struct go in the same `impl` block as private methods, not as loose functions in the file.
- When you touch a file that still has loose functions, move them into the matching struct if it is a small change.

## Layers

- Keep handlers thin: parse, normalize, and validate the request, call a service trait, and return `ApiResponse`.
- Put orchestration in services. Service-to-service dependencies use traits (`Arc<dyn XxxTrait>`), not concrete types.
- Put SQL in repositories.
- Wire concrete implementations only in `src/bootstrap/build.rs`, in this order: repositories, services, controllers.

## API controllers

- Each API domain is one controller struct in `src/api/<domain>.rs` (`DefinitionController`, `SourceController`, ...). It holds the service traits it needs and has a `new(...)` constructor.
- Handlers are associated functions on the controller. They take the controller as axum state, `State(this): State<Arc<Self>>`, and call `this.svc`. Do not write free handler functions, and do not take `State<SharedState>` in a handler.
- Handlers return only `ApiResponse` / `ApiResponse<T>`, never `AppResult<...>`. Convert an error with `err.to_response()`.
- Re-export the controller from `src/api/mod.rs`, build it in `bootstrap/build.rs`, and store it in `AppState` as `Arc<XxxController>`.
- Register routes in `src/api/routes.rs` with one function per controller that ends in `.with_state(controller)`, nested in `protected_routes()` unless the endpoint must be public like `/health`.
- The service uses axum 0.7: write path captures as `/:code`, not `/{code}`.
- Keep the HTTP notify handler and the gRPC handler in `src/grpc/` in step when the send flow changes.

## Shared state and `Arc`

- The state passed around the app is `SharedState` (`Arc<AppState>`, `src/state.rs`). Keep the outer `Arc`, so cloning the state stays one counter increment however large `AppState` grows.
- Use `Arc` for a value that several owners share across tasks: services behind `dyn Trait`, controllers, infrastructure clients. Do not wrap a value that has a single owner.

## Utils

- Helpers in `src/utils/` are grouped into one unit struct per topic (`JsonUtils`, `StringUtils`, `TimeUtils`) with associated functions and constants. Do not add free functions there.
- Each utils struct is independent: it does not call another utils struct, and nothing in `src/utils/` imports from another file in `src/utils/`. Repeat a small piece of logic instead of sharing it between utils.
- Utils do not depend on services, repositories, or config; they may use `crate::error` for `AppResult`.
- `src/utils/mod.rs` keeps the files private and re-exports only the structs; call them as `JsonUtils::get_string(...)`.
- Outside `src/utils/`, use the utils instead of calling the underlying library for the same job: `TimeUtils::now()` instead of `Utc::now()`, `TimeUtils::format(...)` instead of `.format("%Y-...")`, `StringUtils::get_env(...)` instead of `std::env::var(...)`, `JsonUtils::...` instead of hand-written `serde_json` parsing. If the helper you need is missing, add it to the matching utils struct first.

## Repositories and SQL

- Repositories own SQL and return domain models.
- Put all PostgreSQL `Row` to model conversion and persisted-value parsing in the model's `map_row(&Row) -> AppResult<Self>`. Repository methods query rows and call that mapper; they do not read columns or parse persisted enums themselves.
- Write SQL as raw string literals (`r#"..."#`), one clause per line, with the opening and closing delimiters on their own lines.
- Do not add SQL `CHECK` constraints for request or webhook configuration values. Keep structural constraints such as primary keys, foreign keys, and required uniqueness.

## Validation

- Validate values at the application boundary and when parsing persisted configuration.
- Every request type has `normalize()` and `validate()`. Every handler, HTTP and gRPC, calls `payload.normalize()` first and `payload.validate()` second, and only then calls the service.
- `normalize()` trims strings and upper-cases codes and enum-like values (`code`, `status`, `level`, `format`, ...); it never fails. `validate()` checks the already-normalized values and returns `AppResult<()>` with `AppError::bad_request`; it does not trim or re-case.
- Reject invalid HTTP header names and values before accepting the notification.

## Error handling

- Every fallible function returns `AppResult<T>` (`Result<T, AppError>`, defined in `src/error.rs`). Do not use bare `Result<T, E>`, `Result<T, String>`, or `anyhow::Result`; `anyhow` is not a dependency. This applies to all layers, including `infras/`, `config/`, `bootstrap/`, and both `main` functions.
- Build errors with `AppError::bad_request` / `not_found` / `conflict` / `internal`. Wrap a foreign error with `AppError::internal_with("what failed", err)`, or add a `From` impl in `src/error.rs` when `?` should convert it directly.
- A function that cannot fail returns the value itself, not a `Result`.
- Exceptions are only signatures dictated by an external trait: `FromRequest::from_request`, `std::fmt::Result`, and `reqwest_middleware::Result` in the HTTP middleware.

## Webhooks and secrets

- Webhook calls use the shared `HttpClient` from `Infrastructure`, injected into `WebhookService`. Do not build a separate HTTP client for webhooks.
- Never log or serialize static secrets or tokens.
- Do not commit secrets; `auth.api_key` comes from config/ConfigMap.

## Configuration

- Config structs have no serde defaults: every field must be present in `app.yaml` / `app-job.yaml` (only the `auth` section is optional).
- Config goes through `Config`: `Config::new(default_path)` reads and parses the file, `Config::set(config)` stores it once at startup, `Config::load()` reads it anywhere. Do not add free functions for config access.

## Testing

- Unit tests are a verification step, not part of the source. Write them to prove a change, run them until they pass, then delete them before finishing. Do not leave `#[cfg(test)]` modules, `tests/` files, or test-only helpers in the repository.
- Report what was tested and the result when handing the change over, since the tests themselves are gone.
