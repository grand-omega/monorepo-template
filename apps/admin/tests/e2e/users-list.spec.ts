import { expect, test } from "@playwright/test";

const adminEmail = process.env.E2E_ADMIN_EMAIL;
const adminPassword = process.env.E2E_ADMIN_PASSWORD;

test.describe("users list", () => {
  test.skip(
    !adminEmail || !adminPassword,
    "Set E2E_ADMIN_EMAIL and E2E_ADMIN_PASSWORD to run backend users tests"
  );

  test("renders after login and supports email search", async ({ page }) => {
    await page.goto("/admin/login");
    await page.getByLabel("Email").fill(adminEmail ?? "");
    await page.getByLabel("Password").fill(adminPassword ?? "");
    await page.getByRole("button", { name: "Sign in" }).click();

    await expect(page).toHaveURL((url) => url.pathname === "/admin/users");
    await expect(page.getByRole("heading", { name: "Users" })).toBeVisible();
    await expect(page.getByRole("link", { name: adminEmail ?? "" })).toBeVisible();

    await page.getByLabel("Search users").fill(adminEmail ?? "");
    await page.getByRole("button", { name: "Search" }).click();

    await expect(page).toHaveURL((url) => url.searchParams.get("q") === adminEmail);
    await expect(page.getByRole("link", { name: adminEmail ?? "" })).toBeVisible();
  });
});
