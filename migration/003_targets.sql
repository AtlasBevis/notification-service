-- A reusable destination group for EMAIL, MSTEAMS, or WEBHOOK.
-- EMAIL:    { "to": [], "cc": [], "bcc": [] }
-- MSTEAMS:  { "url": "https://..." }
-- Definitions select one or more targets through definition_targets.

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
