-- History of each channel call for a delivery. One row per try.

CREATE TABLE delivery_attempts (
  id                BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  delivery_id       BIGINT       NOT NULL REFERENCES deliveries (id),
  attempt_no        INT          NOT NULL,

  status            VARCHAR(20)  NOT NULL,
  started_at        timestamptz  NOT NULL DEFAULT now(),
  finished_at       timestamptz,
  error_message     TEXT,

  CONSTRAINT uq_delivery_attempts UNIQUE (delivery_id, attempt_no),
  CONSTRAINT chk_delivery_attempts_status
    CHECK (status IN ('PROCESSING', 'SUCCESS', 'FAILED', 'TIMEOUT'))
);

CREATE INDEX idx_delivery_attempts_delivery
  ON delivery_attempts (delivery_id, attempt_no DESC);
