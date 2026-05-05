CREATE TABLE password_reset_tokens (
    id            UUID        PRIMARY KEY,
    user_id       UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash    BYTEA       NOT NULL,
    expires_at    TIMESTAMPTZ NOT NULL,
    consumed_at   TIMESTAMPTZ,
    requested_ip  INET,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX password_reset_token_hash_idx ON password_reset_tokens(token_hash);
CREATE INDEX        password_reset_user_id_idx    ON password_reset_tokens(user_id);
CREATE INDEX        password_reset_expires_at_idx ON password_reset_tokens(expires_at);
