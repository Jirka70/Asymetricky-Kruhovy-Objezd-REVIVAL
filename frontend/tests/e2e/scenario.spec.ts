import { test, expect, type Page } from "@playwright/test";

async function openSchool(page: Page, name: string) {
  await page.getByRole("combobox", { name: "Škola pro změnu nabídky" }).fill(name);
  await page.getByRole("option", { name: new RegExp(name) }).click();
}

test("scenario retains only one school offering change, including after undo", async ({ page }) => {
  await page.goto("/kraj");
  await expect(page.locator(".coverage-summary")).toContainText("2 školy");
  await page.getByRole("button", { name: "Přidat obor", exact: true }).click();
  await expect(page.locator(".scenario-impact")).toContainText("2 → 3");

  await openSchool(page, "SPŠ Ostrov");
  await page.getByRole("button", { name: "Odebrat obor ze simulované nabídky" }).click();
  await expect(page.locator(".changes-summary")).toContainText("1 změna v nabídce");
  await expect(page.locator(".changes-summary")).not.toContainText("SLŠ Žlutice");
  await expect(page.locator(".scenario-impact")).toContainText("2 → 1");

  await openSchool(page, "ISŠTE Sokolov");
  await page.getByRole("button", { name: "Odebrat obor ze simulované nabídky" }).click();
  await expect(page.locator(".changes-summary")).not.toContainText("SPŠ Ostrov");
  await expect(page.locator(".changes-summary .remove-change")).toHaveCount(1);
  await expect(page.locator(".scenario-impact")).toContainText("2 → 1");

  await page.getByRole("button", { name: "Zrušit scénář", exact: true }).click();
  await expect(page.locator(".changes-summary")).toHaveCount(0);
  await page.getByRole("button", { name: "Vrátit předchozí scénář", exact: true }).click();
  await expect(page.locator(".changes-summary .remove-change")).toHaveCount(1);
  await expect(page.locator(".changes-summary")).toContainText("ISŠTE Sokolov");

  await openSchool(page, "ISŠTE Sokolov");
  await page.getByRole("button", { name: "Vrátit odebrání oboru", exact: true }).click();
  await expect(page.locator(".changes-summary")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Scénář", exact: true })).toBeDisabled();
});
