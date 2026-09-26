# API key authentication

Protected routes require a single header:

| Header | Description |
|--------|-------------|
| `api-key` | Value from `auth.api_key` in config (ConfigMap on k8s). |

```http
POST /notification/notify
api-key: <secret>
Content-Type: application/json
```

If `auth` is omitted or `auth.enabled: false`, the check is skipped.

`GET /health`, Swagger UI, and `/api-docs/openapi.json` are public.

Implementation: [`src/middlewares/auth.rs`](../src/middlewares/auth.rs).
