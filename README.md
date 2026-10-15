# Data Notification

Service gửi thông báo nội bộ (Rust + Axum + Kafka + Postgres). Các hệ thống khác gọi một API duy nhất để gửi; service lo việc chọn kênh, render nội dung và giao tới `EMAIL`, `MSTEAMS` hoặc `WEBHOOK`.

## Tổng quan

Caller gọi `POST /notification/noti/notify` với `source`, `code`, `trace_id` và `variables`. `code` trỏ tới một `definition` đã cấu hình sẵn trong database, từ đó service biết gửi qua kênh nào, tới những target nào, dùng template nào. Không có API tạo, xem, sửa hay xoá notification.

Project build ra hai binary, mỗi binary một vai trò:

| Binary | Config | Vai trò |
|---|---|---|
| `data-notification` | `app.yaml` | HTTP API. Validate request, ghi `notifications` + `deliveries`, rồi publish lên Kafka. Không tự gửi ra kênh. |
| `notification-job` | `app-job.yaml` | Kafka consumer. Đọc topic, gửi ra kênh, retry và ghi kết quả vào `deliveries` / `delivery_attempts`. |

```text
Caller ──POST /notification/noti/notify──► data-notification ──► Kafka ──► notification-job ──► EMAIL / MSTEAMS / WEBHOOK
                                            │                                │
                                            └──────────── Postgres ──────────┘
```

API cần Kafka để nhận request: nếu Kafka không kết nối được, lệnh gửi bị từ chối và không ghi gì vào database. `(source, trace_id)` là khoá chống gửi trùng; gửi lại cùng cặp này trả `409 Conflict`.

## Cấu hình gửi

| Bảng | Ý nghĩa |
|---|---|
| `sources` | Hệ thống được phép gọi service |
| `channels` | Kênh gửi: `EMAIL`, `MSTEAMS`, `WEBHOOK` |
| `targets` | Nhóm đích dùng lại được, thuộc một kênh |
| `templates`, `template_versions` | Nội dung có phiên bản, thay `{{key}}` bằng `variables`; luôn dùng phiên bản `ACTIVE` mới nhất |
| `definitions`, `definition_targets` | `code` → source, kênh, và danh sách target kèm template của từng target |
| `webhook_configs`, `webhook_auth_configs` | Endpoint, header và cách xác thực (`STATIC` hoặc `TOKEN_ENDPOINT`) của mỗi webhook target |

Cách mỗi kênh dùng target:

- **EMAIL**: gộp người nhận của mọi target thành một email.
- **MSTEAMS**: gửi riêng tới từng target.
- **WEBHOOK**: gọi riêng endpoint của từng target. Body là template `JSON` của target đó; target không gắn template thì gửi nguyên `variables` của request.

## API

Mọi route dưới `/notification` cần header `api-key`; `GET /health` là public.

| Method | Path | Mục đích |
|---|---|---|
| `GET` | `/health` | Health check |
| `POST` | `/notification/noti/notify` | Gửi notification |
| `GET` / `POST` | `/notification/sources/{code}`, `/notification/sources` | Xem / tạo source |
| `PATCH` | `/notification/sources/metadata` | Sửa metadata của source |
| `GET` / `POST` | `/notification/targets/{code}`, `/notification/targets` | Xem / tạo target |
| `PATCH` | `/notification/targets/metadata` | Sửa metadata của target |
| `POST` | `/notification/templates` | Tạo phiên bản template mới |
| `GET` / `POST` | `/notification/definitions` | Liệt kê / tạo definition |
| `GET` / `PUT` / `DELETE` | `/notification/definitions/{code}` | Xem / sửa / xoá definition |
| `GET` / `POST` | `/notification/webhooks` | Liệt kê / tạo cấu hình webhook cho target WEBHOOK |
| `GET` / `PUT` / `DELETE` | `/notification/webhooks/{target_code}` | Xem / sửa / xoá cấu hình webhook |

Hệ thống nội bộ có thể gọi gRPC thay cho HTTP cho toàn bộ API (`notification.v1.Notification/Notify`, `Sources`, `Targets`, `Templates`, `Definitions`, `Webhooks`) ở cổng `50051`, contract tại [`proto/notification.proto`](proto/notification.proto). Mỗi method xử lý giống hệt route HTTP tương ứng, `api-key` gửi qua metadata. Chi tiết và ví dụ `grpcurl` ở [`docs/api_flow.md`](docs/api_flow.md).

## Chạy local

1. Tạo database và chạy các file trong [`migration/`](migration/) theo thứ tự số (xem [`migration/FLOW.md`](migration/FLOW.md)).
2. Sửa `app.yaml` và `app-job.yaml` cho khớp Postgres và Kafka của bạn. Mọi field đều bắt buộc; thiếu field thì process dừng khi khởi động.
3. Chạy hai process:

   ```bash
   cargo run --bin data-notification
   cargo run --bin notification-job
   ```

API lắng nghe HTTP ở `0.0.0.0:8080`, gRPC ở cổng `50051`, metrics Prometheus ở cổng `9090`. Biến môi trường `CONFIG_PATH` ghi đè đường dẫn file config; `POSTGRES_PASSWORD`, `KAFKA_BROKERS`, `KAFKA_SASL_USERNAME`, `KAFKA_SASL_PASSWORD` ghi đè các giá trị nhạy cảm.

## Tài liệu

- Luồng request và giao nhận: [`docs/api_flow.md`](docs/api_flow.md)
- Cấu hình: [`docs/config.md`](docs/config.md)
- Xác thực `api-key`: [`docs/HMAC_AUTH.md`](docs/HMAC_AUTH.md)
- Thêm API mới và quy ước code: [`docs/how-to-write-new-api.md`](docs/how-to-write-new-api.md)
- Schema và thứ tự migration: [`migration/FLOW.md`](migration/FLOW.md)
- Quy tắc code: [`docs/RULES.md`](docs/RULES.md)
- Hướng dẫn cho AI assistant: [`CLAUDE.md`](CLAUDE.md)

---

## Git workflow

### Nhánh chính

| Branch | Vai trò |
|--------|---------|
| `master` | Môi trường PROD |
| `staging` | Môi trường STAGING, (nhánh các feature chuẩn bị golive PROD) |
| `uat` | Môi trường UAT |
| `feature/<tên-feature>` | Phát triển tính năng / fix (vd. `feature/cdp`, `feature/QTDL-889`). |
| `fix/<issue >` | hotfix tính năng gấp |

### Workflow thường ngày

```text
staging
   │
   ▼
feature/<name>     ← code + MR/PR trên nhánh này
   │
   ▼
uat                ← merge (từ feature) để release cho Nghiệp vụ
```

1. **Checkout từ `staging`**
   ```bash
   git fetch origin
   git checkout staging
   git pull origin staging
   git checkout -b feature/<ten-feature>
   ```

2. **Làm việc trên `feature/<ten-feature>`** — commit, push, mở MR nếu team dùng MR.

3. **Merge vào `uat`** để deploy / release UAT (merge trực tiếp `feature/<ten-feature>` → `uat`):
   ```bash
   git checkout uat
   git pull origin uat
   git merge feature/<ten-feature>
   git push origin uat
   ```

### Golive

Khi release production **merge từ `feature/<ten-feature>` vào `staging`**.

```bash
git checkout staging
git pull origin staging
git merge feature/<ten-feature>
git push origin staging
```

```text
feature/<name>  ──golive──►  staging
```

**Note** Update thông tin release

### Quy định

- Luôn bắt đầu feature từ **`staging` mới nhất**.
- Đặt tên: `feature/<ticket-hoặc-mô-tả-ngắn>` (vd. `feature/QTDL-889`, `feature/cdp-track`).
- Không commit secret / password vào repo.
- Sau golive: có thể xóa `feature/...` đã merge nếu không còn cần.

---
