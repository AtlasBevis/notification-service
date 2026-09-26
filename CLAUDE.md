# CLAUDE.md — Data Notification (`data-notification`)

Hướng dẫn cho AI assistant khi làm việc trong repository này.

---

## Service là gì?

**data-notification** là HTTP microservice (Rust + Axum) dùng để:

1. Nhận thông báo từ các app nội bộ (`POST /notification/notify`)
2. Publish lên Kafka (API) — process `notification-consumer` gửi TEAMS / EMAIL
3. Bảo vệ endpoint bằng header `api-key` (logging middleware trên protected routes)
4. Xuất OpenAPI (`/api-docs/openapi.json`) cho Swagger UI

Default listen: `0.0.0.0:8080` (xem `config.yaml` / env).

---

## Tài liệu (`./docs`)

| File | Nội dung |
|------|----------|
| [`docs/api_flow.md`](docs/api_flow.md) | Request flow (Router → Logging → Auth → Handler → Service → Kafka) |
| [`docs/how-to-write-new-api.md`](docs/how-to-write-new-api.md) | Checklist thêm API |
| [`docs/HMAC_AUTH.md`](docs/HMAC_AUTH.md) | Header `api-key` |
| [`docs/config.md`](docs/config.md) | Load `config.yaml`, secret env override |

---

## Stack

| Area | Crate |
|------|--------|
| HTTP server | `axum` 0.7, `tokio` |
| Kafka | `rskafka` 0.6 |
| Postgres | `deadpool-postgres` + `tokio-postgres` |
| HTTP client | `reqwest` + `reqwest-middleware` |
| Auth | header `api-key` |
| OpenAPI | `utoipa`, `utoipa-swagger-ui` |
| Logging | `tracing` JSON |

---

## Cấu trúc

```
src/
  main.rs              # API process (`data-notification`)
  bin/notification_consumer.rs  # Kafka worker (`notification-consumer`)
  app.rs               # run_api() / run_notification_consumer()
  bootstrap/           # composition root
  api/                 # notify, sources, targets, templates, health, routes, OpenAPI
  middlewares/         # auth (api-key), logging, metrics
  services/            # notification, sources, targets, channels, templates, outbox
  models/              # send_request, channel, source, target, template, ...
  repository/          # channels, sources, targets, templates, notifications, deliveries, outbox (Postgres)
  infras/
    postgres/          # PostgresManager
    kafka/             # KafkaBus
    http/              # HttpClient
  config/
  state.rs
```

Luồng: `api (handler) → services → repository / infras (kafka / postgres / http)`

Templates: stored in Postgres `templates`, loaded into memory at startup (`TemplatesService`).

---

## Routes

| Path | Auth |
|------|------|
| `GET /health` | Public |
| `POST /notification` | `api-key` |
| `GET /notification/{uuid}` | `api-key` |
| `POST /notification/notify` | `api-key` |
| `GET /notification/sources/{code}` | `api-key` |
| `POST /notification/sources` | `api-key` |
| `PATCH /notification/sources/metadata` | `api-key` |
| `GET /notification/targets/{code}` | `api-key` |
| `POST /notification/targets` | `api-key` |
| `PATCH /notification/targets/metadata` | `api-key` |
| `POST /notification/templates` | `api-key` |
| `/swagger-ui`, `/api-docs/openapi.json` | Public |

Middleware (protected): `logging` → `auth` (`api-key`).

---

## Config

- `CONFIG_PATH` (default `config.yaml`)
- Secret override (non-dev): `POSTGRES_PASSWORD`, `KAFKA_BROKERS`, `KAFKA_SASL_USERNAME`, `KAFKA_SASL_PASSWORD`
- `auth.api_key` from config / ConfigMap (not Secrets env override)
