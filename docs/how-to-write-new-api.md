# How to write a new explicit API

Notification service currently exposes two domain endpoints. To add another:

## 1. Service layer
Add a method on `NotificationService` in `src/services/`. Keep Kafka produce/consume and HTTP dispatch out of the handler.

## 2. API handler
Add a thin handler in `src/api/<domain>.rs` with a request DTO, then call the injected service from `AppState`.

Return **only** `ApiResponse` / `ApiResponse<T>` (never `AppResult<…>`). Example: `POST /notification/notify` in `src/api/notify.rs`.

## 3. Register the route
Nest it in `src/api/routes.rs` under `/notification` on the **protected** router (`api-key` + logging), unless the endpoint must be public like `/health`.

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
- Handlers stay thin; composition root stays in `src/bootstrap/` (`build.rs` wires services, `load.rs` loads the channel and source memory caches).
