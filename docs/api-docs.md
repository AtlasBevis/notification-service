# API documentation

| URL | Purpose |
|-----|---------|
| `/swagger-ui/` | Swagger UI |
| `/api-docs/openapi.json` | OpenAPI 3 JSON (OpenMetadata REST connector) |

## Local

- Swagger UI: http://localhost:8080/swagger-ui/
- OpenAPI JSON: http://localhost:8080/api-docs/openapi.json

## OpenMetadata

Set **OpenAPI Schema URL** to:

```text
https://<host>/api-docs/openapi.json
```

Example: `https://<host>/api-docs/openapi.json`

This path is public (no auth). Protected API routes require header `api-key`.

## Response envelope

Every API returns **only** `ApiResponse` (`src/models/api_response.rs`).

PascalCase JSON: `Code`, `Message`, optional `Data`. Omit `Data` when empty.

```json
{
  "Code": 202,
  "Message": "The request has been accepted"
}
```

Handler return type is `ApiResponse` / `ApiResponse<T>`. Errors: `err.to_response()`.

---

## Example: `POST /notification/notify`

Auth: header `api-key`.

**Request** (`NotifyRequest`)

```json
{
  "code": "AIRFLOW_FAIL",
  "trace_id": "trace-001",
  "variables": {
    "dag_id": "daily_etl",
    "task_id": "extract"
  }
}
```

**Response** is always `ApiResponse` (no other wrapper).

| HTTP | Code | Message | Data |
|------|------|---------|------|
| 202 | 202 | The request has been accepted | *(omit)* |
| 400 | 400 | code is required | *(omit)* |
| 401 | 401 | Missing api-key / Invalid api-key | *(omit)* |
| 404 | 404 | … | *(omit)* |
| 409 | 409 | delivery already exists … | *(omit)* |
| 503 | 503 | Service Unavailable | *(omit)* |

```json
{
  "Code": 202,
  "Message": "The request has been accepted"
}
```

```json
{
  "Code": 400,
  "Message": "code is required"
}
```
