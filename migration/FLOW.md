# Flow Notification

Apply migrations in order (`001` → `007`) on an empty Postgres database.

| File | Table |
|------|--------|
| `001_sources.sql` | `sources` |
| `002_channels.sql` | `channels` (+ seed MSTEAMS / EMAIL metadata) |
| `003_targets.sql` | `targets` (destination catalog) |
| `004_templates.sql` | `templates` |
| `005_notifications.sql` | `notifications` (portal config: BIGINT id + uuid string) |
| `006_deliveries.sql` | `deliveries` (per-send audit) |
| `007_outbox.sql` | `outbox` (Kafka publish pipeline) |

```bash
psql -U notification -d notification -f migration/001_sources.sql
psql -U notification -d notification -f migration/002_channels.sql
psql -U notification -d notification -f migration/003_targets.sql
psql -U notification -d notification -f migration/004_templates.sql
psql -U notification -d notification -f migration/005_notifications.sql
psql -U notification -d notification -f migration/006_deliveries.sql
psql -U notification -d notification -f migration/007_outbox.sql
```

After seed, fill `channels.metadata` (MSTEAMS `url`, EMAIL SMTP) and `targets.metadata` (webhook URL / email lists).

---

## POST /notification

Create a notification config (UUID). Body: `name`, `source`, `target`, `channel`, `template`, optional `metadata`.

## POST /notification/notify

1. Body: `uuid`, `correlation_id`, `payload`, optional `kind`
2. Load ACTIVE notification config → resolve source / target / channel / template
3. Dedup `(notification_id, correlation_id)` on `deliveries` → **409** if exists
4. Render template with payload placeholders
5. Insert `deliveries` + `outbox` QUEUED (same transaction when Kafka on)
6. Return **202 Accepted**

## Outbox publisher (API process)

1. Claim `outbox` QUEUED → PROCESSING
2. Publish to Kafka
3. Success → SENT; failure → FAILED

## Worker (`notification-consumer`)

1. Consume Kafka message
2. Dispatch by channel (email / msteams)
3. Retry up to 3 times on failure
