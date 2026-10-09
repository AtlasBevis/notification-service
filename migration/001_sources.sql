-- Who may emit a notification. Caller's service must match notifications.source_code.

CREATE TABLE sources (
  code              VARCHAR(50)  PRIMARY KEY,
  created_at        timestamptz  NOT NULL DEFAULT now(),
  updated_at        timestamptz  NOT NULL,

  name              VARCHAR(255) NOT NULL,
  status            VARCHAR(20)  NOT NULL DEFAULT 'ACTIVE',
  metadata          JSONB        NOT NULL DEFAULT '{}'::jsonb,

  CONSTRAINT chk_sources_status CHECK (status IN ('ACTIVE', 'INACTIVE'))
);

INSERT INTO sources (code, name, updated_at) VALUES
  ('AIRFLOW', 'Airflow', now()),
  ('ETL',     'ETL batch', now()),
  ('GITLAB',  'GitLab', now()),
  ('REVENUE', 'Revenue and metrics', now()),
  ('SPARK',   'Compute job', now());
