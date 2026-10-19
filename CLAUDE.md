# CLAUDE.md — Data Notification

Guidance for AI assistants working on this Rust + Axum + Kafka notification service.

Coding rules are in `docs/RULES.md` and apply to every change. Read them before editing code:

@docs/RULES.md

## What the service does

Callers send a notification by `code`; the service looks up the matching definition, renders its template, and delivers to `EMAIL`, `MSTEAMS`, or `WEBHOOK`.

The project builds two binaries:

| Binary | Config | Role |
|---|---|---|
| `data-notification` | `app.yaml` | HTTP + gRPC API. Validates, records, and publishes to Kafka (`NotifyService`). Never calls a channel itself. |
| `notification-job` | `app-job.yaml` | Kafka consumer. Dispatches to the channel and records the result (`ConsumerService`). |

## Send flow

- The send endpoint is `POST /notification/noti/notify` with `source`, `code`, `trace_id`, and `variables`.
- An accepted send inserts one row into `notifications` and one row into `deliveries` in a transaction, then publishes to Kafka.
- A send fails when Kafka is not connected.
- Internal callers can use gRPC instead: `notification.v1.Notification/Notify` (`proto/notification.proto`, served from `src/grpc/` on `server.grpc.port`). It runs the same flow and checks the `api-key` metadata entry.
- Every other HTTP route also has a gRPC method in the same proto (`Sources`, `Targets`, `Templates`, `Definitions`, `Webhooks`); see `docs/api_flow.md` for the mapping.
- Channels and sources are cached in memory at startup (`bootstrap/load.rs`). Targets are not cached; they are read from Postgres on every use.

## Definitions and templates

- A definition maps a `code` to a source, a channel, and one or more targets (`definition_targets`).
- Definitions are managed through `/notification/definitions` (list, get, create, update, delete). Create and update share `SaveDefinitionRequest`.
- `DefinitionService` resolves `source_code`, `channel_code`, and each `target_code`, and rejects what a send would reject: a target on another channel, a template format that does not match the channel, or EMAIL/MSTEAMS without any template.
- An update replaces `definition_targets` in the given order.
- Templates are selected per target by `definition_targets.template_id`; `definitions` has no `template_id`.
- A WEBHOOK target with a template sends that `format = 'JSON'` template as its request body; without one it sends the request's `variables` unchanged.
- EMAIL and MSTEAMS require a template and render one title/body from the first linked target that has one.

## Webhooks

- Webhook configs are managed through `/notification/webhooks`, addressed by target code (one config per WEBHOOK target). Create and update share `SaveWebhookRequest` and write `webhook_configs` and `webhook_auth_configs` in one transaction; update replaces both and drops the cached token.
- Responses never include `header_value` or the token request headers and body (`has_header_value` says whether a static value is set). The request log replaces `/webhooks` bodies with a placeholder.
- Every webhook config must have a row in `webhook_auth_configs`; a missing row is `AppError::NotFound`.
- `auth_mode = STATIC` sends `header_value` as `header_name`.
- `auth_mode = TOKEN_ENDPOINT` obtains and caches a token from `token_url`, then sends it using `header_name` and `header_prefix`.
- `webhook_configs.headers` contains additional static outgoing headers; auth header values take precedence on duplicate names.
- Each selected target endpoint is called once per dispatch.

## Architecture

```text
src/
  api/          One controller struct per domain (handlers are its methods) and routes.rs
  grpc/         tonic server: one struct per service (notify.rs, sources.rs, targets.rs, templates.rs, definitions.rs, webhooks.rs), convert.rs (Struct/Timestamp/Status), auth.rs (api-key interceptor), mod.rs (generated code + serve)
  models/       Request, response, and event data
  services/     Business operations; service-to-service dependencies use traits
  repository/   PostgreSQL adapters
  infras/       PostgreSQL, Kafka, and HTTP clients
  state.rs      AppState (services and controllers); passed around as SharedState = Arc<AppState>
  bootstrap/    build.rs wires repositories, services, and controllers; load.rs loads the channel and source memory caches; logging.rs sets up tracing
  main.rs       API binary (`data-notification`)
  bin/          notification_job.rs: Kafka worker binary (`notification-job`)
```

## Routes

| Method | Path | Auth |
|---|---|---|
| `GET` | `/health` | Public |
| `POST` | `/notification/noti/notify` | `api-key` |
| `GET` | `/notification/sources` | `api-key` |
| `GET` | `/notification/sources/{code}` | `api-key` |
| `POST` | `/notification/sources` | `api-key` |
| `PATCH` | `/notification/sources/metadata` | `api-key` |
| `GET` | `/notification/targets` | `api-key` |
| `GET` | `/notification/targets/{code}` | `api-key` |
| `POST` | `/notification/targets` | `api-key` |
| `PATCH` | `/notification/targets/metadata` | `api-key` |
| `POST` | `/notification/templates` | `api-key` |
| `GET` | `/notification/definitions` | `api-key` |
| `POST` | `/notification/definitions` | `api-key` |
| `GET` | `/notification/definitions/{code}` | `api-key` |
| `PUT` | `/notification/definitions/{code}` | `api-key` |
| `DELETE` | `/notification/definitions/{code}` | `api-key` |
| `GET` | `/notification/webhooks` | `api-key` |
| `POST` | `/notification/webhooks` | `api-key` |
| `GET` | `/notification/webhooks/{target_code}` | `api-key` |
| `PUT` | `/notification/webhooks/{target_code}` | `api-key` |
| `DELETE` | `/notification/webhooks/{target_code}` | `api-key` |

- The `/notification` prefix is `server.base_path` in config; `/health` is always at the root.
- Protected routes use the `api-key` header.
- List routes (`GET /sources`, `/targets`, `/definitions`, `/webhooks`) take `?page=&size=` as `PageRequest` (`u32`; missing or `0` takes the default page 1 / size 20; `size` at most 100) and return `PageData { items, total }` (`src/models/page.rs`). Repositories page with `LIMIT/OFFSET` and count `total` with `COUNT(*)` (`list_page`); gRPC `List*` RPCs take the `PageRequest` message.
- There are no create/list/get/update/delete routes for notifications themselves.
- gRPC on `server.grpc.port` (metadata `api-key`): `notification.v1.Notification/Notify` plus `Sources`, `Targets`, `Templates`, `Definitions`, `Webhooks`, one method per route above.

## Configuration

- The API loads `app.yaml` and `notification-job` loads `app-job.yaml`; `CONFIG_PATH` overrides either.
- Default listen address is `0.0.0.0:8080`.
- A field missing from `app.yaml` / `app-job.yaml` fails startup with a parse error (only the `auth` section is optional).
- `app-job.yaml` sets `kafka.consumer.enabled: true` and its own `kafka.client_id`; the job refuses to start when the consumer is disabled or Kafka is not connected.
- Secret overrides from env (applied when set) include `POSTGRES_PASSWORD`, `KAFKA_BROKERS`, `KAFKA_SASL_USERNAME`, and `KAFKA_SASL_PASSWORD`.
- `auth.api_key` comes from config/ConfigMap.

## Documentation

| File | Contents |
|---|---|
| [`docs/RULES.md`](docs/RULES.md) | Coding rules: layers, controllers, state, SQL, errors, testing |
| [`docs/api_flow.md`](docs/api_flow.md) | Request and delivery flow, route list |
| [`docs/how-to-write-new-api.md`](docs/how-to-write-new-api.md) | Adding an endpoint, step by step |
| [`docs/config.md`](docs/config.md) | `app.yaml` / `app-job.yaml` and environment configuration |
| [`docs/HMAC_AUTH.md`](docs/HMAC_AUTH.md) | `api-key` authentication |
| [`migration/FLOW.md`](migration/FLOW.md) | Migration order and database lifecycle |
