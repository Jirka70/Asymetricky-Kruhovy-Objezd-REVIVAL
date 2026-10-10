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

for (const path of ["/kraj", "/rodiny"]) {
  test(`acceptance rate is visible at the top of school detail and shares one request (${path})`, async ({page}) => {
    let requests = 0;
    await page.route("**/api/backend/skoly/600009084", async route => {
      requests++;
      const response=await route.fetch(); const school=await response.json();
      await route.fulfill({json:{...school,nabidky:[
        {...school.nabidky[0],prihlasky:95,prijati:30,kapacita:70},
        {...school.nabidky[0],forma:"dal",prihlasky:10,prijati:10},
      ]}});
    });
    await page.goto(path);
    if(path === "/kraj") {
      await page.getByRole("combobox",{name:"Škola pro změnu nabídky"}).fill("SPŠ Ostrov");
      await page.getByRole("option",{name:/SPŠ Ostrov/}).click();
    } else {
      await page.getByRole("button",{name:"Detail školy SPŠ Ostrov z API"}).click();
    }
    const stats=page.locator(".school-admissions");
    await expect(stats).toBeVisible();
    await expect(stats.locator(".admission-numbers dd")).toHaveText(["95","30","31,6 %"]);
    await expect(stats.getByRole("heading",{name:"Míra přijetí"})).toBeVisible();
    await expect(page.getByText("Adresa z API 123",{exact:true})).toBeVisible();
    expect(requests).toBe(1);
    const position=await stats.evaluate(el=>el.getBoundingClientRect().top);
    const following=path === "/kraj" ? page.locator(".detail-actions") : page.locator(".student-route");
    expect(position).toBeLessThan(await following.evaluate(el=>el.getBoundingClientRect().top));
    if(path === "/kraj") {
      await page.getByRole("button",{name:"Odebrat obor ze simulované nabídky"}).click();
      await expect(page.locator(".simulation-result")).toBeVisible();
      await expect(stats.locator(".admission-numbers dd")).toHaveText(["95","30","31,6 %"]);
    }
  });
}
