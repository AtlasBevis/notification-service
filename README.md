# Data Notification

HTTP notification microservice (Rust + Kafka)

- Tài liệu kiến trúc / how-to: [`docs/`](docs/)
- Hướng dẫn AI: [`CLAUDE.md`](CLAUDE.md)

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
