-- WebAuthn / passkey credentials registered against admin users. Used as the
-- second factor (or as the only factor, once the operator decides) on the
-- admin login flow. Regular /v1 users are not in scope.
--
-- The full webauthn-rs `Passkey` struct is stored as JSONB so the sign-counter
-- and any other fields can be round-tripped without flattening every internal
-- attribute into its own column. `credential_id` is duplicated as a top-level
-- column so we can index it for the credential-id → row lookup that webauthn
-- authentication-finish needs.
CREATE TABLE webauthn_credentials (
    id              UUID PRIMARY KEY,
    admin_user_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    credential_id   BYTEA NOT NULL UNIQUE,
    passkey         JSONB NOT NULL,
    label           TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at    TIMESTAMPTZ
);

CREATE INDEX webauthn_credentials_admin_user_idx
    ON webauthn_credentials(admin_user_id);
