import { test, expect, type Page } from "@playwright/test";

async function openSchool(page: Page, name: string) {
  await page.getByRole("combobox", { name: "Škola pro změnu nabídky" }).fill(name);
  await page.getByRole("option", { name: new RegExp(name) }).click();
}

test("scenario combines school changes and preserves them through individual removal and undo", async ({ page }) => {
  const bodies: {zmeny: {redizo: string; zmena_kapacity: number}[]}[] = [];
  await page.route("**/api/backend/simulace/zmeny", async route => {
    bodies.push(route.request().postDataJSON());
    await route.continue();
  });
  await page.goto("/kraj");
  await expect(page.locator(".coverage-summary")).toContainText("2 školy");
  await page.getByRole("button", { name: "Přidat obor", exact: true }).click();
  await expect(page.locator(".scenario-impact")).toContainText("2 → 3");

  await openSchool(page, "SPŠ Ostrov");
  await page.getByRole("button", { name: "Odebrat obor ze simulované nabídky" }).click();
  await expect(page.locator(".changes-summary")).toContainText("2 změny v nabídce");
  await expect(page.locator(".changes-summary")).toContainText("SLŠ Žlutice");
  await expect(page.locator(".scenario-impact")).toContainText("2 → 2");
  expect(bodies.at(-1)?.zmeny).toEqual([
    {redizo: "600009084", zmena_kapacity: -77},
    {redizo: "600009271", zmena_kapacity: 30},
  ]);
  await expect(page.locator(".simulation-result")).toContainText("154 → 107");

  await openSchool(page, "ISŠTE Sokolov");
  await page.getByRole("button", { name: "Odebrat obor ze simulované nabídky" }).click();
  await expect(page.locator(".changes-summary")).toContainText("SPŠ Ostrov");
  await expect(page.locator(".changes-summary .remove-change")).toHaveCount(3);
  await expect(page.locator(".scenario-impact")).toContainText("2 → 1");

  await expect(page.locator(".simulation-result")).toContainText("154 → 30");
  expect(bodies.at(-1)?.zmeny).toHaveLength(3);
  await page.getByRole("button", { name: "Zrušit scénář", exact: true }).click();
  await expect(page.locator(".changes-summary")).toHaveCount(0);
  await page.getByRole("button", { name: "Vrátit předchozí scénář", exact: true }).click();
  await expect(page.locator(".changes-summary .remove-change")).toHaveCount(3);
  await expect(page.locator(".changes-summary")).toContainText("ISŠTE Sokolov");

  await openSchool(page, "ISŠTE Sokolov");
  await page.getByRole("button", { name: "Vrátit odebrání oboru", exact: true }).click();
  await expect(page.locator(".changes-summary .remove-change")).toHaveCount(2);
  await expect(page.locator(".scenario-impact")).toContainText("2 → 2");
  await openSchool(page, "SPŠ Ostrov");
  await page.getByRole("button", { name: "Vrátit odebrání oboru", exact: true }).click();
  await expect(page.locator(".changes-summary .remove-change")).toHaveCount(1);
  await expect(page.locator(".scenario-impact")).toContainText("2 → 3");
  await openSchool(page, "SLŠ Žlutice");
  await page.getByRole("button", { name: "Vrátit přidání oboru", exact: true }).click();
  await expect(page.locator(".changes-summary")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Scénář", exact: true })).toBeDisabled();
});


test("editing an added school's capacity preserves other school changes", async ({page}) => {
  const bodies: {zmeny: {redizo: string; zmena_kapacity: number}[]}[] = [];
  await page.route("**/api/backend/simulace/zmeny", async route => {
    bodies.push(route.request().postDataJSON());
    await route.continue();
  });
  await page.goto("/kraj");
  await page.getByRole("button", {name: "Přidat obor", exact: true}).click();
  await expect(page.locator(".simulation-result")).toBeVisible();
  await openSchool(page, "SPŠ Ostrov");
  await page.getByRole("button", {name: "Odebrat obor ze simulované nabídky"}).click();
  await expect(page.locator(".simulation-result")).toContainText("154 → 107");
  await openSchool(page, "SLŠ Žlutice");
  await page.getByRole("spinbutton", {name: "Kapacita nového oboru"}).fill("60");
  await expect(page.locator(".simulation-result")).toContainText("154 → 137");
  expect(bodies.at(-1)?.zmeny).toEqual([
    {redizo: "600009084", zmena_kapacity: -77},
    {redizo: "600009271", zmena_kapacity: 60},
  ]);
  await page.getByRole("spinbutton", {name: "Kapacita nového oboru"}).fill("0");
  await expect(page.locator(".simulation-feedback [role=alert]")).toBeVisible();
  await page.getByRole("spinbutton", {name: "Kapacita nového oboru"}).fill("60");
  await expect(page.locator(".simulation-result")).toContainText("154 → 137");
});
