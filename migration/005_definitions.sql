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
  target_id         INT   REFERENCES targets (id),
  template_id       INT   REFERENCES templates (id)
);
