// WebAuthn helpers for the admin console.
//
// Routes covered:
//   POST /admin/api/login                    (handled in login-page.tsx)
//   POST /admin/api/webauthn/login/finish
//   POST /admin/api/webauthn/register/begin  (CSRF required)
//   POST /admin/api/webauthn/register/finish (CSRF required)
//   GET  /admin/api/webauthn/credentials
//   DELETE /admin/api/webauthn/credentials/:id (CSRF required)
//
// We bypass openapi-fetch for these calls because the OpenAPI snapshot is
// regenerated only when a developer runs `bun run openapi` against a live
// backend; this file lets the SPA work even with a stale snapshot. Once the
// snapshot is refreshed, callers can be migrated to `api.POST(...)` later.
import { startAuthentication, startRegistration } from "@simplewebauthn/browser";
import { ApiError, type ErrorBody } from "@/lib/errors";
import { readCookie } from "@/api/client";

const BASE = "/admin/api";
const CSRF_COOKIE = "admin_csrf";
const CSRF_HEADER = "X-CSRF-Token";

interface AuthenticatedLogin {
  kind: "authenticated";
  user_id: string;
  email: string;
}

interface WebauthnRequiredLogin {
  kind: "webauthn_required";
  pending_token: string;
  // Server-side webauthn-rs returns { publicKey: {...} }; @simplewebauthn/browser
  // wants the inner object.
  challenge: { publicKey: PublicKeyCredentialRequestOptionsJSON };
}

export type LoginOutcome = AuthenticatedLogin | WebauthnRequiredLogin;

export interface CredentialSummary {
  id: string;
  label: string;
  created_at: string;
  last_used_at: string | null;
}

// Minimal subset of @simplewebauthn/browser's option types so we don't have to
// depend on every internal field.
type PublicKeyCredentialRequestOptionsJSON = Parameters<
  typeof startAuthentication
>[0]["optionsJSON"];
type PublicKeyCredentialCreationOptionsJSON = Parameters<
  typeof startRegistration
>[0]["optionsJSON"];

interface RegisterBeginResponse {
  challenge: { publicKey: PublicKeyCredentialCreationOptionsJSON };
}

async function jsonRequest<T>(
  path: string,
  init: RequestInit & { csrf?: boolean } = {}
): Promise<T> {
  const headers = new Headers(init.headers);
  headers.set("content-type", "application/json");
  if (init.csrf) {
    const token = readCookie(CSRF_COOKIE);
    if (token) headers.set(CSRF_HEADER, token);
  }
  const response = await fetch(`${BASE}${path}`, {
    ...init,
    credentials: "include",
    headers,
  });
  if (!response.ok) {
    let errorBody: ErrorBody = {
      code: "request_failed",
      message: response.statusText || "Request failed",
    };
    try {
      const parsed = (await response.json()) as unknown;
      if (
        parsed !== null &&
        typeof parsed === "object" &&
        "code" in parsed &&
        "message" in parsed
      ) {
        errorBody = parsed as ErrorBody;
      }
    } catch {
      // Server returned a non-JSON error body; keep the default.
    }
    throw new ApiError(errorBody, response);
  }
  if (response.status === 204) return undefined as T;
  return (await response.json()) as T;
}

export async function loginWithPassword(email: string, password: string): Promise<LoginOutcome> {
  return jsonRequest<LoginOutcome>("/login", {
    method: "POST",
    body: JSON.stringify({ email, password }),
  });
}

/**
 * Run `navigator.credentials.get` against the backend's challenge and post the
 * assertion to /webauthn/login/finish. The server sets session cookies on
 * success.
 */
export async function completeWebauthnLogin(
  pending_token: string,
  challenge: WebauthnRequiredLogin["challenge"]
): Promise<{ user_id: string; email: string }> {
  const assertion = await startAuthentication({ optionsJSON: challenge.publicKey });
  return jsonRequest<{ user_id: string; email: string }>("/webauthn/login/finish", {
    method: "POST",
    body: JSON.stringify({ pending_token, assertion }),
  });
}

export async function listCredentials(): Promise<{ items: CredentialSummary[] }> {
  return jsonRequest("/webauthn/credentials", { method: "GET" });
}

export async function deleteCredential(id: string): Promise<void> {
  await jsonRequest(`/webauthn/credentials/${id}`, {
    method: "DELETE",
    csrf: true,
  });
}

/**
 * Full registration ceremony: ask the server for a challenge, perform
 * `navigator.credentials.create`, and post the attestation back to be stored.
 */
export async function registerNewCredential(label: string): Promise<CredentialSummary> {
  const begin = await jsonRequest<RegisterBeginResponse>("/webauthn/register/begin", {
    method: "POST",
    csrf: true,
  });
  const credential = await startRegistration({ optionsJSON: begin.challenge.publicKey });
  return jsonRequest<CredentialSummary>("/webauthn/register/finish", {
    method: "POST",
    csrf: true,
    body: JSON.stringify({ label, credential }),
  });
}
