import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { QueryObserver } from "@tanstack/react-query";
import { createQueryClient, schoolsQuery, employersQuery, schoolQuery, programsQuery, simulationQuery, ApiError } from "../src/lib/api.ts";
import { catalogData, selectionData } from "../src/lib/api-data.ts";
import { proxyApi } from "../src/lib/server/api-proxy.ts";
import { schoolIds, travelTimes } from "../src/lib/data.ts";
import { schools, daily, distance, employers, points, responseFor, simulation } from "./fixtures/api.mjs";

const snapshot = JSON.parse(readFileSync(new URL("../public/data/snapshot.json", import.meta.url)));

test("API catalog replaces business data, unions both study forms and preserves the local matrix", () => {
  const data = catalogData(snapshot, schools, daily, distance);
  assert.equal(data.schools.length, 3);
  assert.equal(data.schools[0].name, "SPŠ Ostrov z API");
  assert.equal(data.schools[0].shortName, "SPŠ Ostrov");
  assert.equal(data.routes, snapshot.routes);
  assert.equal(data.zsj, snapshot.zsj);
  assert.equal(data.offerings.length, 0);
  assert.equal(data.employers.length, 0);
  assert.deepEqual(data.fields.find((f) => f.id === "26-41-M/01").forms, ["dal"]);
  assert.ok(!data.demand.some((d) => d.field === "23-51-E/01")); // unknown, not zero
  assert.equal(data.demand.find((d) => d.field === "26-41-M/01").jobs, 0);
  assert.equal(data.demand[0].jobs, 321);
});

test("filtered API totals feed offers and local simulation; empty responses never revive snapshot offers", () => {
  const base = catalogData(snapshot, schools, daily, distance);
  const data = selectionData(base, "18-20-M/01", "den", points(schools.features.slice(0, 2)), employers);
  assert.equal(data.offerings[0].applications, 999);
  assert.equal(data.offerings[0].capacity, 77);
  assert.equal(data.offerings[0].accepted, null);
  const before = schoolIds(data, "18-20-M/01", "den");
  const after = schoolIds(data, "18-20-M/01", "den", [{ school: "600009271", action: "add" }]);
  assert.equal(before.size, 2);
  assert.equal(after.size, 3);
  assert.equal(Object.keys(travelTimes(data, after)).length, 839);
  assert.equal(selectionData(base, "18-20-M/01", "dal", points([])).offerings.length, 0);
  assert.equal(selectionData(base, "23-51-E/01", "den").offerings.length, 0);
  assert.equal(data.employers[0].id, "694625177794571187");
  assert.equal(data.employers[0].jobs, 321);
  assert.equal(data.employers[0].professions[0].education, null);
  assert.equal(data.employers[0].addresspoint, null);
});

test("queries deduplicate concurrent requests and cache each field/form separately", async (t) => {
  const client = createQueryClient();
  t.after(() => client.clear());
  const urls = [];
  t.mock.method(globalThis, "fetch", async (input) => {
    urls.push(String(input));
    return Response.json(responseFor(new URL(String(input).replace("/api/backend", "/api/v1"), "http://test")));
  });
  const options = schoolsQuery("18-20-M/01", "den");
  await Promise.all([client.fetchQuery(options), client.fetchQuery(options)]);
  await client.fetchQuery(options);
  assert.equal(urls.length, 1);
  await client.fetchQuery(schoolsQuery("18-20-M/01", "dal"));
  await client.fetchQuery(schoolsQuery("23-51-E/01", "den"));
  assert.equal(urls.length, 3);
  assert.equal(client.getQueryData(options.queryKey).features.length, 2);
  assert.equal(client.getQueryData(schoolsQuery("18-20-M/01", "dal").queryKey).features.length, 0);
  await client.fetchQuery(employersQuery("18-20-M/01"));
  await client.fetchQuery(schoolQuery("600009084"));
  await client.fetchQuery(programsQuery("den"));
  assert.ok(urls.some((url) => url.includes("18-20-M%2F01/zamestnavatele")));
});

test("new keys expose a pending state, while background refresh retains cached data", async (t) => {
  const client = createQueryClient();
  t.after(() => client.clear());
  client.setQueryData(schoolsQuery("18-20-M/01").queryKey, schools);
  let resolve;
  t.mock.method(globalThis, "fetch", () => new Promise((done) => { resolve = done; }));
  const observer = new QueryObserver(client, schoolsQuery("18-20-M/01"));
  const unsubscribe = observer.subscribe(() => {});
  t.after(() => { unsubscribe(); client.clear(); });
  const refresh = observer.refetch();
  assert.equal(observer.getCurrentResult().isFetching, true);
  assert.equal(observer.getCurrentResult().data, schools);
  assert.equal(observer.getCurrentResult().isPending, false);
  resolve(Response.json(schools));
  await refresh;
  observer.setOptions(schoolsQuery("23-51-E/01"));
  assert.equal(observer.getCurrentResult().isPending, true);
  assert.equal(observer.getCurrentResult().data, undefined);
  resolve(Response.json(points([])));
  await client.fetchQuery(schoolsQuery("23-51-E/01"));
});

test("cancelled queries abort fetch; API errors remain errors instead of empty school lists", async (t) => {
  const client = createQueryClient();
  t.after(() => client.clear());
  let signal;
  t.mock.method(globalThis, "fetch", (_url, options) => {
    signal = options.signal;
    return new Promise((_resolve, reject) => signal.addEventListener("abort", () => reject(signal.reason)));
  });
  const options = schoolsQuery("18-20-M/01");
  const pending = client.fetchQuery(options).catch((error) => error);
  await client.cancelQueries({ queryKey: options.queryKey });
  assert.equal(signal.aborted, true);
  await pending;
  t.mock.method(globalThis, "fetch", async () => Response.json({}, { status: 503 }));
  await assert.rejects(client.fetchQuery({ ...options, retry: false }), (error) => error instanceof ApiError && error.status === 503);
  assert.equal(client.getQueryData(options.queryKey), undefined);
});

test("proxy preserves encoded program paths, canonicalizes filters and sets a daily server cache", async () => {
  let captured;
  const fetcher = async (url, options) => {
    captured = { url: String(url), options };
    return new Response(JSON.stringify(employers), { headers: { "Content-Type": "application/geo+json" } });
  };
  const response = await proxyApi(new Request("http://frontend/api/backend/obory/x/zamestnavatele?vhodnost=2&jen_ss=true"),
    ["obory", "18-20-M/01", "zamestnavatele"], "http://backend:8000", fetcher);
  assert.equal(captured.url, "http://backend:8000/api/v1/obory/18-20-M%2F01/zamestnavatele?jen_ss=true&vhodnost=2");
  assert.equal(captured.options.next.revalidate, 86400);
  assert.equal(response.headers.get("Content-Type"), "application/geo+json");
  assert.equal(response.headers.get("Cache-Control"), "no-store");
  assert.deepEqual(await response.json(), employers);
});

test("proxy forwards backend errors, handles outages and excludes unfinished/live endpoints", async () => {
  const request = new Request("http://frontend/api/backend/skoly");
  const unavailable = await proxyApi(request, ["skoly"], "http://backend", async () => { throw Error("offline"); });
  assert.equal(unavailable.status, 502);
  const missing = await proxyApi(request, ["skoly", "600000000"], "http://backend", async () => Response.json({ error: { kod: "nenalezeno" } }, { status: 404 }));
  assert.equal(missing.status, 404);
  assert.equal((await missing.json()).error.kod, "nenalezeno");
  for (const path of [["student", "trasa"], ["zsj", "unsupported"], ["..", "docs"]]) {
    const result = await proxyApi(request, path, "http://backend", () => { throw Error("must not fetch"); });
    assert.equal(result.status, 404);
  }
});


test("simulation queries cache all inputs independently and request complete ZSJ results", async (t) => {
  const client = createQueryClient();
  t.after(() => client.clear());
  const urls = [];
  t.mock.method(globalThis, "fetch", async (url) => {
    urls.push(new URL(url, "http://frontend"));
    return Response.json(simulation());
  });
  const input = {redizo: "600009271", obor: "18-20-M/01", kapacita: 30, max_min: 45};
  await Promise.all([client.fetchQuery(simulationQuery(input)), client.fetchQuery(simulationQuery(input))]);
  await client.fetchQuery(simulationQuery(input));
  assert.equal(urls.length, 1);
  for (const change of [{redizo: "600009084"}, {obor: "23-51-E/01"}, {kapacita: 60}, {max_min: 60}]) {
    await client.fetchQuery(simulationQuery({...input, ...change}));
  }
  assert.equal(urls.length, 5);
  assert.equal(urls[0].searchParams.get("obor"), input.obor);
  assert.equal(urls[0].searchParams.get("format"), "geojson");
  assert.equal(urls[0].searchParams.get("uroven"), "zsj");
  assert.equal(urls[0].searchParams.get("scenar"), "rano");
  assert.equal(simulationQuery({...input, redizo: ""}).enabled, false);
  for (const kapacita of [0, 301, 1.5, NaN]) assert.equal(simulationQuery({...input, kapacita}).enabled, false);
});

test("simulation proxy caches parameterized GET for a day and preserves domain errors", async () => {
  const request = new Request("http://frontend/api/backend/simulace?redizo=600009271&kapacita=30&obor=18-20-M%2F01&max_min=45");
  let upstream;
  const result = await proxyApi(request, ["simulace"], "http://backend", async (url, options) => {
    upstream = url;
    assert.equal(options.next.revalidate, 86400);
    return Response.json({error: {kod: "bez_denni_nabidky"}}, {status: 422});
  });
  assert.equal(upstream.searchParams.get("kapacita"), "30");
  assert.equal(upstream.pathname, "/api/v1/simulace");
  assert.equal(result.status, 422);
});


test("cached data stays fresh for 86400 seconds and refreshes on the next request after expiry", async (t) => {
  const client = createQueryClient();
  t.after(() => client.clear());
  const started = Date.now();
  let now = started;
  t.mock.method(Date, "now", () => now);
  let requests = 0;
  t.mock.method(globalThis, "fetch", async () => {
    requests++;
    return Response.json(schools);
  });
  const options = schoolsQuery("18-20-M/01", "den");
  await client.fetchQuery(options);
  now = started + 86400 * 1000 - 1;
  await client.fetchQuery(options);
  assert.equal(requests, 1);
  now++;
  await client.fetchQuery(options);
  assert.equal(requests, 2);
});
