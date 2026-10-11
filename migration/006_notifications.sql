-- One accepted request. Status is derived from deliveries, not stored here.
-- idempotency_key is the caller trace. A replay of (source_code, idempotency_key) is rejected.

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE notifications (
  id                BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  public_id         UUID         NOT NULL DEFAULT gen_random_uuid() UNIQUE,
  created_at        timestamptz  NOT NULL DEFAULT now(),

  definition_id     BIGINT       NOT NULL REFERENCES definitions (id),
  source_code       VARCHAR(50)  NOT NULL REFERENCES sources (code),
  code              VARCHAR(50)  NOT NULL,
  idempotency_key   VARCHAR(255) NOT NULL,
  variables         JSONB        NOT NULL DEFAULT '{}'::jsonb,

  CONSTRAINT uq_notifications_idempotency UNIQUE (source_code, idempotency_key)
);

CREATE INDEX idx_notifications_created_at ON notifications (created_at DESC);
CREATE INDEX idx_notifications_definition ON notifications (definition_id);
