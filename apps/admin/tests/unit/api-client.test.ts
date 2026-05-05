import { afterEach, beforeEach, describe, expect, it, type MockInstance, vi } from "vitest";
import { createApi, readCookie } from "@/api/client";

const ADMIN_API = "http://localhost/admin/api";

function jsonResponse(body: unknown, init: ResponseInit = {}): Response {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { "Content-Type": "application/json" },
    ...init,
  });
}

describe("csrfMiddleware", () => {
  // openapi-fetch captures globalThis.fetch at createClient time, so the spy
  // must be installed before constructing the api inside beforeEach.
  let api: ReturnType<typeof createApi>;
  let fetchSpy: MockInstance<typeof fetch>;

  beforeEach(() => {
    fetchSpy = vi.spyOn(globalThis, "fetch").mockResolvedValue(jsonResponse({ ok: true }));
    api = createApi(ADMIN_API);
  });
  afterEach(() => {
    fetchSpy.mockRestore();
  });

  function lastRequest(): Request {
    const call = fetchSpy.mock.calls.at(-1);
    if (!call) throw new Error("fetch was not called");
    return call[0] as Request;
  }

  it("does not set X-CSRF-Token on GET", async () => {
    document.cookie = "admin_csrf=tok-abc; path=/";
    await api.GET("/me");
    expect(lastRequest().headers.get("X-CSRF-Token")).toBeNull();
  });

  it("sets X-CSRF-Token on POST when admin_csrf cookie is present", async () => {
    document.cookie = "admin_csrf=tok-xyz; path=/";
    await api.POST("/logout");
    expect(lastRequest().headers.get("X-CSRF-Token")).toBe("tok-xyz");
  });

  it("omits X-CSRF-Token on POST when no cookie is set", async () => {
    await api.POST("/logout");
    expect(lastRequest().headers.get("X-CSRF-Token")).toBeNull();
  });

  it("sets X-CSRF-Token on DELETE", async () => {
    document.cookie = "admin_csrf=tok-del; path=/";
    await api.DELETE("/users/{id}/sessions", {
      params: { path: { id: "00000000-0000-0000-0000-000000000000" } },
    });
    expect(lastRequest().headers.get("X-CSRF-Token")).toBe("tok-del");
  });
});

describe("readCookie", () => {
  it("url-decodes the cookie value", () => {
    document.cookie = `admin_csrf=${encodeURIComponent("a/b+c==")}; path=/`;
    expect(readCookie("admin_csrf")).toBe("a/b+c==");
  });

  it("returns null when the cookie is absent", () => {
    expect(readCookie("admin_csrf")).toBeNull();
  });
});

describe("authMiddleware", () => {
  let api: ReturnType<typeof createApi>;
  let fetchSpy: MockInstance<typeof fetch>;
  const originalLocation = window.location;
  const assignSpy = vi.fn();

  function stubLocation(pathname: string) {
    Object.defineProperty(window, "location", {
      configurable: true,
      value: { ...originalLocation, pathname, assign: assignSpy },
    });
  }

  beforeEach(() => {
    fetchSpy = vi.spyOn(globalThis, "fetch").mockResolvedValue(jsonResponse({ ok: true }));
    api = createApi(ADMIN_API);
  });
  afterEach(() => {
    Object.defineProperty(window, "location", {
      configurable: true,
      value: originalLocation,
    });
    assignSpy.mockReset();
    fetchSpy.mockRestore();
  });

  it("does not redirect on a 200", async () => {
    stubLocation("/admin/users");
    await api.GET("/me");
    expect(assignSpy).not.toHaveBeenCalled();
  });

  it("redirects to /admin/login on 401", async () => {
    fetchSpy.mockResolvedValueOnce(
      jsonResponse({ code: "unauthorized", message: "no session" }, { status: 401 })
    );
    stubLocation("/admin/users");
    await api.GET("/me");
    expect(assignSpy).toHaveBeenCalledTimes(1);
    expect(assignSpy).toHaveBeenCalledWith("/admin/login");
  });

  it("does not redirect when already on /admin/login", async () => {
    fetchSpy.mockResolvedValueOnce(
      jsonResponse({ code: "unauthorized", message: "bad password" }, { status: 401 })
    );
    stubLocation("/admin/login");
    await api.POST("/login", { body: { email: "a@b.c", password: "x" } });
    expect(assignSpy).not.toHaveBeenCalled();
  });
});
