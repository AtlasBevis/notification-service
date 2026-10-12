-- Optional step 1 of a WEBHOOK delivery: fetch an access token before calling the webhook.
-- No row for a webhook_config => webhook is called directly (no auth step).
CREATE TABLE webhook_auth_configs (
  id                   INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  webhook_config_id    INT             NOT NULL REFERENCES webhook_configs (id) ON DELETE CASCADE,

  -- Token request
  token_url            VARCHAR(1024)   NOT NULL,
  method               VARCHAR(10)     NOT NULL DEFAULT 'POST',
  body_format          VARCHAR(10)     NOT NULL DEFAULT 'JSON',      -- JSON | FORM (x-www-form-urlencoded)
  request_headers      JSONB           NOT NULL DEFAULT '{}'::jsonb, -- {"x-client-id": "..."}
  request_body         JSONB           NOT NULL DEFAULT '{}'::jsonb, -- {"grant_type": "client_credentials", ...}
  timeout_seconds      INT             NOT NULL DEFAULT 10,

  -- Token response (dot path into JSON body, e.g. "data.access_token")
  token_path           VARCHAR(255)    NOT NULL DEFAULT 'access_token',
  expires_in_path      VARCHAR(255)             DEFAULT 'expires_in', -- seconds; NULL => default_ttl_seconds
  default_ttl_seconds  INT             NOT NULL DEFAULT 300,

  -- How the token is sent on the webhook call
  header_name          VARCHAR(100)    NOT NULL DEFAULT 'Authorization',
  header_prefix        VARCHAR(50)     NOT NULL DEFAULT 'Bearer ',

  created_at           timestamptz     NOT NULL DEFAULT now(),
  updated_at           timestamptz     NOT NULL DEFAULT now(),

  CONSTRAINT uq_webhook_auth_configs_webhook UNIQUE (webhook_config_id)
);
