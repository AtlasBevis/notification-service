-- destination catalog (portal); EMAIL: { "to": [], "cc": [], "bcc": [] }; MSTEAMS: { "url": "..." }

CREATE TABLE targets (
  code              VARCHAR(50)  PRIMARY KEY,
  created_at        timestamptz  NOT NULL DEFAULT now(),
  updated_at        timestamptz  NOT NULL,

  name              VARCHAR(255) NOT NULL,
  status            VARCHAR(20)  NOT NULL DEFAULT 'ACTIVE',
  metadata          JSONB        NOT NULL DEFAULT '{}'::jsonb
);

INSERT INTO targets (code, name, metadata, updated_at) VALUES
  (
    'TEAM_MIS',
    'MIS Teams webhook',
    '{"url": ""}'::jsonb,
    now()
  ),
  (
    'OPS_EMAIL',
    'Ops email group',
    '{"to": [], "cc": [], "bcc": []}'::jsonb,
    now()
  );
