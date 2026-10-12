# Data Notification

HTTP notification microservice (Rust + Axum + Kafka).

Caller chỉ gọi `POST /notification/notify` với `source`, `code`, `trace_id` và `variables` để gửi notification. Service tra `definitions`, render template, tự ghi bản ghi runtime vào `notifications` và `deliveries`, rồi gửi qua `EMAIL`, `MSTEAMS` hoặc `WEBHOOK`. Không có API tạo, xem danh sách, cập nhật hoặc xóa notification; `definitions` được cấu hình trong database.

`definitions` liên kết với một hoặc nhiều nhóm đích qua `definition_targets`. EMAIL gộp người nhận từ các nhóm; MSTEAMS gửi riêng tới từng Teams; WEBHOOK gửi riêng tới mỗi target có cấu hình trong `webhook_configs`. Mỗi webhook target có auth riêng (`STATIC` hoặc `TOKEN_ENDPOINT`) và header riêng. Worker ghi kết quả tổng hợp vào `delivery_attempts`.

`WebhookService` dùng `HttpClient` chung được tạo bởi `Infrastructure` và inject vào service; cả request webhook lẫn token đều đi qua HTTP client và middleware dùng chung.

- Request flow: [`docs/api_flow.md`](docs/api_flow.md)
- Database setup: [`migration/FLOW.md`](migration/FLOW.md)
- Tài liệu khác: [`docs/`](docs/)
- Hướng dẫn AI: [`CLAUDE.md`](CLAUDE.md)

Schema nằm trong [`migration/`](migration/), theo thứ tự:

1. `sources`
2. `channels`
3. `targets`
4. `webhook_configs`
5. `webhook_auth_configs`
6. `templates`, `template_versions`
7. `definitions`
8. `notifications`
9. `deliveries`
10. `delivery_attempts`

Webhook configuration is created by migrations `009` and `010`; see [`migration/FLOW.md`](migration/FLOW.md) for the migration order and auth modes.

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
