# CLAUDE.md — Data Notification

Repository guidance for AI assistants working on this Rust + Axum + Kafka notification service.

## Service behavior

- The send endpoint is `POST /notification/notify` with `source`, `code`, `trace_id`, and `variables`.
- `definitions` is configuration provisioned in the database. The service does not expose notification CRUD endpoints.
- Templates are selected per target by `definition_targets.template_id`; `definitions` has no `template_id`. A WEBHOOK target with a template sends that `format = 'JSON'` template as its request body; without one it sends the request's `variables` unchanged. EMAIL and MSTEAMS require a template and render one title/body from the first linked target that has one.
- An accepted send internally inserts one row into `notifications` and one row into `deliveries` in a transaction.
- Delivery channels are `EMAIL`, `MSTEAMS`, and `WEBHOOK`. The API (`data-notification`) only publishes to Kafka (`NotifyService`); `notification-job` only consumes and dispatches to the channel (`ConsumerService`). The API never calls a channel itself, so a send fails when Kafka is not connected.
- Channels and sources are cached in memory at startup (`bootstrap/load.rs`). Targets are not cached; they are read from Postgres on every use.
- Protected API routes use the `api-key` header. `GET /health` is public.

Default listen address is `0.0.0.0:8080` (see `app.yaml` and environment overrides).

Every webhook config must have a row in `webhook_auth_configs`; a missing row is `AppError::NotFound`. `auth_mode = STATIC` sends `header_value` as `header_name`. `auth_mode = TOKEN_ENDPOINT` obtains and caches a token from `token_url`, then sends it using `header_name` and `header_prefix`. `webhook_configs.headers` contains additional static outgoing headers; auth header values take precedence on duplicate names. Webhook calls use the shared `HttpClient` from `Infrastructure`, injected into `WebhookService`; do not build a separate HTTP client for webhooks. Never log or serialize static secrets or tokens. Each selected target endpoint is called once per dispatch.

## Database constraints and validation

- Do not add SQL `CHECK` constraints for request or webhook configuration values. Keep structural constraints such as primary keys, foreign keys, and required uniqueness.
- Validate values at the application boundary and when parsing persisted configuration. Reject invalid HTTP header names/values before accepting the notification.

## Documentation

| File | Contents |
|---|---|
| [`docs/api_flow.md`](docs/api_flow.md) | Request and delivery flow, route list |
| [`migration/FLOW.md`](migration/FLOW.md) | Migration order and database lifecycle |
| [`docs/how-to-write-new-api.md`](docs/how-to-write-new-api.md) | API handler conventions |
| [`docs/HMAC_AUTH.md`](docs/HMAC_AUTH.md) | `api-key` authentication |
| [`docs/config.md`](docs/config.md) | `app.yaml` / `app-job.yaml` and environment configuration |

## Architecture

```text
src/
  api/          Axum handlers and routes
  models/       Request, response, and event data
  services/     Business operations; service-to-service dependencies use traits
  repository/   PostgreSQL adapters
  infras/       PostgreSQL, Kafka, and HTTP clients
  main.rs       API binary (`data-notification`)
  bin/          notification_job.rs: Kafka worker binary (`notification-job`)
  bootstrap/    build.rs wires concrete implementations behind service traits; load.rs loads the channel and source memory caches; logging.rs sets up tracing
```

Keep handlers thin: parse/normalize/validate requests, call a service trait, and return `ApiResponse`. Put orchestration in services and SQL in repositories. Keep concrete service implementations wired in `bootstrap/`.

Repositories own SQL and return domain models. Put all PostgreSQL `Row` to model conversion and persisted-value parsing in the corresponding model's `map_row(&Row) -> AppResult<Self>` method; repository methods should query rows and call that mapper rather than reading columns themselves.

Write SQL as raw string literals (`r#"..."#`), one clause per line, with the opening and closing delimiters on their own lines.

## Error handling

- Every fallible function returns `AppResult<T>` (`Result<T, AppError>`, defined in `src/error.rs`). Do not use bare `Result<T, E>`, `Result<T, String>`, or `anyhow::Result`; `anyhow` is not a dependency. This applies to all layers, including `infras/`, `config/`, `bootstrap/`, and both `main` functions.
- Build errors with `AppError::bad_request` / `not_found` / `conflict` / `internal`. Wrap a foreign error with `AppError::internal_with("what failed", err)`, or add a `From` impl in `src/error.rs` when `?` should convert it directly.
- Validation methods such as `validate()` return `AppResult<()>` with `AppError::bad_request`; handlers turn the error into a response with `err.to_response()`.
- A function that cannot fail returns the value itself, not a `Result`.
- Exceptions are only signatures dictated by an external trait: `FromRequest::from_request`, `std::fmt::Result`, and `reqwest_middleware::Result` in the HTTP middleware.

## Routes

| Method | Path | Auth |
|---|---|---|
| `GET` | `/health` | Public |
| `POST` | `/notification/notify` | `api-key` |
| `GET` | `/notification/sources/{code}` | `api-key` |
| `POST` | `/notification/sources` | `api-key` |
| `PATCH` | `/notification/sources/metadata` | `api-key` |
| `GET` | `/notification/targets/{code}` | `api-key` |
| `POST` | `/notification/targets` | `api-key` |
| `PATCH` | `/notification/targets/metadata` | `api-key` |
| `POST` | `/notification/templates` | `api-key` |

There are no create/list/get/update/delete notification routes.

## Configuration

- The API loads `app.yaml` and `notification-job` loads `app-job.yaml`; `CONFIG_PATH` overrides either.
- `app-job.yaml` sets `kafka.consumer.enabled: true` and its own `kafka.client_id`; the job refuses to start when the consumer is disabled or Kafka is not connected.
- Non-development secret overrides include `POSTGRES_PASSWORD`, `KAFKA_BROKERS`, `KAFKA_SASL_USERNAME`, and `KAFKA_SASL_PASSWORD`.
- `auth.api_key` comes from config/ConfigMap; do not commit secrets.
