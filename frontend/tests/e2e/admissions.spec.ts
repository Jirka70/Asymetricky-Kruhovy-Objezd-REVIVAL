import { test, expect } from "@playwright/test";

test("school detail distinguishes missing admission counts, zero accepted and an unoffered program", async ({ page }) => {
  const openSchool = async () => {
    await page.getByRole("combobox", { name: "Škola pro změnu nabídky" }).fill("SPŠ Ostrov");
    await page.getByRole("option", { name: /SPŠ Ostrov/ }).click();
  };
  await page.route("**/api/backend/skoly/600009084", async route => {
    const response = await route.fetch();
    const detail = await response.json();
    await route.fulfill({ json: {
      ...detail, nabidky: [
        { ...detail.nabidky[0], prihlasky: 95, prijati: null },
        { ...detail.nabidky[0], kod_oboru: "23-51-E/01", prihlasky: 20, prijati: 0 },
      ],
    } });
  });
  await page.goto("/kraj");
  await openSchool();
  await expect(page.locator(".admission-numbers dd")).toHaveText(["95", "Neuvedeno", "Nelze určit"]);
  await page.getByRole("combobox", { name: "Obor", exact: true }).fill("Strojírenské práce");
  await page.getByRole("option", { name: /Strojírenské práce/ }).click();
  await openSchool();
  await expect(page.locator(".admission-numbers dd")).toHaveText(["20", "0", "0 %"]);
  await page.getByRole("combobox", { name: "Forma studia" }).selectOption("dal");
  await openSchool();
  await expect(page.getByText("Pro tento obor a formu nemáme údaje o přijímání.")).toBeVisible();
  await expect(page.locator(".admission-numbers")).toHaveCount(0);
});
