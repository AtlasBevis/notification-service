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

Auth / application envelope uses PascalCase fields: `Status`, `Code`, `Message`, and optional `Data`.
