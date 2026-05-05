CREATE TABLE refresh_tokens (
    id              UUID        PRIMARY KEY,
    user_id         UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    family_id       UUID        NOT NULL,
    token_hash      BYTEA       NOT NULL,
    parent_id       UUID        REFERENCES refresh_tokens(id),
    issued_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at      TIMESTAMPTZ NOT NULL,
    used_at         TIMESTAMPTZ,
    revoked_at      TIMESTAMPTZ,
    revoked_reason  TEXT,
    user_agent      TEXT,
    ip              INET
);

CREATE INDEX refresh_tokens_user_id_idx    ON refresh_tokens(user_id);
CREATE INDEX refresh_tokens_family_id_idx  ON refresh_tokens(family_id);
CREATE UNIQUE INDEX refresh_tokens_token_hash_idx ON refresh_tokens(token_hash);
CREATE INDEX refresh_tokens_expires_at_idx ON refresh_tokens(expires_at);
