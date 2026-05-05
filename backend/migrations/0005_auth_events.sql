CREATE TABLE auth_events (
    id          UUID        PRIMARY KEY,
    user_id     UUID        REFERENCES users(id) ON DELETE SET NULL,
    event_type  TEXT        NOT NULL,
    ip          INET,
    user_agent  TEXT,
    detail      JSONB,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX auth_events_user_id_created_idx    ON auth_events(user_id, created_at DESC);
CREATE INDEX auth_events_event_type_created_idx ON auth_events(event_type, created_at DESC);
