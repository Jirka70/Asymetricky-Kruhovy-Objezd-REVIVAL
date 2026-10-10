import test from "node:test";
import assert from "node:assert/strict";
import { createQueryClient, accessibilityQuery, programQuery, studentSchoolsQuery } from "../src/lib/api.ts";
import { accessibilityData } from "../src/lib/accessibility-data.ts";
import { proxyApi } from "../src/lib/server/api-proxy.ts";

test("accessibility uses API cohort and reachability before rounding; missing time is unknown", () => {
  const result = accessibilityData({features: [
    {properties: {kod: "a", nazev: "A", cas_min: 45, deti: 10, v_dosahu: false}},
    {properties: {kod: "b", nazev: "B", cas_min: 0, deti: 20, v_dosahu: true}},
    {properties: {kod: "c", nazev: "C", deti: 10, v_dosahu: false}},
  ]});
  assert.deepEqual(result.times, {a: 45, b: 0, c: null});
  assert.deepEqual(result.coverage, {total: 40, accessible: 20, percent: 50});
  assert.equal(result.zones[0].children, 10);
});

test("analytical queries deduplicate and separate every effective filter", async (t) => {
  const client = createQueryClient();
  t.after(() => client.clear());
  const urls = [];
  t.mock.method(globalThis, "fetch", async url => {urls.push(new URL(url, "http://frontend")); return Response.json({});});
  const student = {lat: 50.2, lon: 12.8, obor: "18-20-M/01", forma: "den", max_min: 45};
  const options = [accessibilityQuery(student.obor, "den", 45), programQuery(student.obor, 45), studentSchoolsQuery(student)];
  for (const query of options) {
    await Promise.all([client.fetchQuery(query), client.fetchQuery(query)]);
    await client.fetchQuery(query);
  }
  assert.equal(urls.length, 3);
  assert.equal(urls[0].searchParams.get("uroven"), "zsj");
  assert.ok(urls[1].pathname.includes("18-20-M%2F01"));
  for (const query of [accessibilityQuery(student.obor, "dal", 45), accessibilityQuery(student.obor, "den", 60), accessibilityQuery("23-51-E/01", "den", 45), programQuery(student.obor, 60), programQuery(student.obor, 45, 10), programQuery("23-51-E/01", 45)]) await client.fetchQuery(query);
  for (const input of [{lat: 50.3}, {lon: 12.9}, {obor: "23-51-E/01"}, {forma: "dal"}, {max_min: 60}]) await client.fetchQuery(studentSchoolsQuery({...student, ...input}));
  assert.equal(urls.length, 14);
  assert.equal(studentSchoolsQuery({...student, lat: NaN}).enabled, false);
});

test("proxy permits only implemented analytical routes and forwards parameters", async () => {
  for (const path of [["zsj"], ["obory", "18-20-M/01"], ["student", "skoly"]]) {
    let upstream;
    const result = await proxyApi(new Request("http://frontend?max_min=45&forma=den"), path, "http://backend", async (url, options) => {
      upstream = url;
      assert.equal(options.next.revalidate, 86400);
      return Response.json({ok: true});
    });
    assert.equal(result.status, 200);
    assert.equal(upstream.searchParams.get("max_min"), "45");
    assert.equal(upstream.pathname, `/api/v1/${path.map(encodeURIComponent).join("/")}`);
  }
  assert.equal((await proxyApi(new Request("http://frontend"), ["student", "trasa"])).status, 404);
});
