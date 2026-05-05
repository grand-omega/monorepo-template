import { expect, test } from "@playwright/test";

const adminEmail = process.env.E2E_ADMIN_EMAIL;
const adminPassword = process.env.E2E_ADMIN_PASSWORD;
const targetEmail = process.env.E2E_TARGET_EMAIL;

test.describe("user actions", () => {
  test.skip(
    !adminEmail || !adminPassword || !targetEmail,
    "Set E2E_ADMIN_EMAIL, E2E_ADMIN_PASSWORD, and E2E_TARGET_EMAIL to run backend action tests"
  );

  test("locks, unlocks, and revokes sessions with target confirmation", async ({ page }) => {
    await page.goto("/admin/login");
    await page.getByLabel("Email").fill(adminEmail ?? "");
    await page.getByLabel("Password").fill(adminPassword ?? "");
    await page.getByRole("button", { name: "Sign in" }).click();

    await page.getByLabel("Search users").fill(targetEmail ?? "");
    await page.getByRole("button", { name: "Search" }).click();
    await expect(page).toHaveURL((url) => url.searchParams.get("q") === targetEmail);
    await page.getByRole("link", { name: targetEmail ?? "" }).click();

    await expect(page).toHaveURL((url) => url.pathname.startsWith("/admin/users/"));
    await expect(page.getByRole("heading", { name: targetEmail ?? "" })).toBeVisible();

    if (await page.getByRole("button", { exact: true, name: "Unlock user" }).isVisible()) {
      await page.getByRole("button", { exact: true, name: "Unlock user" }).click();
      await expect(page.getByRole("dialog")).toContainText(targetEmail ?? "");
      await page.getByRole("button", { exact: true, name: "Unlock user" }).last().click();
      await expect(page.getByRole("button", { exact: true, name: "Lock user" })).toBeVisible();
    }

    await page.getByRole("button", { exact: true, name: "Lock user" }).click();
    await expect(page.getByRole("dialog")).toContainText(targetEmail ?? "");
    await page.getByLabel("Reason").fill("Playwright lock action test");
    await page.getByRole("button", { exact: true, name: "Lock user" }).last().click();
    await expect(page.getByRole("button", { exact: true, name: "Unlock user" })).toBeVisible();

    await page.getByRole("button", { exact: true, name: "Unlock user" }).click();
    await expect(page.getByRole("dialog")).toContainText(targetEmail ?? "");
    await page.getByRole("button", { exact: true, name: "Unlock user" }).last().click();
    await expect(page.getByRole("button", { exact: true, name: "Lock user" })).toBeVisible();

    await page.getByRole("button", { name: "Revoke sessions" }).click();
    await expect(page.getByRole("dialog")).toContainText(targetEmail ?? "");
    await page.getByRole("button", { name: "Revoke sessions" }).last().click();
    await expect(page.getByRole("dialog")).toBeHidden();
  });
});
