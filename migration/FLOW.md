# Database setup and notification flow

`definitions` stores the send configuration. `notifications` and `deliveries` are runtime rows created internally by `POST /notification/notify`; the API does not expose notification CRUD.

## Migration order

Apply migrations in numerical order to an empty database:

```powershell
Get-ChildItem migration\*.sql | Sort-Object Name | ForEach-Object {
    psql -U notification -d notification -f $_.FullName
}
```

| File | Tables / purpose |
|---|---|
| `001_sources.sql` | Sources allowed to call the service |
| `002_channels.sql` | Delivery channel catalog |
| `003_targets.sql` | Destinations attached to channels |
| `004_templates.sql` | Versioned message templates |
| `005_definitions.sql` | `code` to source/channel/target/template mapping |
| `006_notifications.sql` | Accepted request and trace ID uniqueness |
| `007_deliveries.sql` | Rendered message and current delivery state |
| `008_delivery_attempts.sql` | Dispatch attempt history |
| `009_webhook_config.sql` | Webhook endpoint and outgoing headers |
| `010_webhook_auth_configs.sql` | Static or token endpoint authentication |

After the migrations, add each notification definition directly to `definitions` and associate it with existing source, channel, target, and template rows. Each webhook channel needs a `webhook_configs` row and a `webhook_auth_configs` row. Use `auth_mode = STATIC` with `header_value`, or `auth_mode = TOKEN_ENDPOINT` with `token_url` and the token request/response fields. Put additional outgoing headers such as `{"service":"MIS"}` in `webhook_configs.headers`.

## Runtime lifecycle

```text
POST /notification/notify
  -> resolve active definition and configuration
  -> render the selected template
  -> transactionally insert notifications + deliveries
  -> return 202 Accepted
  -> dispatch through Kafka consumer, or directly when Kafka is disabled
  -> update delivery state and append delivery_attempts
```

The unique `(source, trace_id)` constraint prevents duplicate accepted requests. Delivery retries update the existing delivery and append attempt rows; they do not create another notification or delivery.
