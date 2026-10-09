# Flow Notification

Apply migrations in order (`001` → `007`) on an empty Postgres database.

| File | Table |
|------|--------|
| `001_sources.sql` | `sources` — AIRFLOW, ETL, GITLAB, REVENUE, SPARK |
| `002_channels.sql` | `channels` — EMAIL, MSTEAMS, WEBHOOK |
| `003_targets.sql` | `targets` — destination bound to one channel |
| `004_templates.sql` | `templates` — versioned body (`TEXT`, `HTML`, `ADAPTIVE_CARD`, `JSON`) |
| `005_notifications.sql` | `notifications` — event catalog (`code`, `kind`, `level`) |
| `006_notification_routes.sql` | `notification_routes` — fan-out to channel + target + template |
| `007_deliveries.sql` | `deliveries` — one rendered send per route |

```bash
psql -U notification -d notification -f migration/001_sources.sql
psql -U notification -d notification -f migration/002_channels.sql
psql -U notification -d notification -f migration/003_targets.sql
psql -U notification -d notification -f migration/004_templates.sql
psql -U notification -d notification -f migration/005_notifications.sql
psql -U notification -d notification -f migration/006_notification_routes.sql
psql -U notification -d notification -f migration/007_deliveries.sql
```

After seed, fill `channels.metadata` (EMAIL SMTP), `targets.metadata` (Teams URL, email lists, webhook URL).

---

## POST /notification/notify

1. Body: `code`, `trace_id`, `variables`. Caller `service` must match `notifications.source_code`.
2. Load the ACTIVE notification and its ACTIVE routes.
3. For each route, render the template. Reject the route when `targets.channel_code` differs from the route channel.
4. Dedup `(route_id, trace_id)` on `deliveries` → **409** if that route was already sent for this trace.
5. Insert `deliveries` (`QUEUED`). When Kafka is on, publish the message directly. When Kafka is off, the API process dispatches the channel.
6. Return **202 Accepted**.

Webhook body is a fixed envelope (`code`, `source`, `trace_id`, `level`, `kind`, `title`, `data`). Downstream services trigger their own work from that payload.

## Worker (`notification-consumer`)

1. Consume Kafka message
2. Dispatch by channel (EMAIL / MSTEAMS / WEBHOOK)
3. Set `deliveries.status` to SENT or FAILED
4. Retry up to 3 times on failure
