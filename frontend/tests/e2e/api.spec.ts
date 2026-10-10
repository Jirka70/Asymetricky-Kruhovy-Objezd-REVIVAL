import { test, expect } from "@playwright/test";

test("server cache shares responses across requests and keeps filters separate", async ({ request }) => {
  const nonce = Date.now();
  const first = `/api/backend/skoly?obor=18-20-M%2F01&stupen=M&forma=den&test_run=${nonce}`;
  const reordered = `/api/backend/skoly?test_run=${nonce}&forma=den&stupen=M&obor=18-20-M%2F01`;
  const key = `/api/v1/skoly?forma=den&obor=18-20-M%2F01&stupen=M&test_run=${nonce}`;
  const before = await (await request.get("http://127.0.0.1:8107/__counts")).json();
  expect((await request.get(first)).status()).toBe(200);
  expect((await request.get(reordered)).status()).toBe(200);
  const after = await (await request.get("http://127.0.0.1:8107/__counts")).json();
  expect((after[key] ?? 0) - (before[key] ?? 0)).toBe(1);
  const distance = await request.get(first.replace("forma=den", "forma=dal"));
  expect((await distance.json()).features).toHaveLength(0);
  const employers = await request.get("/api/backend/obory/18-20-M%2F01/zamestnavatele");
  expect(employers.status()).toBe(200);
  expect((await employers.json()).features[0].properties.pocet_mist).toBe(321);
  expect((await request.get("/api/backend/student/unsupported")).status()).toBe(404);
});

test("region loads a skeleton, uses API school details and reuses cached form results", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  let release!: () => void;
  const gate = new Promise<void>((resolve) => { release = resolve; });
  await page.route("**/api/backend/obory?forma=den", async (route) => {
    await gate;
    await route.continue();
  });
  await page.goto("/kraj");
  await expect(page.locator(".data-skeleton")).toBeVisible();
  release();
  await expect(page.getByRole("combobox", { name: "Forma studia" })).toBeVisible();
  await expect(page.locator(".map-frame")).toBeVisible();

  const schoolRequests: string[] = [];
  page.on("request", (request) => {
    if (request.url().includes("/api/backend/skoly?") && request.url().includes("obor=")) {
      schoolRequests.push(request.url());
    }
  });
  const form = page.getByRole("combobox", { name: "Forma studia" });
  await form.selectOption("dal");
  await expect(page.locator(".coverage-summary")).toContainText("0 školy");
  await form.selectOption("den");
  await expect(page.locator(".coverage-summary")).toContainText("2 školy");
  expect(schoolRequests).toHaveLength(1);

  await expect(page.getByText("Seznam míst", { exact: true })).toHaveCount(0);
  await page.getByRole("combobox", { name: "Škola pro změnu nabídky" }).fill("SPŠ Ostrov");
  await page.getByRole("option", { name: /SPŠ Ostrov/ }).click();
  await expect(page.getByText("Adresa z API 123", { exact: true })).toBeVisible();
  await expect(page.locator(".technical-details")).toContainText("Kapacita: 77");
  await expect(page.getByRole("region", { name: "Statistiky přijetí" })).toBeVisible();
  await expect(page.locator(".admission-numbers dd")).toHaveText(["999", "50", "5 %"]);
  await page.getByRole("button", { name: "Odebrat obor ze simulované nabídky" }).click();
  await expect(page.locator(".scenario-impact")).toContainText("2 → 1");
  expect(errors).toEqual([]);
});

test("API failure offers retry without quietly showing the snapshot", async ({ page }) => {
  await page.route("**/api/backend/skoly?forma=den", (route) => route.fulfill({
    status: 503, contentType: "application/json", body: '{"error":{"zprava":"offline"}}',
  }));
  await page.goto("/kraj");
  await expect(page.locator(".query-notice[role=alert]")).toContainText("Data se nepodařilo načíst");
  await expect(page.locator(".map-frame")).toHaveCount(0);
  await page.unroute("**/api/backend/skoly?forma=den");
  await page.getByRole("button", { name: "Zkusit znovu" }).click();
  await expect(page.locator(".map-frame")).toBeVisible();
});

test("family search uses API travel times and offers; empty form can recover", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("/rodiny");
  await expect(page.locator(".journey-result").first()).toBeVisible();
  await expect(page.locator(".journey-result").first().locator(".admission-numbers dd")).toHaveText(["999", "50", "5 %"]);
  await page.getByRole("combobox", { name: "Forma studia" }).selectOption("dal");
  await page.getByRole("button", { name: "Najít školy", exact: true }).click();
  await expect(page.getByText("Tento obor v této formě nemá v nabídce žádná škola.")).toBeVisible();
  await page.getByRole("button", { name: "Zkusit denní studium" }).click();
  await expect(page.locator(".journey-result").first()).toBeVisible();
  await page.locator(".journey-select").first().click();
  await expect(page.getByText("Adresa z API 123", { exact: true })).toBeVisible();
  await expect(page.getByRole("region", { name: "Statistiky přijetí" })).toHaveCount(1);
  await expect(page.locator(".admission-numbers dd")).toHaveText(["999", "50", "5 %"]);
  expect(errors).toEqual([]);
});

test("employer failures do not hide schools or masquerade as no employers", async ({ page }) => {
  await page.route("**/api/backend/obory/*/zamestnavatele?*", (route) => route.fulfill({
    status: 503, contentType: "application/json", body: "{}",
  }));
  await page.goto("/kraj");
  await expect(page.locator(".map-frame")).toBeVisible();
  await expect(page.locator(".query-notice[role=alert]")).toContainText("Data se nepodařilo načíst");
  await expect(page.locator(".coverage-summary")).toContainText("2 školy");
  await page.unroute("**/api/backend/obory/*/zamestnavatele?*");
  await page.getByRole("button", { name: "Zkusit znovu" }).click();
  await page.getByRole("button", { name: /Zaměstnavatel z API ·.*zobrazit detail/ }).click();
  await expect(page.getByRole("heading", { name: "321 míst v příbuzných profesích" })).toBeVisible();
  await expect(page.getByText("Technici z API", { exact: true })).toBeVisible();
});
