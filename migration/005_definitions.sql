-- One code, one destination. Callers trigger `code`; they do not choose the channel.
-- level: INFO, WARN, ERROR, CRITICAL

CREATE TABLE definitions (
  id                BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  created_at        timestamptz  NOT NULL DEFAULT now(),
  updated_at        timestamptz  NOT NULL,

  code              VARCHAR(50)  NOT NULL UNIQUE,
  name              VARCHAR(255) NOT NULL,
  description       VARCHAR(255),
  status            VARCHAR(20)  NOT NULL DEFAULT 'ACTIVE'
  level             VARCHAR(20)  NOT NULL DEFAULT 'INFO',
  
  source_id         INT   NOT NULL REFERENCES sources (id),
  channel_id        INT   NOT NULL REFERENCES channels (id),
  target_id         INT   NOT NULL REFERENCES targets (id),
  template_id       INT   NOT NULL REFERENCES templates (id),
);

-- INSERT INTO definitions (
--   code, name, description, source_code, channel_code, target_code, template_id, level, updated_at
-- )
-- SELECT
--   'AIRFLOW_TASK_FAILED',
--   'Airflow task failed',
--   'Task failure on Airflow',
--   'AIRFLOW',
--   'MSTEAMS',
--   'TEAM_MIS',
--   id,
--   'ERROR',
--   now()
-- FROM templates
-- WHERE name = 'airflow' AND version = 1 AND status = 'ACTIVE';

-- INSERT INTO definitions (
--   code, name, description, source_code, channel_code, target_code, template_id, level, updated_at
-- )
-- SELECT
--   v.code,
--   v.name,
--   v.description,
--   v.source_code,
--   v.channel_code,
--   v.target_code,
--   t.id,
--   v.level,
--   now()
-- FROM (
--   VALUES
--     ('ETL_JOB_FAILED', 'ETL job failed', 'Batch pipeline failure', 'ETL', 'MSTEAMS', 'TEAM_MIS', 'ERROR'),
--     ('ETL_JOB_SUCCEEDED', 'ETL job succeeded', 'Batch pipeline success', 'ETL', 'MSTEAMS', 'TEAM_MIS', 'INFO'),
--     ('GITLAB_MR_OPENED', 'GitLab merge request opened', 'Merge request opened', 'GITLAB', 'MSTEAMS', 'TEAM_MIS', 'INFO'),
--     ('REVENUE_DAILY', 'Daily revenue', 'Daily revenue and metrics digest', 'REVENUE', 'EMAIL', 'OPS_EMAIL', 'INFO')
-- ) AS v(code, name, description, source_code, channel_code, target_code, level)
-- JOIN templates t ON t.name = 'default' AND t.version = 1 AND t.status = 'ACTIVE';
