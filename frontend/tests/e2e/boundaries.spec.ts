import { test, expect, type Page } from "@playwright/test";
import { readFileSync } from "node:fs";

const municipalities = JSON.parse(readFileSync("public/data/municipalities.geojson", "utf8"));
const ringCount = municipalities.features.reduce((total: number, feature: {
  geometry: { type: string; coordinates: unknown[][][] };
}) => total + (feature.geometry.type === "Polygon"
  ? feature.geometry.coordinates.length
  : feature.geometry.coordinates.reduce((n, polygon) => n + polygon.length, 0)), 0);

async function bounds(page: Page) {
  return page.locator(".map-frame .echart").evaluate((chart) => {
    const zone = chart.querySelectorAll("svg path")[14]; // Bečov nad Teplou
    const border = chart.querySelector('path[stroke="#fff"][stroke-width="0.7"][fill="none"]')!;
    return [zone, border].map((element) => {
      const box = element.getBoundingClientRect();
      return { x: box.x, y: box.y, width: box.width, height: box.height };
    });
  });
}

test("municipal outlines remain aligned with ZSJ through zoom, pan and reset", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("/kraj");
  const borders = page.locator('.map-frame .echart path[stroke="#fff"][stroke-width="0.7"][fill="none"]');
  await expect(borders).toHaveCount(ringCount);
  await expect(borders.first()).toHaveAttribute("stroke-width", "0.7");
  await expect(borders.first()).toHaveAttribute("fill", "none");
  await expect(page.locator('.map-frame .echart path[stroke="#fff"][stroke-width="0.3"]')).toHaveCount(839);
  const initial = await bounds(page);

  await page.getByRole("button", { name: "Přiblížit mapu", exact: true }).click();
  await expect.poll(async () => (await bounds(page))[0].width / initial[0].width).toBeCloseTo(1.4, 2);
  const zoomed = await bounds(page);
  expect(zoomed[1].width / initial[1].width).toBeCloseTo(1.4, 2);
  expect(zoomed[1].x - zoomed[0].x).toBeCloseTo((initial[1].x - initial[0].x) * 1.4, 1);
  expect(zoomed[1].y - zoomed[0].y).toBeCloseTo((initial[1].y - initial[0].y) * 1.4, 1);
  await expect(borders.first()).toHaveAttribute("stroke-width", "0.7");

  const map = await page.locator(".map-frame .echart").boundingBox();
  if (!map) throw new Error("Map is missing");
  const origin = { x: map.x + map.width * 0.7, y: map.y + map.height * 0.45 };
  await page.mouse.move(origin.x, origin.y);
  await page.mouse.down();
  await page.mouse.move(origin.x + 50, origin.y + 25, { steps: 5 });
  await page.mouse.up();
  await expect.poll(async () => (await bounds(page))[0].x - zoomed[0].x).toBeCloseTo(50, 1);
  const panned = await bounds(page);
  expect(panned[1].x - zoomed[1].x).toBeCloseTo(50, 1);
  expect(panned[1].y - zoomed[1].y).toBeCloseTo(25, 1);

  await page.getByRole("button", { name: "Zobrazit celý kraj", exact: true }).click();
  await expect.poll(async () => (await bounds(page))[0].width).toBeCloseTo(initial[0].width, 1);
  const reset = await bounds(page);
  expect(reset[1].x).toBeCloseTo(initial[1].x, 1);
  expect(reset[1].y).toBeCloseTo(initial[1].y, 1);
  expect(reset[1].width).toBeCloseTo(initial[1].width, 1);
  expect(errors).toEqual([]);
});
