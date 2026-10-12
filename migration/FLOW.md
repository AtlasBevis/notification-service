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
| `009_webhook_auth_configs.sql` | Webhook authentication configuration |
| `010_webhook_config.sql` | Webhook endpoint, method, timeout, and retries |
| `011_drop_webhook_auth_checks.sql` | Removes legacy webhook `CHECK` constraints; validation stays in the application |

After the migrations, add each notification definition directly to `definitions` and associate it with existing source, channel, target, and template rows. Configure webhook channels in `webhook_configs` as needed.

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
