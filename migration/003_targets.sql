-- Where a message is delivered. channel_code must match the route that uses this target.
-- EMAIL:    { "to": [], "cc": [], "bcc": [] }
-- MSTEAMS:  { "url": "https://..." }
-- WEBHOOK:  { "url": "https://internal-service/hooks/notify", "method": "POST" }

CREATE TABLE targets (
  code              VARCHAR(50)  PRIMARY KEY,
  channel_code      VARCHAR(50)  NOT NULL REFERENCES channels (code),
  created_at        timestamptz  NOT NULL DEFAULT now(),
  updated_at        timestamptz  NOT NULL,

  name              VARCHAR(255) NOT NULL,
  status            VARCHAR(20)  NOT NULL DEFAULT 'ACTIVE',
  metadata          JSONB        NOT NULL DEFAULT '{}'::jsonb,

  CONSTRAINT chk_targets_status CHECK (status IN ('ACTIVE', 'INACTIVE'))
);

INSERT INTO targets (code, channel_code, name, metadata, updated_at) VALUES
  (
    'TEAM_MIS',
    'MSTEAMS',
    'MIS Teams webhook',
    '{"url": ""}'::jsonb,
    now()
  ),
  (
    'OPS_EMAIL',
    'EMAIL',
    'Ops email group',
    '{"to": [], "cc": [], "bcc": []}'::jsonb,
    now()
  ),
  (
    'ORCHESTRATOR',
    'WEBHOOK',
    'Internal orchestrator',
    '{"url": "", "method": "POST"}'::jsonb,
    now()
  );
