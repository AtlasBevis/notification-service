# How to write a new explicit API

Notification service currently exposes two domain endpoints. To add another:

## 1. Service layer
Add a method on `NotificationService` in `src/services/`. Keep Kafka produce/consume and HTTP dispatch out of the handler.

## 2. API handler
Add a thin handler in `src/api/<domain>.rs` with a request DTO, then call the injected service from `AppState`.

Return **only** `ApiResponse` / `ApiResponse<T>` (never `AppResult<…>`). Example: `POST /notification/notify` in `src/api/notify.rs`.

## 3. Register the route
Nest it in `src/api/routes.rs` under `/notification` on the **protected** router (`api-key` + logging), unless the endpoint must be public like `/health`.

## Why do this?
- Handlers stay thin; composition root stays in `src/bootstrap/mod.rs`.
