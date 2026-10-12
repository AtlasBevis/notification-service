# CLAUDE.md — Data Notification

Repository guidance for AI assistants working on this Rust + Axum + Kafka notification service.

## Service behavior

- The send endpoint is `POST /notification/notify` with `source`, `code`, `trace_id`, and `variables`.
- `definitions` is configuration provisioned in the database. The service does not expose notification CRUD endpoints.
- An accepted send internally inserts one row into `notifications` and one row into `deliveries` in a transaction.
- Delivery channels are `EMAIL`, `MSTEAMS`, and `WEBHOOK`. Kafka dispatches asynchronously when enabled; otherwise the API dispatches directly.
- Protected API routes use the `api-key` header. `GET /health` is public.

Default listen address is `0.0.0.0:8080` (see `config.yaml` and environment overrides).

Every webhook config must have a row in `webhook_auth_configs`; a missing row is `AppError::NotFound`. `auth_mode = STATIC` sends `header_value` as `header_name`. `auth_mode = TOKEN_ENDPOINT` obtains and caches a token from `token_url`, then sends it using `header_name` and `header_prefix`. `webhook_configs.headers` contains additional static outgoing headers; auth header values take precedence on duplicate names. Never log or serialize static secrets or tokens. Each delivery calls the webhook endpoint once.

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
| [`docs/config.md`](docs/config.md) | `config.yaml` and environment configuration |

## Architecture

```text
src/
  api/          Axum handlers and routes
  models/       Request, response, and event data
  services/     Business operations; service-to-service dependencies use traits
  repository/   PostgreSQL adapters
  infras/       PostgreSQL, Kafka, and HTTP clients
  bootstrap/    Concrete implementations wired behind service traits
```

Keep handlers thin: parse/normalize/validate requests, call a service trait, and return `ApiResponse`. Put orchestration in services and SQL in repositories. Keep concrete service implementations wired in `bootstrap/`.

Repositories own SQL and return domain models. Put all PostgreSQL `Row` to model conversion and persisted-value parsing in the corresponding model's `map_row(&Row) -> AppResult<Self>` method; repository methods should query rows and call that mapper rather than reading columns themselves.

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

- `CONFIG_PATH` defaults to `config.yaml`.
- Non-development secret overrides include `POSTGRES_PASSWORD`, `KAFKA_BROKERS`, `KAFKA_SASL_USERNAME`, and `KAFKA_SASL_PASSWORD`.
- `auth.api_key` comes from config/ConfigMap; do not commit secrets.
