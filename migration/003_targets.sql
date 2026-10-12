-- Where a message is delivered. channel_id must match the route that uses this target.
-- EMAIL:    { "to": [], "cc": [], "bcc": [] }
-- MSTEAMS:  { "url": "https://..." }
-- WEBHOOK destinations are configured in webhook_configs.

CREATE TABLE targets (
  id                INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  code              VARCHAR(50)  NOT NULL UNIQUE,
  created_at        timestamptz  NOT NULL DEFAULT now(),
  updated_at        timestamptz  NOT NULL,

  name              VARCHAR(255) NOT NULL,
  channel_id        INT          NOT NULL REFERENCES channels (id),
  status            VARCHAR(20)  NOT NULL DEFAULT 'ACTIVE',
  metadata          JSONB        NOT NULL DEFAULT '{}'::jsonb
);

-- Example:
-- INSERT INTO targets (code, channel_id, name, metadata, updated_at)
-- SELECT v.code, c.id, v.name, v.metadata, now()
-- FROM (
--   VALUES
--     ('TEAM_MIS', 'MSTEAMS', 'MIS Teams webhook', '{"url": ""}'::jsonb),
--     ('OPS_EMAIL', 'EMAIL', 'Ops email group', '{"to": [], "cc": [], "bcc": []}'::jsonb),
--     ('ORCHESTRATOR', 'WEBHOOK', 'Internal orchestrator', '{}'::jsonb)
-- ) AS v(code, channel_code, name, metadata)
-- JOIN channels c ON c.code = v.channel_code;
