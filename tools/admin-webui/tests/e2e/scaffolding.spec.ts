import { expect, test } from "@playwright/test";

test("renders the login route", async ({ page }) => {
  await page.goto("/admin/login");
  await expect(page.getByRole("heading", { name: "Admin login" })).toBeVisible();
});
