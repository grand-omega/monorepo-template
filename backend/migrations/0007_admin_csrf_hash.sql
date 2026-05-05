-- Double-submit cookie CSRF: the SHA-256 of the cookie value is stored on the
-- session row. The wire token is set as a non-HttpOnly cookie so the SPA can
-- echo it back in the X-CSRF-Token header on state-changing requests.
ALTER TABLE admin_sessions
ADD COLUMN csrf_hash BYTEA NOT NULL DEFAULT '\x00';

ALTER TABLE admin_sessions
ALTER COLUMN csrf_hash DROP DEFAULT;
