# Data Notification — Request Flow

Flow of an API call through `data-notification`.

```mermaid
sequenceDiagram
    participant Client
    participant Router as Router (axum)
    participant Logging as LoggingMiddleware
    participant Auth as AuthMiddleware
    participant Handler as Notification handler
    participant Kafka
    participant Consumer as notification-consumer

    Client->>Router: POST /notification/notify
    Router->>Logging: Buffer body, log request
    Logging->>Auth: Verify api-key
    Auth->>Handler: Validated request
    Handler->>Kafka: Produce request JSON (key = correlation_id)
    Handler-->>Client: 202 Accepted
    Kafka-->>Consumer: Process `notification-consumer` dispatches TEAMS / EMAIL
```

## Routing `src/api/routes.rs`

- **Public:** `GET /health`, Swagger UI / OpenAPI JSON
- **Protected (`api-key` + logging), nested under `/notification`:** `POST /notification/notify`

## Logging `src/middlewares/logging.rs`

Buffers the request body, logs method/path/body (truncated), then logs the response.

## Auth `src/middlewares/auth.rs`

Header `api-key` compared to `auth.api_key`. See [`HMAC_AUTH.md`](HMAC_AUTH.md).

## Handler → Kafka

Thin handler normalizes + validates the request body and returns 400. The service checks catalog (memory then DB). If Kafka is on, it produces and the consumer dispatches TEAMS / EMAIL. If Kafka is off, the API process dispatches the channel directly.
