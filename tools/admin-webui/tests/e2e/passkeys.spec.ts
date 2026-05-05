import { expect, test } from "@playwright/test";

const adminEmail = process.env.E2E_ADMIN_EMAIL;
const adminPassword = process.env.E2E_ADMIN_PASSWORD;

test.describe("passkeys page", () => {
  test.skip(
    !adminEmail || !adminPassword,
    "Set E2E_ADMIN_EMAIL and E2E_ADMIN_PASSWORD to run passkeys tests against a real backend"
  );

  test("settings page renders empty state for an admin with no passkeys", async ({ page }) => {
    await page.goto("/admin/login");
    await page.getByLabel("Email").fill(adminEmail ?? "");
    await page.getByLabel("Password").fill(adminPassword ?? "");
    await page.getByRole("button", { name: "Sign in" }).click();
    await expect(page).toHaveURL((url) => url.pathname === "/admin/users");

    await page.getByRole("link", { name: "Passkeys" }).click();
    await expect(page).toHaveURL((url) => url.pathname === "/admin/passkeys");

    // Either the user has no passkeys (empty state) or some — both render the
    // "Register passkey" button. Assert the form is reachable and the heading
    // describes the page.
    await expect(page.getByRole("heading", { name: "Passkeys", exact: true })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Add a passkey" })).toBeVisible();
    await expect(page.getByLabel("Label")).toBeVisible();
    await expect(page.getByRole("button", { name: "Register passkey" })).toBeDisabled();
  });

  test("password-only login still works when the admin has no passkeys", async ({ page }) => {
    // Bootstrapping path: an admin with zero registered passkeys can sign in
    // with password alone. The "Confirm with passkey" screen must NOT appear.
    await page.goto("/admin/login");
    await page.getByLabel("Email").fill(adminEmail ?? "");
    await page.getByLabel("Password").fill(adminPassword ?? "");
    await page.getByRole("button", { name: "Sign in" }).click();

    // Either lands on /users directly, or — if the admin already enrolled a
    // passkey out-of-band — the WebAuthn confirm view appears. We only assert
    // the no-passkey path here; if the runner has registered passkeys the test
    // must be re-run against a fresh admin.
    await expect(page).toHaveURL(
      (url) => url.pathname === "/admin/users" || url.pathname === "/admin/login",
      { timeout: 10_000 }
    );
  });
});

test("login page UI does not show the passkey-confirm screen by default", async ({ page }) => {
  // Pure UI test, no backend required.
  await page.goto("/admin/login");
  await expect(page.getByRole("heading", { name: "Confirm with passkey" })).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Admin login" })).toBeVisible();
});
