import { test, expect, type Page } from "@playwright/test";

async function openSchool(page: Page, name = "SPŠ Ostrov") {
  await page.getByRole("combobox", {name: "Škola pro změnu nabídky"}).fill(name);
  await page.getByRole("option", {name: new RegExp(name)}).click();
  await expect(page.getByRole("heading", {name: "Informace o škole"})).toBeVisible();
}

test("removal posts one school's full API capacity and keeps map, zoom and layers during loading and refresh", async ({page}) => {
  const bodies: Record<string, unknown>[] = [];
  let release!: () => void;
  let gate = new Promise<void>(resolve => {release = resolve;});
  await page.route("**/api/backend/simulace/zmeny", async route => {
    bodies.push(route.request().postDataJSON());
    await gate;
    await route.continue();
  });
  await page.goto("/kraj"); await openSchool(page);
  await page.getByRole("checkbox", {name:"Zaměstnavatelé"}).uncheck();
  await page.getByRole("button", {name:"Přiblížit mapu", exact:true}).click();
  const chart=page.locator(".map-frame .echart");
  const svg=(await chart.locator("svg").elementHandle())!;
  const detail=(await page.locator(".region-detail").elementHandle())!;
  const instance=await chart.getAttribute("_echarts_instance_");
  const paths=()=>chart.locator("svg path").evaluateAll(nodes => nodes.slice(0,839).map(node=>({d:node.getAttribute("d"),fill:node.getAttribute("fill")})));
  const before=await paths();
  await page.getByRole("button",{name:"Odebrat obor ze simulované nabídky"}).click();
  await expect(page.locator(".simulation-feedback .data-skeleton")).toBeVisible();
  await expect(page.getByRole("heading",{name:"Informace o škole"})).toBeVisible();
  expect(await svg.evaluate(node=>node.isConnected)).toBe(true);
  expect(await paths()).toEqual(before);
  release();
  await expect(page.locator(".simulation-result")).toContainText("154 → 77");
  await expect(page.locator(".simulation-result")).toContainText("Děti, které ztratí dostupnost oboru: 10");
  await expect(page.locator(".coverage-gain")).toContainText("-10 dětí");
  await expect(page.locator(".changes-summary")).toContainText("Modelová simulace z API");
  expect(bodies).toEqual([{obor:"18-20-M/01",zmeny:[{redizo:"600009084",zmena_kapacity:-77}],max_min:45,scenar:"rano",uroven:"zsj",format:"geojson"}]);
  const completed=await paths();
  expect(completed.map(p=>p.d)).toEqual(before.map(p=>p.d));
  expect(completed.filter((p,i)=>p.fill!==before[i].fill)).toHaveLength(1);
  expect(await detail.evaluate(node=>node.isConnected)).toBe(true);
  expect(await chart.getAttribute("_echarts_instance_")).toBe(instance);
  await expect(page.getByRole("checkbox",{name:"Zaměstnavatelé"})).not.toBeChecked();
  gate=new Promise<void>(resolve=>{release=resolve;});
  await page.getByRole("combobox",{name:"Hranice dostupnosti"}).selectOption("60");
  await expect(page.locator(".simulation-feedback .data-skeleton")).toBeVisible();
  expect(await paths()).toEqual(completed);
  release();
  await expect(page.locator(".simulation-result")).toContainText("Děti, které ztratí dostupnost oboru: 0");
  expect(bodies).toHaveLength(2);
  await page.getByRole("button",{name:"Zrušit scénář",exact:true}).click();
  await page.getByRole("button",{name:"Vrátit předchozí scénář",exact:true}).click();
  await openSchool(page);
  await expect(page.locator(".simulation-result")).toBeVisible();
  expect(bodies).toHaveLength(2);
});

test("removal failure retries locally; cancellation discards pending data and never fabricates a scenario", async({page})=>{
  await page.route("**/api/backend/simulace/zmeny",route=>route.fulfill({status:503,json:{}}));
  await page.goto("/kraj"); await openSchool(page);
  await page.getByRole("button",{name:"Odebrat obor ze simulované nabídky"}).click();
  await expect(page.locator(".region-detail [role=alert]")).toBeVisible();
  await expect(page.locator(".map-frame")).toBeVisible();
  await expect(page.locator(".changes-summary")).toHaveCount(0);
  await page.unroute("**/api/backend/simulace/zmeny");
  await page.getByRole("button",{name:"Zkusit znovu"}).click();
  await expect(page.locator(".simulation-result")).toBeVisible();
  let release!:()=>void;
  const gate=new Promise<void>(resolve=>{release=resolve;});
  await page.route("**/api/backend/simulace/zmeny",async route=>{await gate;await route.continue().catch(()=>{});});
  await page.getByRole("combobox",{name:"Hranice dostupnosti"}).selectOption("30");
  await expect(page.locator(".simulation-feedback .data-skeleton")).toBeVisible();
  await page.getByRole("button",{name:"Zrušit scénář",exact:true}).click();release();
  await expect(page.locator(".simulation-result")).toHaveCount(0);
  await expect(page.locator(".changes-summary")).toHaveCount(0);
  await expect(page.locator(".coverage-summary")).toContainText("2 školy");
});

test("read-only POST caches by canonical body and retains error status without caching failures",async({request})=>{
  const nonce=Date.now();const url=`/api/backend/simulace/zmeny?test_run=${nonce}`;
  const body={obor:"18-20-M/01",zmeny:[{redizo:"600009084",zmena_kapacity:-77}],max_min:45,uroven:"zsj",format:"geojson",scenar:"rano"};
  const first=await request.post(url,{data:body});expect(first.status()).toBe(200);
  const data=await first.json();expect(data.features).toHaveLength(839);expect(data.features[0]).not.toHaveProperty("geometry");
  const second=await request.post(url,{data:{...Object.fromEntries(Object.entries(body).reverse()),zmeny:[{zmena_kapacity:-77,redizo:"600009084"}]}});
  expect(await second.json()).toEqual(data);
  await request.post(url,{data:{...body,max_min:60}});
  await request.post(url,{data:{...body,zmeny:[{redizo:"600009084",zmena_kapacity:-30}]}});
  for(let i=0;i<2;i++) expect((await request.post(url,{data:{...body,zmeny:[{redizo:"600000000",zmena_kapacity:-77}]}})).status()).toBe(404);
  const counts=await(await request.get("http://127.0.0.1:8107/__counts")).json();
  const keys=Object.keys(counts).filter(key=>key.includes(`test_run=${nonce}`));
  expect(keys).toHaveLength(4);expect(keys.map(key=>counts[key]).sort()).toEqual([1,1,1,2]);
});

test("ZSJ API supplies names, representative coordinates and map polygons instead of the local list",async({page})=>{
  const staticRequests:string[]=[]; const searches:URL[]=[];
  page.on("request",request=>{
    if(request.url().includes("/data/zsj.geojson"))staticRequests.push(request.url());
    if(request.url().includes("/api/backend/student/skoly?"))searches.push(new URL(request.url()));
  });
  await page.route("**/api/backend/zsj/seznam",async route=>{
    const response=await route.fetch();const zones=await response.json();
    zones[0]={...zones[0],nazev:"Abertamy z API",lat:50.333,lon:12.777,boundary:{type:"Polygon",coordinates:[[[12.82,50.36],[12.84,50.36],[12.83,50.38],[12.82,50.36]]]}};
    await route.fulfill({json:zones});
  });
  await page.goto("/rodiny");
  await expect(page.locator(".journey-result").first()).toBeVisible();
  await page.getByRole("combobox",{name:"Odkud"}).fill("Abertamy z API");
  await page.getByRole("option",{name:/Abertamy z API/}).click();
  await page.getByRole("button",{name:"Najít školy",exact:true}).click();
  await expect.poll(()=>searches.length).toBe(2);
  expect(searches[1].searchParams.get("lat")).toBe("50.333");
  expect(searches[1].searchParams.get("lon")).toBe("12.777");
  const path=page.locator(".map-frame .echart svg path").first();
  await expect(path).toBeVisible();
  const d=await path.getAttribute("d"); expect((d?.match(/[ML]/g)??[]).length).toBeLessThanOrEqual(4);
  expect(staticRequests).toEqual([]);
});

test("ZSJ list supports retry and treats an empty result as empty, without reviving the snapshot",async({page})=>{
  await page.route("**/api/backend/zsj/seznam",route=>route.fulfill({status:503,json:{}}));
  await page.goto("/kraj");
  await expect(page.locator(".query-notice[role=alert]")).toContainText("Data se nepodařilo načíst");
  await expect(page.locator(".map-frame")).toHaveCount(0);
  await page.unroute("**/api/backend/zsj/seznam");
  await page.getByRole("button",{name:"Zkusit znovu"}).click();
  await expect(page.locator(".map-frame .echart svg path").first()).toBeVisible();
  await page.route("**/api/backend/zsj/seznam",route=>route.fulfill({json:[]}));
  await page.reload();
  await expect(page.getByText("API nevrátilo žádné základní sídelní jednotky.")).toBeVisible();
  await expect(page.locator(".map-frame")).toHaveCount(0);
});

test("ZSJ geometry is cached once on the server and stays below the fetch-cache limit",async({request})=>{
  const nonce=Date.now();const url=`/api/backend/zsj/seznam?test_run=${nonce}`;
  const first=await request.get(url);expect(first.status()).toBe(200);
  const data=await first.json();expect(data).toHaveLength(839);expect(data[0].boundary.type).toBe("Polygon");
  expect((await first.body()).byteLength).toBeLessThan(2*1024*1024);
  expect(await(await request.get(url)).json()).toEqual(data);
  const counts=await(await request.get("http://127.0.0.1:8107/__counts")).json();
  expect(counts[`/api/v1/zsj/seznam?test_run=${nonce}`]).toBe(1);
});
