import createClient, { type Middleware } from "openapi-fetch";
import type { paths } from "./schema";

// Backend cookie names — see lab-rust-server admin auth and plan §1 item 1.
// `admin_session` is HttpOnly (never read here); `admin_csrf` is readable by JS
// for the double-submit pattern.
const CSRF_COOKIE = "admin_csrf";
const CSRF_HEADER = "X-CSRF-Token";
const LOGIN_PATH = "/admin/login";
const STATE_CHANGING_METHODS = new Set(["POST", "PATCH", "PUT", "DELETE"]);
const ADMIN_API_BASE_URL = "/admin/api";

export const csrfMiddleware: Middleware = {
  async onRequest({ request }) {
    if (STATE_CHANGING_METHODS.has(request.method.toUpperCase())) {
      const token = readCookie(CSRF_COOKIE);
      if (token) request.headers.set(CSRF_HEADER, token);
    }
    return request;
  },
};

export const authMiddleware: Middleware = {
  async onResponse({ response }) {
    if (
      response.status === 401 &&
      typeof window !== "undefined" &&
      window.location.pathname !== LOGIN_PATH
    ) {
      window.location.assign(LOGIN_PATH);
    }
    return response;
  },
};

// Factory so tests can inject an absolute baseUrl (jsdom + undici fetch don't
// resolve relative URLs the way real browsers do). Production code uses the
// default singleton `api`.
export function createApi(baseUrl: string = ADMIN_API_BASE_URL) {
  const client = createClient<paths>({ baseUrl });
  client.use(csrfMiddleware, authMiddleware);
  return client;
}

export const api = createApi();

export function readCookie(name: string): string | null {
  if (typeof document === "undefined") return null;
  const prefix = `${name}=`;
  for (const row of document.cookie.split("; ")) {
    if (row.startsWith(prefix)) return decodeURIComponent(row.slice(prefix.length));
  }
  return null;
}
