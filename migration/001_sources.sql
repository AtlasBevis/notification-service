CREATE TABLE sources (
  code              VARCHAR(50)  PRIMARY KEY,
  created_at        timestamptz  NOT NULL DEFAULT now(),
  updated_at        timestamptz  NOT NULL,

  name              VARCHAR(255) NOT NULL,
  status            VARCHAR(20)  NOT NULL DEFAULT 'ACTIVE',
  metadata          JSONB        NOT NULL DEFAULT '{}'::jsonb
);

INSERT INTO sources (code, name) VALUES
  ('PIPELINE_ETL', 'ETL batch'),
  ('GITLAB-RUNNER', 'Pipeline job'),
  ('AIRFLOW', 'Airflow'),
  ('SPARK', 'Compute Job'),
;