import { test, expect } from "@playwright/test";

test("map uses API accessibility flags and cohort; program details and candidates use their endpoint", async ({page}) => {
  const requests: URL[] = [];
  page.on("request", request => {if (request.url().includes("/api/backend/zsj?")) requests.push(new URL(request.url()));});
  await page.goto("/kraj");
  await expect(page.locator(".coverage-summary")).toContainText("0 %");
  await expect(page.locator(".coverage-summary")).toContainText("8 390");
  await expect(page.locator(".program-detail")).toContainText("Detail IT z API");
  await expect(page.locator(".program-detail")).toContainText("Kapacita: 432 · Přihlášky: 876");
  await expect(page.locator(".program-detail")).toContainText("Pro obor není dostupné mapování trhu práce");
  await page.locator(".program-detail").getByRole("button", {name: "SLŠ Žlutice z API"}).click();
  await expect(page.locator(".place-title")).toContainText("SLŠ Žlutice");
  await expect(page.locator(".scenario-impact")).toHaveCount(0);
  await page.getByRole("combobox", {name: "Hranice dostupnosti"}).selectOption("60");
  await expect.poll(() => requests.length).toBe(2);
  expect(requests[1].searchParams.get("max_min")).toBe("60");
  expect(requests[0].searchParams.get("uroven")).toBe("zsj");
  await expect(page.locator(".coverage-summary")).toBeVisible();
  await page.getByRole("combobox", {name: "Hranice dostupnosti"}).selectOption("45");
  await expect(page.locator(".coverage-summary")).toContainText("do 45 minut");
  expect(requests).toHaveLength(2);
});

test("map errors do not fall back; program-detail errors leave the map available", async ({page}) => {
  await page.route("**/api/backend/zsj?*", route => route.fulfill({status: 503, json: {}}));
  await page.goto("/kraj");
  await expect(page.locator(".query-notice[role=alert]")).toBeVisible();
  await expect(page.locator(".map-frame")).toHaveCount(0);
  await page.unroute("**/api/backend/zsj?*");
  await page.route("**/api/backend/obory/*?max_min=*", route => route.fulfill({status: 404, json: {}}));
  await page.getByRole("button", {name: "Zkusit znovu"}).click();
  await expect(page.locator(".map-frame")).toBeVisible();
  await expect(page.locator(".program-detail [role=alert]")).toBeVisible();
  await page.unroute("**/api/backend/obory/*?max_min=*");
  await page.locator(".program-detail").getByRole("button", {name: "Zkusit znovu"}).click();
  await expect(page.locator(".program-detail")).toContainText("Detail IT z API");
});

test("family search uses API order, unknown and out-of-limit times; filters apply on submit", async ({page}) => {
  const urls: URL[] = [];
  let release!: () => void;
  const gate = new Promise<void>(resolve => {release = resolve;});
  await page.route("**/api/backend/student/skoly?*", async route => {urls.push(new URL(route.request().url())); await gate; await route.continue();});
  await page.goto("/rodiny");
  await expect(page.locator(".data-skeleton")).toBeVisible();
  release();
  await expect(page.locator(".journey-result")).toHaveCount(3);
  await expect(page.locator(".journey-result").nth(0)).toContainText("17 min");
  await expect(page.locator(".journey-result").nth(1)).toContainText("180 min");
  await expect(page.locator(".journey-result").nth(1)).toContainText("Mimo zadaný limit");
  await expect(page.locator(".journey-result").nth(2)).toContainText("Dojezd neznámý");
  await expect(page.getByRole("combobox", {name: "Časová podmínka"})).toHaveCount(0);
  await expect(page.getByText("Porovnat cesty na časové ose")).toHaveCount(0);
  expect(urls[0].searchParams.get("lat")).toBeTruthy();
  expect(urls[0].searchParams.get("lon")).toBeTruthy();
  await page.getByRole("combobox", {name: "Maximální dojezd"}).selectOption("30");
  expect(urls).toHaveLength(1);
  await page.getByRole("button", {name: "Najít školy", exact: true}).click();
  await expect.poll(() => urls.length).toBe(2);
  expect(urls[1].searchParams.get("max_min")).toBe("30");
  await expect(page.locator(".journey-result")).toHaveCount(3);
  await page.getByRole("combobox", {name: "Odkud"}).fill("Abertamy");
  await page.getByRole("option", {name: /Abertamy/}).first().click();
  await page.getByRole("button", {name: "Najít školy", exact: true}).click();
  await expect.poll(() => urls.length).toBe(3);
  expect(urls[2].searchParams.get("lat")).not.toBe(urls[0].searchParams.get("lat"));
});

test("student endpoint failure supports retry without local journeys", async ({page}) => {
  await page.route("**/api/backend/student/skoly?*", route => route.fulfill({status: 422, json: {error: {kod: "mimo_uzemi"}}}));
  await page.goto("/rodiny");
  await expect(page.locator(".query-notice[role=alert]")).toBeVisible();
  await expect(page.locator(".journey-result")).toHaveCount(0);
  await page.unroute("**/api/backend/student/skoly?*");
  await page.getByRole("button", {name: "Zkusit znovu"}).click();
  await expect(page.locator(".journey-result")).toHaveCount(3);
});


test("analytical endpoints share the daily server cache and strip redundant ZSJ geometry", async ({request}) => {
  const nonce = Date.now();
  const routes = [
    `/zsj?obor=18-20-M%2F01&forma=den&max_min=45&uroven=zsj&test_run=${nonce}`,
    `/obory/18-20-M%2F01?max_min=45&kandidatu=5&test_run=${nonce}`,
    `/student/skoly?lat=50.2&lon=12.8&obor=18-20-M%2F01&forma=den&max_min=120&test_run=${nonce}`,
  ];
  for (const route of routes) {
    const first = await request.get(`/api/backend${route}`);
    expect(first.status()).toBe(200);
    const body = await first.json();
    if (route.startsWith("/zsj")) expect(body.features[0]).not.toHaveProperty("geometry");
    const [path, query] = route.split("?");
    const second = await request.get(`/api/backend${path}?${query.split("&").reverse().join("&")}`);
    expect(await second.json()).toEqual(body);
  }
  const counts = await (await request.get("http://127.0.0.1:8107/__counts")).json();
  const keys = Object.keys(counts).filter(key => key.includes(`test_run=${nonce}`));
  expect(keys).toHaveLength(3);
  expect(keys.map(key => counts[key])).toEqual([1, 1, 1]);
});

test("unlimited duration is sent to student and map APIs and can be switched back", async ({page}) => {
  const urls: URL[] = [];
  page.on("request", request => { if (request.url().includes("/api/backend/student/skoly?") || request.url().includes("/api/backend/zsj?")) urls.push(new URL(request.url())); });
  await page.goto("/rodiny");
  await expect(page.locator(".journey-results .journey-result")).toHaveCount(3);
  await page.getByRole("combobox", {name: "Maximální dojezd"}).selectOption("0");
  await page.getByRole("button", {name: "Najít školy", exact: true}).click();
  await expect(page.locator(".journey-list-heading")).toContainText("Bez časového limitu");
  await expect(page.locator(".journey-results .journey-result").nth(1)).toContainText("Spojení nalezeno · bez časového limitu");
  await expect(page.locator(".journey-results .journey-result").nth(2)).toContainText("Dojezd neznámý");
  expect(urls.filter(url => url.searchParams.get("max_min") === "0")).toHaveLength(2);
  await page.getByRole("combobox", {name: "Maximální dojezd"}).selectOption("30");
  await page.getByRole("button", {name: "Najít školy", exact: true}).click();
  await expect(page.locator(".journey-list-heading")).toContainText("Do 30 minut");
  await expect(page.locator(".journey-results .journey-result").nth(1)).toContainText("Mimo zadaný limit");
  await page.goto("/kraj");
  await page.getByRole("combobox", {name: "Hranice dostupnosti"}).selectOption("0");
  await expect(page.locator(".coverage-summary")).toContainText("bez časového limitu");
  await expect(page.locator(".coverage-summary")).toContainText("100 %");
  await expect(page.locator(".program-detail")).toContainText("ranní dojezd bez časového limitu");
});
