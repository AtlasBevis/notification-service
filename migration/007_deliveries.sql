-- One rendered send for a runtime notification. The destination is copied from definitions.
-- Retry updates this row and appends delivery_attempts. It does not insert another delivery.
-- status: PENDING → PROCESSING → SENT | RETRY_WAIT | FAILED

CREATE TABLE deliveries (
  id                BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  created_at        timestamptz  NOT NULL DEFAULT now(),
  updated_at        timestamptz,

  notification_id   BIGINT       NOT NULL REFERENCES notifications (id),
  target_code       VARCHAR(50)  NOT NULL,
  channel_code      VARCHAR(50)  NOT NULL,

  status            VARCHAR(20)  NOT NULL DEFAULT 'PENDING',
  title             VARCHAR(255) NOT NULL,
  body              TEXT         NOT NULL,
  payload           JSONB        NOT NULL DEFAULT '{}'::jsonb,
  destination       JSONB        NOT NULL DEFAULT '{}'::jsonb,

  attempt_count     INT          NOT NULL DEFAULT 0,
  next_retry_at     timestamptz,
  last_error_message TEXT,
  sent_at           timestamptz,

  CONSTRAINT uq_deliveries_notification UNIQUE (notification_id)
);

CREATE INDEX idx_deliveries_pending ON deliveries (next_retry_at, id)
  WHERE status IN ('PENDING', 'RETRY_WAIT');
CREATE INDEX idx_deliveries_notification_id ON deliveries (notification_id);
