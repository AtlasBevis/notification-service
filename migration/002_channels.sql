-- channel catalog; dispatch config lives in metadata JSON
-- MSTEAMS: { "url": "https://..." }
-- EMAIL:   { "host": "smtp.office365.com", "username": "...", "password": "..." }

CREATE TABLE channels (
  code              VARCHAR(20)  PRIMARY KEY,
  created_at        timestamptz  NOT NULL DEFAULT now(),
  updated_at        timestamptz  NOT NULL,

  name              VARCHAR(255) NOT NULL,
  status            VARCHAR(20)  NOT NULL DEFAULT 'ACTIVE',
  metadata          JSONB        NOT NULL DEFAULT '{}'::jsonb
);

INSERT INTO channels (code, name, metadata, updated_at) VALUES
  (
    'MSTEAMS',
    'Microsoft Teams',
    '{"url": ""}'::jsonb,
    now()
  ),
  (
    'EMAIL',
    'Email',
    '{"host": "smtp.office365.com", "username": "", "password": ""}'::jsonb,
    now()
  );
