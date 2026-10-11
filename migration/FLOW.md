# Flow Notification

`notification_definitions` là catalog mã thông báo. `notifications` là một request đã nhận. `deliveries` là một lần gửi tới một route. Retry ghi `delivery_attempts` trên cùng delivery.

Chi tiết request: [`docs/api_flow.md`](../docs/api_flow.md).

## Bảng

Áp dụng trên database trống, theo thứ tự file.

| File | Bảng | Vai trò |
|------|------|---------|
| `001_sources.sql` | `sources` | Ai được gọi API |
| `002_channels.sql` | `channels` | `EMAIL`, `MSTEAMS`, `WEBHOOK` |
| `003_targets.sql` | `targets` | Đích có tên, một `channel_code` |
| `004_templates.sql` | `templates` | Nội dung, version trên cùng bảng |
| `005_notification_definitions.sql` | `notification_definitions` | Mã sự kiện: `code`, `source_code`, `level` |
| `006_notification_routes.sql` | `notification_routes` | Fan-out của một definition |
| `007_notifications.sql` | `notifications` | Request: `definition_id`, `idempotency_key`, `variables` |
| `008_deliveries.sql` | `deliveries` | Một route của request, payload đã render |
| `009_delivery_attempts.sql` | `delivery_attempts` | Từng lần gửi |

```bash
psql -U notification -d notification -f migration/001_sources.sql
psql -U notification -d notification -f migration/002_channels.sql
psql -U notification -d notification -f migration/003_targets.sql
psql -U notification -d notification -f migration/004_templates.sql
psql -U notification -d notification -f migration/005_notification_definitions.sql
psql -U notification -d notification -f migration/006_notification_routes.sql
psql -U notification -d notification -f migration/007_notifications.sql
psql -U notification -d notification -f migration/008_deliveries.sql
psql -U notification -d notification -f migration/009_delivery_attempts.sql
```

## Vòng đời

```text
POST /notification/notify
        |
        | load notification_definitions by code
        | load ACTIVE notification_routes
        | one transaction
        v
notifications                         deliveries
  definition_id                            |
  source_code + code                       v
  idempotency_key = trace_id          PENDING
  variables                                |
                                           v
                                      SENT | FAILED
                                           |
                                           +---- delivery_attempts
```

Unique:

- `notification_definitions (code)`
- `notifications (source_code, idempotency_key)`
- `deliveries (notification_id, route_id)`
- `delivery_attempts (delivery_id, attempt_no)`

Gọi lại cùng `(source_code, idempotency_key)` trả **409**. Không tạo delivery mới.
