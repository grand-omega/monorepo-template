import { expect, test } from "@playwright/test";

const adminEmail = process.env.E2E_ADMIN_EMAIL;
const adminPassword = process.env.E2E_ADMIN_PASSWORD;

test.describe("polish", () => {
  test.skip(
    !adminEmail || !adminPassword,
    "Set E2E_ADMIN_EMAIL and E2E_ADMIN_PASSWORD to run backend polish tests"
  );

  test("supports keyboard navigation and search focus", async ({ page }) => {
    await page.goto("/admin/login");
    await page.getByLabel("Email").fill(adminEmail ?? "");
    await page.getByLabel("Password").fill(adminPassword ?? "");
    await page.getByRole("button", { name: "Sign in" }).click();
    await expect(page).toHaveURL((url) => url.pathname === "/admin/users");
    await expect(page.getByLabel("Search users")).toBeVisible();

    await page.keyboard.press("Slash");
    await expect(page.getByLabel("Search users")).toBeFocused();

    await page.keyboard.press("Escape");
    await page.keyboard.press("g");
    await page.keyboard.press("e");
    await expect(page).toHaveURL((url) => url.pathname === "/admin/auth-events");

    await page.keyboard.press("g");
    await page.keyboard.press("u");
    await expect(page).toHaveURL((url) => url.pathname === "/admin/users");
  });
});
