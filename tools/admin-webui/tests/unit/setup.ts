import "@testing-library/jest-dom/vitest";
import { afterEach } from "vitest";

// Component-level tests will reach for MSW (plan §10); middleware tests stub
// globalThis.fetch directly. Either way, document.cookie leaks across tests
// in jsdom, so wipe it.
afterEach(() => {
  for (const c of document.cookie.split(";")) {
    const eq = c.indexOf("=");
    const name = (eq > -1 ? c.slice(0, eq) : c).trim();
    if (name) document.cookie = `${name}=; expires=Thu, 01 Jan 1970 00:00:00 GMT; path=/`;
  }
});
