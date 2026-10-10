import test from "node:test";
import assert from "node:assert/strict";
import { QueryObserver } from "@tanstack/react-query";
import { readFileSync } from "node:fs";
import { createQueryClient, removalQuery, zsjListQuery, ApiError } from "../src/lib/api.ts";
import { catalogData } from "../src/lib/api-data.ts";
import { simulationData } from "../src/lib/simulation-data.ts";
import { displayZones } from "../src/lib/server/zsj-display.ts";
import { proxyApi } from "../src/lib/server/api-proxy.ts";
import { schools, daily, distance, zsjList, removal } from "./fixtures/api.mjs";
const snapshot = JSON.parse(readFileSync(new URL("../public/data/snapshot.json", import.meta.url)));
const input = {redizo: "600009084", obor: "18-20-M/01", kapacita: 77, max_min: 45};

test("read-only POST uses the whole offering capacity, deduplicates and caches all inputs", async t => {
  const client = createQueryClient(); t.after(() => client.clear());
  const requests = [];
  t.mock.method(globalThis, "fetch", async (url, options) => {
    requests.push({url, ...options}); return Response.json(removal());
  });
  const options = removalQuery(input);
  await Promise.all([client.fetchQuery(options), client.fetchQuery(options)]);
  await client.fetchQuery(options);
  assert.equal(requests.length, 1);
  assert.equal(requests[0].url, "/api/backend/simulace/zmeny");
  assert.equal(requests[0].method, "POST");
  assert.deepEqual(JSON.parse(requests[0].body), {obor: input.obor, zmeny: [{redizo: input.redizo, zmena_kapacity: -77}], max_min: 45, scenar: "rano", uroven: "zsj", format: "geojson"});
  for (const change of [{redizo: "600170527"}, {obor: "23-51-E/01"}, {kapacita: 90}, {max_min: 60}]) await client.fetchQuery(removalQuery({...input, ...change}));
  assert.equal(requests.length, 5);
  for (const kapacita of [NaN, 0, -30, 301, 1.5]) assert.equal(removalQuery({...input, kapacita}).enabled, false);
  assert.equal(removalQuery({...input, redizo: ""}).enabled, false);
});

test("removal retains its completed map only for the same offering and aborts when cancelled", async t => {
  let signal;
  t.mock.method(globalThis, "fetch", (_url, options) => {
    signal = options.signal;
    return new Promise((_, reject) => signal.addEventListener("abort", () => reject(signal.reason)));
  });
  const client = createQueryClient();
  const data = removal(); client.setQueryData(removalQuery(input).queryKey, data);
  const observer = new QueryObserver(client, removalQuery(input));
  const unsubscribe = observer.subscribe(() => {});
  t.after(() => {unsubscribe(); client.clear();});
  observer.setOptions(removalQuery({...input, max_min: 60}));
  assert.equal(observer.getCurrentResult().isPlaceholderData, true);
  assert.deepEqual(observer.getCurrentResult().data, data);
  const previousSignal = signal;
  observer.setOptions(removalQuery({...input, redizo: ""}));
  assert.equal(previousSignal.aborted, true);
  assert.equal(observer.getCurrentResult().data, undefined);
  observer.setOptions(removalQuery({...input, redizo: "600170527"}));
  assert.equal(observer.getCurrentResult().data, undefined);
});

test("removed connection normalizes omitted times to null, never local journeys", () => {
  const data = simulationData(removal());
  assert.equal(data.before["000019"], 35);
  assert.equal(data.after["000019"], null);
  assert.equal(data.after["001261"], 80);
});

test("ZSJ catalog owns membership, names, coordinates and geometry; only demography is joined by code", () => {
  const known = {...zsjList[0], nazev: "Název z API", lat: 50.222, lon: 12.555, kod_obce: "123456"};
  const added = {...zsjList[1], kod: "999999", kod_obce: null};
  const data = catalogData(snapshot, schools, daily, distance, [added, known]);
  assert.deepEqual(data.zsj.map(z => z.id), ["999999", known.kod]);
  assert.equal(data.zsj[1].name, known.nazev);
  assert.equal(data.zsj[1].lat, known.lat);
  assert.equal(data.zsj[1].municipality, "123456");
  assert.equal(data.zsj[1].boundary, known.boundary);
  assert.equal(data.zsj[1].children, snapshot.zsj[0].children);
  assert.equal(data.zsj[0].children, null);
  assert.equal(data.zsj[0].municipality, "");
  assert.deepEqual(catalogData(snapshot, schools, daily, distance, []).zsj, []);
});

test("display simplification retains polygon holes, multipolygons, metadata and closes every ring", () => {
  const ring = [[12,50],[12.00001,50],[12.01,50],[12.01,50.01],[12,50.01],[12,50]];
  const hole = [[12.002,50.002],[12.004,50.002],[12.004,50.004],[12.002,50.002]];
  const source = [{...zsjList[0], boundary:{type:"Polygon",coordinates:[ring,hole]}}, {...zsjList[1], boundary:{type:"MultiPolygon",coordinates:[[ring],[ring]]}}];
  const before = structuredClone(source);
  const result = displayZones(source);
  assert.deepEqual(source, before);
  assert.ok(result[0].boundary.coordinates[0].length < ring.length);
  assert.equal(result[0].boundary.coordinates.length, 2);
  assert.equal(result[1].boundary.coordinates.length, 2);
  for (const zone of result) {
    const polygons = zone.boundary.type === "Polygon" ? [zone.boundary.coordinates] : zone.boundary.coordinates;
    for (const polygon of polygons) for (const ring of polygon) {
      assert.ok(ring.length >= 4); assert.deepEqual(ring[0], ring.at(-1));
    }
  }
  assert.equal(result[0].lat, source[0].lat);
  assert.equal(result[0].kod, source[0].kod);
});

test("ZSJ query deduplicates and caches, propagates errors and aborts", async t => {
  const client = createQueryClient(); t.after(() => client.clear());
  let requests = 0;
  t.mock.method(globalThis, "fetch", async url => {requests++; assert.equal(url, "/api/backend/zsj/seznam"); return Response.json([zsjList[0]]);});
  await Promise.all([client.fetchQuery(zsjListQuery), client.fetchQuery(zsjListQuery)]);
  await client.fetchQuery(zsjListQuery); assert.equal(requests, 1);
  client.removeQueries(zsjListQuery);
  t.mock.method(globalThis, "fetch", async () => Response.json({}, {status: 503}));
  await assert.rejects(client.fetchQuery({...zsjListQuery, retry:false}), error => error instanceof ApiError && error.status===503);
  let signal;
  t.mock.method(globalThis, "fetch", (_, options) => {
    signal=options.signal; return new Promise((_,reject) => signal.addEventListener("abort",()=>reject(signal.reason)));
  });
  const pending=client.fetchQuery(zsjListQuery).catch(() => {});
  await client.cancelQueries(zsjListQuery); await pending; assert.equal(signal.aborted,true);
});

test("proxy allows only the simulation POST, canonicalizes its body and forwards domain errors", async () => {
  let captured;
  const fetcher=async(url,options)=>{captured={url,options};return Response.json({error:{kod:"zaporná_kapacita"}},{status:422});};
  const body={zmeny:[{zmena_kapacity:-77,redizo:input.redizo}],obor:input.obor};
  const req=new Request("http://frontend/api/backend/simulace/zmeny",{method:"POST",body:JSON.stringify(body)});
  const res=await proxyApi(req,["simulace","zmeny"],"http://backend",fetcher);
  assert.equal(res.status,422); assert.equal(captured.url.pathname,"/api/v1/simulace/zmeny");
  assert.equal(captured.options.method,"POST"); assert.equal(captured.options.next.revalidate,86400);
  assert.equal(captured.options.body,JSON.stringify({obor:input.obor,zmeny:[{redizo:input.redizo,zmena_kapacity:-77}]}));
  assert.equal((await proxyApi(new Request("http://frontend",{method:"POST",body:"no-json"}),["simulace","zmeny"],"http://backend",fetcher)).status,400);
  assert.equal((await proxyApi(new Request("http://frontend",{method:"POST",body:"{}"}),["skoly"],"http://backend",fetcher)).status,404);
  assert.equal((await proxyApi(new Request("http://frontend"),["simulace","zmeny"],"http://backend",fetcher)).status,404);
  assert.equal((await proxyApi(new Request("http://frontend"),["zsj","seznam"],"http://backend",async()=>Response.json([]))).status,200);
});
