import { expect, test } from "@playwright/test";

const adminEmail = process.env.E2E_ADMIN_EMAIL;
const adminPassword = process.env.E2E_ADMIN_PASSWORD;

test.describe("auth events", () => {
  test.skip(
    !adminEmail || !adminPassword,
    "Set E2E_ADMIN_EMAIL and E2E_ADMIN_PASSWORD to run backend auth-events tests"
  );

  test("filters admin login events and opens the detail drawer", async ({ page }) => {
    await page.goto("/admin/login");
    await page.getByLabel("Email").fill(adminEmail ?? "");
    await page.getByLabel("Password").fill(adminPassword ?? "");
    await page.getByRole("button", { name: "Sign in" }).click();

    await page.getByRole("link", { name: "Auth events" }).click();
    await expect(page.getByRole("heading", { name: "Auth events" })).toBeVisible();

    await page.getByLabel("Event type").fill("admin_login_success");
    await page.getByRole("button", { name: "Filter" }).click();

    await expect(page).toHaveURL(
      (url) => url.searchParams.get("event_type") === "admin_login_success"
    );
    await expect(page.getByText("admin_login_success").first()).toBeVisible();

    await page.getByRole("button", { name: "View" }).first().click();
    await expect(page.getByLabel("Auth event detail")).toBeVisible();
    await expect(page.getByLabel("Auth event detail")).toContainText("Detail JSON");
  });
});
