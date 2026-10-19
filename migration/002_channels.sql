-- How a message is transported. WEBHOOK destinations live in webhook_configs;
-- SMTP settings live in config (`smtp`).

CREATE TABLE channels (
  id                INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  code              VARCHAR(50)  NOT NULL UNIQUE,
  created_at        timestamptz  NOT NULL DEFAULT now(),
  updated_at        timestamptz,

  name              VARCHAR(255) NOT NULL,
  status            VARCHAR(20)  NOT NULL DEFAULT 'ACTIVE'
);

-- data
INSERT INTO channels (code, name) VALUES
  ('EMAIL',    'Email'          ),
  ('MSTEAMS',  'Microsoft Teams'),
  ('WEBHOOK',  'HTTP Webhook')
;
