ALTER TABLE email_outbox
ADD COLUMN email_kind TEXT NOT NULL DEFAULT 'notification';

ALTER TABLE email_outbox
ADD CONSTRAINT email_outbox_kind_check
CHECK (email_kind IN ('notification', 'email_verification', 'password_reset'));

CREATE INDEX email_outbox_auth_retention_idx
ON email_outbox(email_kind, status, updated_at)
WHERE email_kind IN ('email_verification', 'password_reset');
