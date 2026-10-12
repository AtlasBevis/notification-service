--  Callers trigger `code`.
-- level: INFO, WARN, ERROR, CRITICAL

CREATE TABLE definitions (
  id                BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  created_at        timestamptz  NOT NULL DEFAULT now(),
  updated_at        timestamptz,

  code              VARCHAR(50)  NOT NULL UNIQUE,
  name              VARCHAR(255) NOT NULL,
  description       VARCHAR(255),
  status            VARCHAR(20)  NOT NULL DEFAULT 'ACTIVE',
  level             VARCHAR(20)  NOT NULL DEFAULT 'INFO',
  
  source_id         INT   NOT NULL REFERENCES sources (id),
  channel_id        INT   NOT NULL REFERENCES channels (id),
  template_id       INT   REFERENCES templates (id)
);

-- A definition may deliver to multiple destination groups on its channel.
-- payload is the per-target webhook body template; {{variable}} placeholders
-- are populated from the incoming notification variables at dispatch time.
CREATE TABLE definition_targets (
  id                  BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  definition_id       BIGINT NOT NULL REFERENCES definitions (id) ON DELETE CASCADE,
  target_id           INT NOT NULL REFERENCES targets (id) ON DELETE RESTRICT,
  payload             JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at          timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_definition_targets_definition_id ON definition_targets (definition_id);
