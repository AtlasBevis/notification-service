# Configuration

Each binary loads its own file once at startup via `load_config(default_path)`:

| Binary | `main` | Default file | Role |
|---|---|---|---|
| `data-notification` | `src/main.rs` | `app.yaml` | HTTP API; publishes to Kafka |
| `notification-job` | `src/bin/notification_job.rs` | `app-job.yaml` | Kafka consumer; dispatches to channels |

The two files differ in `server.profile` (`app` / `job`), `kafka.client_id`, and `kafka.consumer.enabled` (`false` for the API, `true` for the job). `client_id` is only a label for broker logs, metrics, and quotas; it does not need to differ, but separate values keep the two processes distinguishable.

1. Path from env `CONFIG_PATH`, otherwise the binary's default file.
2. Parse YAML → `Config`.
3. Init profile from `server.profile`.
4. Secret override when profile is not `dev`: `POSTGRES_PASSWORD`, `KAFKA_BROKERS`, `KAFKA_SASL_USERNAME`, `KAFKA_SASL_PASSWORD`, `TEAMS_URL`, `EMAIL_USERNAME`, `EMAIL_PASSWORD`.
   `auth.api_key` comes from config (ConfigMap on k8s), not from Secrets/env override.
5. `config::set(...)` (`OnceLock`). Middleware/service read via `config::load()`.

No hot-reload. On k8s, update secret/config then restart the pod.

Main sections: `server`, `auth`, `postgres`, `kafka`, `notification`.

`notification` section:

- `teams.url` (or env `TEAMS_URL`): Teams webhook URL.
- `email.username` / `email.password` (or env `EMAIL_USERNAME` / `EMAIL_PASSWORD`): SMTP credentials. Recipients come from the request. SMTP host from env `SMTP_HOST` (default `smtp.office365.com`).

Channels and sources are loaded into memory at API/job startup. Targets and templates are read from Postgres on demand.
