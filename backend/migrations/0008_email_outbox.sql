CREATE TABLE email_outbox (
    id              UUID        PRIMARY KEY,
    recipient       CITEXT      NOT NULL,
    subject         TEXT        NOT NULL,
    html_body       TEXT        NOT NULL,
    text_body       TEXT        NOT NULL,
    status          TEXT        NOT NULL DEFAULT 'pending',
    attempts        INTEGER     NOT NULL DEFAULT 0,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_error      TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    sent_at         TIMESTAMPTZ,
    locked_at       TIMESTAMPTZ
);

ALTER TABLE email_outbox
ADD CONSTRAINT email_outbox_status_check
CHECK (status IN ('pending', 'sending', 'sent', 'failed'));

CREATE INDEX email_outbox_due_idx
ON email_outbox(status, next_attempt_at)
WHERE status IN ('pending', 'failed');

CREATE INDEX email_outbox_recipient_idx ON email_outbox(recipient);
