-- One accepted request. Status is derived from deliveries, not stored here.
-- public_id will generated on app. purpose to trace notifications on portal
-- trace_id is the caller trace. A replay of (source_code, trace_id) is rejected.

CREATE TABLE notifications (
  id                BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  public_id         VARCHAR(50)  NOT NULL UNIQUE,
  created_at        timestamptz  NOT NULL DEFAULT now(),

  code              VARCHAR(50)  NOT NULL,
  trace_id          VARCHAR(100) NOT NULL,
  definition_id     INT,
  source            VARCHAR(50),
  variables         JSONB        NOT NULL DEFAULT '{}'::jsonb,
  CONSTRAINT uq_noti_source UNIQUE (source, trace_id)
);
