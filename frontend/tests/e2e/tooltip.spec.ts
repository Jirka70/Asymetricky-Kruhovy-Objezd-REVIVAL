import { test, expect, type Page } from "@playwright/test";
import { readFileSync } from "node:fs";

const geometry = JSON.parse(readFileSync("public/data/zsj.geojson", "utf8"));

// GeoJSON order matches the SVG regions. Find an interior point that is not
// covered by a school marker, so the test exercises a real pointer hover.
async function hoverZone(page: Page, id: string) {
  const index = geometry.features.findIndex((f: { properties: { name: string } }) => f.properties.name === id);
  expect(index).toBeGreaterThanOrEqual(0);
  const area = page.locator(".map-frame .echart svg path").nth(index);
  // Closing the detail resizes the map and replaces its SVG paths.
  await expect(async () => {
    await page.locator(".map-frame").scrollIntoViewIfNeeded();
    await expect(area).toBeVisible();
    const point = await area.evaluate((element) => {
    const path = element as SVGPathElement;
    const bounds = path.getBBox();
    for (let x = 0.2; x < 0.9; x += 0.1) {
      for (let y = 0.2; y < 0.9; y += 0.1) {
        const local = new DOMPoint(bounds.x + bounds.width * x, bounds.y + bounds.height * y);
        const screen = local.matrixTransform(path.getScreenCTM()!);
        if (path.isPointInFill(local) && document.elementFromPoint(screen.x, screen.y) === path) {
          return { x: screen.x, y: screen.y };
        }
      }
    }
    throw new Error("No uncovered interior point in ZSJ polygon");
  });
    await page.mouse.move(point.x, point.y);
  }).toPass({ timeout: 5000 });
}

test("ZSJ tooltip waits half a second, describes the area and clears when leaving", async ({ page }) => {
  await page.goto("/kraj");
  await expect(page.locator(".map-frame .echart svg path").first()).toBeVisible();
  const tooltip = page.locator(".zsj-tooltip");
  await hoverZone(page, "001261");
  // Real timing is intentional: verify the visible hover behaviour.
  await page.waitForTimeout(200);
  await expect(tooltip).toBeHidden();
  await expect(tooltip).toBeVisible({ timeout: 1500 });
  await expect(tooltip).toContainText("Bečov nad Teplou");
  await expect(tooltip).toContainText("Obec: Bečov nad Teplou");
  await expect(tooltip).toContainText("ZSJ 001261");
  await expect(tooltip).toContainText("Odhad dětí 10–14 let: 48");
  await expect(tooltip).toContainText("Demografický podklad: 2021");
  await expect(tooltip).toContainText("K nejbližší škole s vybraným oborem");
  await expect(tooltip).toContainText("Dojezd:");
  await expect(tooltip).toContainText("Nejbližší škola: ISŠTE Sokolov z API");

  await hoverZone(page, "000019");
  await expect(tooltip).toBeHidden();
  await page.waitForTimeout(200);
  await expect(tooltip).toBeHidden();
  await expect(tooltip).toBeVisible({ timeout: 1500 });
  await expect(tooltip).toContainText("Abertamy");
  await expect(tooltip).not.toContainText("Bečov nad Teplou");
  await expect(tooltip).toContainText("Nejbližší škola: SPŠ Ostrov z API");
  await expect(tooltip).not.toContainText("ISŠTE Sokolov z API");
  await page.mouse.move(10, 10);
  await expect(tooltip).toBeHidden();

  await hoverZone(page, "001261");
  await page.waitForTimeout(100);
  await page.mouse.move(10, 10);
  await page.waitForTimeout(600);
  await expect(tooltip).toBeHidden();
});

test("ZSJ tooltip distinguishes missing journeys and compares the scenario", async ({ page }) => {
  await page.goto("/kraj");
  await expect(page.locator(".map-frame .echart svg path").first()).toBeVisible();
  await page.getByRole("combobox", { name: "Forma studia" }).selectOption("dal");
  await expect(page.locator(".coverage-summary")).toContainText("0 školy");
  await hoverZone(page, "001261");
  const tooltip = page.locator(".zsj-tooltip");
  await expect(tooltip).toBeVisible();
  await expect(tooltip).toContainText("Bez uloženého spojení");
  await expect(tooltip).not.toContainText("Nejbližší škola:");
  await expect(page.getByRole("button", { name: "Přidat obor", exact: true })).toBeDisabled();
  await page.getByRole("combobox", { name: "Forma studia" }).selectOption("den");
  await page.getByRole("button", { name: "Přidat obor", exact: true }).click();
  await expect(page.locator(".simulation-result")).toBeVisible();
  await page.getByRole("button", { name: /Zavřít detail/ }).click();
  await hoverZone(page, "001261");
  await expect(tooltip).toBeVisible();
  await expect(tooltip).toContainText("Současný stav: Bez uloženého spojení");
  await expect(tooltip).toContainText(/Scénář: \d+ min/);
  await expect(tooltip).toContainText("Nejbližší škola: SLŠ Žlutice z API");
});
