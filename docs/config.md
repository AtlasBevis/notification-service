# Configuration

`config.yaml` is loaded once at startup in `app::run_api()` via `load_config()`.

1. Path from env `CONFIG_PATH` (default `config.yaml`).
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

Templates live in Postgres (`templates` table), loaded into memory at API/consumer startup — not from the filesystem.
