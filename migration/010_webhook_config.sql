
CREATE TABLE webhook_configs (
  id                INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  channel_id        INT             NOT NULL REFERENCES channels (id),
  domain            VARCHAR(255)    NOT NULL,
  "path"            VARCHAR(255),
  method            VARCHAR(10)     NOT NULL DEFAULT 'POST',
  timeout_seconds   INT             NOT NULL DEFAULT 10,
  max_retries       INT             NOT NULL DEFAULT 3,
  
  created_at        timestamptz     NOT NULL DEFAULT now(),
  updated_at        timestamptz
);
