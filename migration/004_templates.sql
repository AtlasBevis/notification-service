-- templates is the stable identity a definition target points at (definition_targets.template_id).
-- template_versions holds the renderable content; a send uses the latest ACTIVE version.
-- format: TEXT / HTML for EMAIL, ADAPTIVE_CARD for MSTEAMS, JSON for WEBHOOK
-- (content is the request body object; {{variable}} placeholders are filled from
-- the notify request's variables).
--
-- version used when sending:
-- SELECT v.id, v.version, v.title, v.content, v.format
-- FROM template_versions v
-- WHERE v.template_id = $1 AND v.status = 'ACTIVE'
-- ORDER BY v.version DESC
-- LIMIT 1;

CREATE TABLE templates (
  id                INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  created_at        timestamptz NOT NULL DEFAULT now(),
  updated_at        timestamptz,

  name              VARCHAR(255) NOT NULL UNIQUE,
  status            VARCHAR(20)  NOT NULL DEFAULT 'ACTIVE'
);

-- Insert-only: a content change is a new row with version = max(version) + 1.
-- Set status = 'INACTIVE' on a version to roll back to the previous ACTIVE one.
CREATE TABLE template_versions (
  id                INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  created_at        timestamptz NOT NULL DEFAULT now(),

  template_id       INT          NOT NULL REFERENCES templates (id) ON DELETE CASCADE,
  version           INTEGER      NOT NULL DEFAULT 1,
  status            VARCHAR(20)  NOT NULL DEFAULT 'ACTIVE',

  title             TEXT         NOT NULL, -- email subject / card title template
  content           TEXT         NOT NULL, -- body / Adaptive Card JSON / webhook body template
  format            VARCHAR(30)  NOT NULL DEFAULT 'TEXT', -- TEXT / HTML / ADAPTIVE_CARD / JSON

  CONSTRAINT uq_template_versions_template_version UNIQUE (template_id, version)
);

CREATE INDEX idx_template_versions_active ON template_versions (template_id, version DESC)
  WHERE status = 'ACTIVE';

-- data
INSERT INTO templates (name) VALUES
  ('airflow'),
  ('default')
;

INSERT INTO template_versions (template_id, version, title, content, format)
SELECT id, 1, 'Airflow Task Failed',
  $card${
  "type": "AdaptiveCard",
  "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
  "version": "1.5",
  "msteams": { "width": "Full" },
  "body": [
    {
      "type": "TextBlock",
      "text": "🚨 Airflow Task Failed",
      "weight": "Bolder",
      "size": "Large",
      "color": "Attention"
    },
    {
      "type": "TextBlock",
      "text": "Task cần xử lý trên môi trường PRODUCTION",
      "isSubtle": true,
      "wrap": true,
      "spacing": "None"
    },
    {
      "type": "Container",
      "style": "attention",
      "spacing": "Medium",
      "items": [
        {
          "type": "TextBlock",
          "text": "Team: {{team}}",
          "weight": "Bolder",
          "wrap": true,
          "spacing": "Small"
        },
        {
          "type": "TextBlock",
          "text": "{{status}} - retry: {{try_number}}",
          "weight": "Bolder",
          "wrap": true
        }
      ]
    },
    {
      "type": "FactSet",
      "spacing": "Medium",
      "facts": [
        { "title": "DAG", "value": "{{dag_id}}" },
        { "title": "Task", "value": "{{task_id}}" },
        { "title": "Run ID", "value": "{{run_id}}" },
        { "title": "Start", "value": "{{start_date}}" },
        { "title": "Logical", "value": "{{logical_date}}" }
      ]
    },
    {
      "type": "TextBlock",
      "text": "MIS · Data Platform",
      "isSubtle": true,
      "size": "Small",
      "spacing": "Medium"
    }
  ],
  "actions": [
    {
      "type": "Action.OpenUrl",
      "title": "View Task Log",
      "url": "{{log_url}}"
    }
  ]
}$card$,
  'ADAPTIVE_CARD'
FROM templates WHERE name = 'airflow';

INSERT INTO template_versions (template_id, version, title, content, format)
SELECT id, 1, '[{{kind}}] {{source}}', '{{message}}', 'TEXT'
FROM templates WHERE name = 'default';
