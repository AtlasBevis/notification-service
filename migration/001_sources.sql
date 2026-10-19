-- Source emits notifications

CREATE TABLE sources (
  id                INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  code              VARCHAR(50)  NOT NULL UNIQUE,
  created_at        timestamptz  NOT NULL DEFAULT now(),
  updated_at        timestamptz,

  name              VARCHAR(255) NOT NULL,
  status            VARCHAR(20)  NOT NULL DEFAULT 'ACTIVE'
);

-- data
INSERT INTO sources (code, name) VALUES
  ('SPARK_AIRFLOW',   'Spark Airflow'),
  ('GITLAB_RUNNER',  'GitLab Runner')
;
