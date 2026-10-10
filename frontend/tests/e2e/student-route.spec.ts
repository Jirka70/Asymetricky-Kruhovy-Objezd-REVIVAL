import { test, expect } from "@playwright/test";

const openSchool = async (page: import("@playwright/test").Page) => {
  await page.locator(".journey-results").getByRole("button", { name: "Detail školy SPŠ Ostrov z API" }).click();
};

test("family results show API connections and load ordered route legs and alternatives on selection", async ({ page }) => {
  // Keep automated runs off the public tile service.
  await page.route("https://tile.openstreetmap.org/**", route => route.fulfill({
    contentType: "image/svg+xml",
    body: '<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256"><rect width="256" height="256" fill="#eef1e8"/><path d="M0 128H256M128 0V256" stroke="#fff" stroke-width="8"/></svg>',
  }));
  const urls: URL[] = [];
  let release!: () => void;
  const gate = new Promise<void>(resolve => { release = resolve; });
  await page.route("**/api/backend/student/trasa?*", async route => {
    urls.push(new URL(route.request().url()));
    await gate;
    const response = await route.fetch();
    await route.fulfill({response});
  });
  await page.goto("/rodiny");
  const first = page.locator(".journey-results .journey-result").first();
  await expect(first).toContainText("Odjezd 07:12 · Příjezd 07:29");
  await expect(first).toContainText("Bez přestupu");
  await expect(first).toContainText("Chůze 250 m");
  await expect(first).toContainText("Linky: 421");
  expect(urls).toHaveLength(0);
  await openSchool(page);
  const detail = page.locator(".student-route");
  await expect(detail.locator(".data-skeleton")).toBeVisible();
  await expect.poll(() => urls.length).toBe(1);
  expect(urls[0].searchParams.get("redizo")).toBe("600009084");
  expect(urls[0].searchParams.get("lat")).toBeTruthy();
  release();
  await expect(detail).toContainText("Den spojení: 12. 10. 2026");
  await expect(detail.locator(".route-legs li")).toHaveCount(2);
  await expect(detail.locator(".route-legs li").first()).toContainText("Chůze");
  await expect(detail.locator(".route-legs li").nth(1)).toContainText("Autobus · 421");
  await expect(detail.getByRole("region", {name: "Mapa vybraného spojení"})).toBeVisible();
  await expect(detail.locator("path.route-line")).toHaveCount(2);
  await expect(detail.locator(".leaflet-tile-loaded").first()).toBeVisible();
  await expect(detail.locator(".leaflet-control-attribution").getByRole("link", {name: "OpenStreetMap"})).toBeVisible();
  await detail.getByRole("button", {name: "Přiblížit trasu"}).click();
  await detail.getByRole("button", {name: "Zobrazit celou trasu"}).click();
  await detail.getByRole("button", {name: /Spoj 2/}).click();
  await expect(detail.locator(".route-legs li")).toHaveCount(1);
  await expect(detail).toContainText("Nádraží z API → Cílové nádraží");
  await expect(detail).not.toContainText("Výchozí bod ZSJ");
  await page.getByRole("button", {name: "Zpět na školy"}).click();
  await openSchool(page);
  await expect(detail).toContainText("Výchozí bod ZSJ");
  expect(urls).toHaveLength(1);
  await detail.screenshot({path: test.info().outputPath("route-desktop.png")});
  await page.setViewportSize({width: 390, height: 844});
  await detail.scrollIntoViewIfNeeded();
  await expect(detail.locator(".route-legs li")).toHaveCount(2);
  await detail.screenshot({path: test.info().outputPath("route-mobile.png")});
});

test("route failures preserve the summary and retry; no-route responses stay empty", async ({page}) => {
  await page.route("**/api/backend/student/trasa?*", route => route.fulfill({status: 502, json: {error: {kod: "otp_nedostupne"}}}));
  await page.goto("/rodiny");
  await openSchool(page);
  const detail = page.locator(".student-route");
  await expect(detail.getByRole("alert")).toBeVisible();
  await expect(page.locator(".place-detail-content .journey-result")).toContainText("17 min");
  await expect(detail.locator(".route-legs li")).toHaveCount(0);
  await page.unroute("**/api/backend/student/trasa?*");
  await page.route("**/api/backend/student/trasa?*", route => route.fulfill({json: {features: [], spoje: [], meta: {den: "2026-10-12"}}}));
  await detail.getByRole("button", {name: "Zkusit znovu"}).click();
  await expect(detail).toContainText("nebylo v ranním okně nalezeno spojení");
  await expect(detail.locator("path.route-line")).toHaveCount(0);
});

test("unavailable OSM tiles leave route geometry and journey legs usable", async ({page}) => {
  await page.route("https://tile.openstreetmap.org/**", route => route.abort());
  await page.goto("/rodiny");
  await openSchool(page);
  const detail = page.locator(".student-route");
  await expect(detail).toContainText("Mapový podklad se nepodařilo načíst");
  await expect(detail.locator("path.route-line")).toHaveCount(2);
  await expect(detail.locator(".route-legs li")).toHaveCount(2);
  await detail.getByRole("button", {name: /Spoj 2/}).click();
  await expect(detail.locator("path.route-line")).toHaveCount(1);
  await expect(detail.locator(".route-legs li")).toHaveCount(1);
});
