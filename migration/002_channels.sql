-- How a message is transported. Destination addresses live on targets.
-- EMAIL metadata:    { "host", "username", "password" }
-- MS_TEAMS metadata: {}  (incoming webhook URL is targets.metadata.url)
-- WEBHOOK metadata:  { "timeout_ms": 5000 }

CREATE TABLE channels (
  id                INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  code              VARCHAR(50)  NOT NULL UNIQUE,
  created_at        timestamptz  NOT NULL DEFAULT now(),
  updated_at        timestamptz,

  name              VARCHAR(255) NOT NULL,
  status            VARCHAR(20)  NOT NULL DEFAULT 'ACTIVE',
  metadata          JSONB        NOT NULL DEFAULT '{}'::jsonb
);

-- data
INSERT INTO channels (code, name) VALUES
  ('EMAIL',    'Email'          ),
  ('MSTEAMS',  'Microsoft Teams'),
  ('WEBHOOK',  'HTTP Webhook')
;
