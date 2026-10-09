-- How a message is transported. Destination addresses live on targets.
-- EMAIL metadata:    { "host", "username", "password" }
-- MS_TEAMS metadata: {}  (incoming webhook URL is targets.metadata.url)
-- WEBHOOK metadata:  { "timeout_ms": 5000 }

CREATE TABLE channels (
  code              VARCHAR(50)  PRIMARY KEY,
  created_at        timestamptz  NOT NULL DEFAULT now(),
  updated_at        timestamptz  NOT NULL,

  name              VARCHAR(255) NOT NULL,
  status            VARCHAR(20)  NOT NULL DEFAULT 'ACTIVE',
  metadata          JSONB        NOT NULL DEFAULT '{}'::jsonb,

  CONSTRAINT chk_channels_status CHECK (status IN ('ACTIVE', 'INACTIVE'))
);

INSERT INTO channels (code, name, metadata, updated_at) VALUES
  ('EMAIL',    'Email',            '{"host": "", "username": "", "password": ""}'::jsonb, now()),
  ('MSTEAMS',  'Microsoft Teams',  '{}'::jsonb, now()),
  ('WEBHOOK',  'HTTP webhook',     '{"timeout_ms": 5000}'::jsonb, now());
