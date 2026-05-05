ALTER TABLE users
ADD COLUMN role TEXT NOT NULL DEFAULT 'user';

ALTER TABLE users
ADD CONSTRAINT users_role_check CHECK (role IN ('user', 'admin'));

CREATE TABLE admin_sessions (
    id              UUID        PRIMARY KEY,
    admin_user_id   UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash      BYTEA       NOT NULL UNIQUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at      TIMESTAMPTZ NOT NULL,
    revoked_at      TIMESTAMPTZ,
    ip              INET,
    user_agent      TEXT
);

CREATE INDEX admin_sessions_admin_user_id_idx ON admin_sessions(admin_user_id);
CREATE INDEX admin_sessions_expires_at_idx ON admin_sessions(expires_at);
