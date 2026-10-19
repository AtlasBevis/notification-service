# Database setup and notification flow

`definitions` stores the send configuration. `notifications` and `deliveries` are runtime rows created internally by `POST /notification/noti/notify`; the API does not expose notification CRUD.

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
| `003_targets.sql` | Reusable destination groups for EMAIL, MSTEAMS, and WEBHOOK |
| `004_templates.sql` | `templates` (name, status) and `template_versions` (versioned title, content, format; `JSON` format is a webhook body) |
| `005_definitions.sql` | `code` to source/channel mapping, and definition-to-target groups with the template per target |
| `006_notifications.sql` | Accepted request and trace ID uniqueness |
| `007_deliveries.sql` | Rendered message and current delivery state |
| `008_delivery_attempts.sql` | Dispatch attempt history |
| `009_webhook_config.sql` | Webhook endpoint configuration linked to a target |
| `010_webhook_auth_configs.sql` | Static or token endpoint authentication per webhook target |

After the migrations, create target groups with the matching `channel_id`, then link one or more targets to each definition through `definition_targets`. Sources, channels, and targets have no `metadata` column. EMAIL recipients and MSTEAMS webhook URLs are not stored yet, so sends to EMAIL and MSTEAMS targets fail until they are added. Each WEBHOOK target has one `webhook_configs` row, with its own endpoint, headers, and `webhook_auth_configs` row. The webhook body is a `format = 'JSON'` template version: `definition_targets.template_id` selects it for that target (`definitions` has no template column). When it is NULL, that WEBHOOK target receives the notify request's `variables` unchanged as its body. EMAIL and MSTEAMS require a template and render one message from the first linked target that has a `template_id`. String placeholders such as `{{date_key}}` are filled from the notify request's `variables`. For example, the template content can be `{"requestId":"{{requestId}}","data":{"date_key":"{{date_key}}"}}`. The worker sends each target its own rendered payload and records one aggregate delivery result.

## Runtime lifecycle

```text
POST /notification/noti/notify
  -> resolve active definition and configuration
  -> render the template's latest ACTIVE version
  -> transactionally insert notifications + deliveries
  -> return 202 Accepted
  -> API publishes to Kafka; notification-job consumes and dispatches
  -> update delivery state and append delivery_attempts
```

The unique `(source, trace_id)` constraint prevents duplicate accepted requests. Delivery retries update the existing delivery and append attempt rows; they do not create another notification or delivery.
