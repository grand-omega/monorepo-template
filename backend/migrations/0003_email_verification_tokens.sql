CREATE TABLE email_verification_tokens (
    id           UUID        PRIMARY KEY,
    user_id      UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash   BYTEA       NOT NULL,
    email        CITEXT      NOT NULL,
    expires_at   TIMESTAMPTZ NOT NULL,
    consumed_at  TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX email_verification_token_hash_idx ON email_verification_tokens(token_hash);
CREATE INDEX        email_verification_user_id_idx    ON email_verification_tokens(user_id);
CREATE INDEX        email_verification_expires_at_idx ON email_verification_tokens(expires_at);
