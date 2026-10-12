-- One authentication configuration per webhook config.
-- STATIC uses header_value directly; TOKEN_ENDPOINT fetches and caches a token.
CREATE TABLE webhook_auth_configs (
  id                   INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  webhook_config_id    INT             NOT NULL REFERENCES webhook_configs (id) ON DELETE CASCADE,

  auth_mode            VARCHAR(20)     NOT NULL DEFAULT 'STATIC', -- STATIC | TOKEN_ENDPOINT
  header_name          VARCHAR(100)    NOT NULL DEFAULT 'x-api-key',
  header_value         TEXT,

  token_url            VARCHAR(1024),
  token_method          VARCHAR(10)     NOT NULL DEFAULT 'POST',
  token_body_format     VARCHAR(10)     NOT NULL DEFAULT 'JSON', -- JSON | FORM
  request_headers       JSONB           NOT NULL DEFAULT '{}'::jsonb,
  request_body          JSONB           NOT NULL DEFAULT '{}'::jsonb,
  timeout_seconds       INT             NOT NULL DEFAULT 10,
  token_path            VARCHAR(255)    NOT NULL DEFAULT 'access_token',
  expires_in_path       VARCHAR(255)             DEFAULT 'expires_in',
  default_ttl_seconds   INT             NOT NULL DEFAULT 300,
  header_prefix         VARCHAR(50)     NOT NULL DEFAULT 'Bearer ',

  created_at            timestamptz     NOT NULL DEFAULT now(),
  updated_at            timestamptz,

  CONSTRAINT uq_webhook_auth_configs_webhook UNIQUE (webhook_config_id)
);
