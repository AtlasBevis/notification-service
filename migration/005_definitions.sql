--  Callers trigger `code`.
-- level: INFO, WARN, ERROR, CRITICAL

CREATE TABLE definitions (
  id                INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  created_at        timestamptz  NOT NULL DEFAULT now(),
  updated_at        timestamptz,

  code              VARCHAR(50)  NOT NULL UNIQUE,
  name              VARCHAR(255) NOT NULL,
  description       VARCHAR(255),
  status            VARCHAR(20)  NOT NULL DEFAULT 'ACTIVE',
  level             VARCHAR(20)  NOT NULL DEFAULT 'INFO',
  
  source_id         INT   NOT NULL REFERENCES sources (id),
  channel_id        INT   NOT NULL REFERENCES channels (id)
);

-- A definition may deliver to multiple destination groups on its channel.
-- template_id is the template rendered for that target. A WEBHOOK target sends
-- it (a format = 'JSON' template) as its request body; when NULL the target
-- sends the notify request's variables as the body.
-- EMAIL / MSTEAMS render one message from the first target that has a template.
CREATE TABLE definition_targets (
  id                  INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  definition_id       INT NOT NULL REFERENCES definitions (id) ON DELETE CASCADE,
  target_id           INT NOT NULL REFERENCES targets (id) ON DELETE RESTRICT,
  template_id         INT REFERENCES templates (id),
  created_at          timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX idx_definition_targets_definition_id ON definition_targets (definition_id);
