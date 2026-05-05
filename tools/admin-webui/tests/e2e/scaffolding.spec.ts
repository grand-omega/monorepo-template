import { expect, test } from "@playwright/test";

test("renders the scaffolding placeholder", async ({ page }) => {
  await page.goto("/admin/");
  await expect(page.getByRole("heading", { name: "lab-rust-server admin" })).toBeVisible();
});
