import { test, expect, type Page } from "@playwright/test";

const add = (page: Page) => page.getByRole("button", { name: "Přidat obor", exact: true });

test("simulation renders API results, loads a skeleton and caches undo and parameter changes", async ({ page }) => {
  const urls: URL[] = [];
  let release!: () => void;
  const gate = new Promise<void>((resolve) => { release = resolve; });
  await page.route("**/api/backend/simulace?*", async (route) => {
    urls.push(new URL(route.request().url()));
    await gate;
    await route.continue();
  });
  await page.goto("/kraj");
  await expect(page.locator(".coverage-summary")).toBeVisible();
  expect(urls).toHaveLength(0);
  await add(page).click();
  await expect(page.locator(".data-skeleton")).toBeVisible();
  await expect(page.locator(".scenario-impact")).toHaveCount(0);
  release();
  await expect(page.locator(".simulation-result")).toContainText("17,25");
  await expect(page.locator(".simulation-result")).toContainText("Dobré místo");
  await expect(page.locator(".coverage-summary")).toContainText("10 dětí v dosahu");
  await expect(page.locator(".coverage-summary")).toContainText("dětí v odhadovaném ročníku");
  expect(urls).toHaveLength(1);
  expect(Object.fromEntries(urls[0].searchParams)).toEqual({
    redizo: "600009271", obor: "18-20-M/01", kapacita: "30", max_min: "45",
    uroven: "zsj", format: "geojson", scenar: "rano",
  });
  await page.getByRole("button", { name: "Zrušit scénář", exact: true }).click();
  await page.getByRole("button", { name: "Vrátit předchozí scénář", exact: true }).click();
  await page.getByRole("combobox", { name: "Škola pro změnu nabídky" }).fill("SLŠ Žlutice");
  await page.getByRole("option", { name: /SLŠ Žlutice/ }).click();
  await expect(page.locator(".simulation-result")).toContainText("17,25");
  expect(urls).toHaveLength(1);
  await page.getByRole("spinbutton", { name: "Kapacita nového oboru" }).fill("60");
  await expect(page.locator(".simulation-result")).toContainText("28,75 %");
  await page.getByRole("combobox", { name: "Hranice dostupnosti" }).selectOption("60");
  await expect.poll(() => urls.length).toBe(3);
  expect(urls[2].searchParams.get("max_min")).toBe("60");
  await expect(page.locator(".simulation-result")).toBeVisible();
  await page.getByRole("spinbutton", { name: "Kapacita nového oboru" }).fill("0");
  await expect(page.getByRole("alert").filter({ hasText: /Zadejte|Data se/ })).toContainText("Zadejte celou kapacitu");
  await expect(page.locator(".simulation-result")).toHaveCount(0);
  expect(urls).toHaveLength(3);
});

test("simulation failure offers retry without a local replacement; sufficient capacity stays a no-op", async ({ page }) => {
  await page.route("**/api/backend/simulace?*", (route) => route.fulfill({ status: 503, json: { error: { kod: "offline" } } }));
  await page.goto("/kraj");
  await add(page).click();
  await expect(page.getByRole("alert").filter({ hasText: /Zadejte|Data se/ })).toContainText("Data se nepodařilo načíst");
  await expect(page.locator(".scenario-impact")).toHaveCount(0);
  await expect(page.locator(".map-frame")).toHaveCount(0);
  await page.unroute("**/api/backend/simulace?*");
  await page.getByRole("button", { name: "Zkusit znovu" }).click();
  await expect(page.locator(".simulation-result")).toBeVisible();
  await page.route("**/api/backend/simulace?*", (route) => route.fulfill({ json: {
    type: "FeatureCollection", features: [], souhrn: null, meta: { duvod: "kapacita_staci" },
  } }));
  await page.getByRole("spinbutton", { name: "Kapacita nového oboru" }).fill("31");
  await expect(page.getByText("Současná kapacita podle modelu stačí. Simulace neprovedla změnu.")).toBeVisible();
  await expect(page.locator(".scenario-impact")).toHaveCount(0);
  await expect(page.locator(".coverage-summary")).toContainText("2 školy");
  await page.getByRole("button", { name: "Zrušit scénář", exact: true }).click();
});

test("cancelling a pending simulation prevents stale results and distance additions are disabled", async ({ page }) => {
  let release!: () => void;
  const gate = new Promise<void>((resolve) => { release = resolve; });
  await page.route("**/api/backend/simulace?*", async (route) => { await gate; await route.continue().catch(() => {}); });
  await page.goto("/kraj");
  await add(page).click();
  await expect(page.locator(".data-skeleton")).toBeVisible();
  await page.getByRole("button", { name: "Zrušit scénář", exact: true }).click();
  release();
  await expect(page.locator(".coverage-summary")).toContainText("2 školy");
  await expect(page.locator(".simulation-result")).toHaveCount(0);
  await page.getByRole("combobox", { name: "Forma studia" }).selectOption("dal");
  await expect(add(page)).toBeDisabled();
  await expect(page.locator(".coverage-summary")).toContainText("0 školy");
});


test("server caches compact simulation values independently of query order and parameters", async ({ request }) => {
  const nonce = Date.now();
  const params = `redizo=600009271&obor=18-20-M%2F01&kapacita=30&max_min=45&uroven=zsj&format=geojson&scenar=rano&test_run=${nonce}`;
  const first = await request.get(`/api/backend/simulace?${params}`);
  expect(first.status()).toBe(200);
  const data = await first.json();
  expect(data.features).toHaveLength(839);
  expect(data.features[0]).not.toHaveProperty("geometry");
  const second = await request.get(`/api/backend/simulace?${params.split("&").reverse().join("&")}`);
  expect(await second.json()).toEqual(data);
  await request.get(`/api/backend/simulace?${params.replace("kapacita=30", "kapacita=60")}`);
  const counts = await (await request.get("http://127.0.0.1:8107/__counts")).json();
  const keys = Object.keys(counts).filter(key => key.includes(`/simulace?`) && key.includes(`test_run=${nonce}`));
  expect(keys).toHaveLength(2);
  expect(keys.map(key => counts[key])).toEqual([1, 1]);
});


test("simulation domain errors preserve their status and are not cached", async ({ request }) => {
  const nonce = Date.now();
  const path = `/api/backend/simulace?redizo=600000000&obor=18-20-M%2F01&kapacita=30&test_run=${nonce}`;
  for (let i = 0; i < 2; i++) {
    const response = await request.get(path);
    expect(response.status()).toBe(404);
    expect((await response.json()).error.kod).toBe("skola_nenalezena");
  }
  const counts = await (await request.get("http://127.0.0.1:8107/__counts")).json();
  const key = Object.keys(counts).find(key => key.includes(`/simulace?`) && key.includes(`test_run=${nonce}`));
  expect(key).toBeTruthy();
  expect(counts[key!]).toBe(2);
});
