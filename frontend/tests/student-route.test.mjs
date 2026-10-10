import test from "node:test";
import assert from "node:assert/strict";
import { createQueryClient, studentRouteQuery } from "../src/lib/api.ts";
import { proxyApi } from "../src/lib/server/api-proxy.ts";

test("route queries cache exact origins and schools independently and forward cancellation", async (t) => {
  const client = createQueryClient();
  t.after(() => client.clear());
  const urls = [];
  t.mock.method(globalThis, "fetch", async (url, options) => {
    assert.ok(options.signal instanceof AbortSignal);
    urls.push(new URL(url, "http://frontend"));
    return Response.json({features: [], spoje: [], meta: {}});
  });
  const input = {lat: 50.2, lon: 12.8, redizo: "600009084"};
  const query = studentRouteQuery(input);
  await Promise.all([client.fetchQuery(query), client.fetchQuery(query)]);
  await client.fetchQuery(query);
  assert.equal(urls.length, 1);
  for (const change of [{lat: 50.2001}, {lon: 12.8001}, {redizo: "600170527"}]) {
    await client.fetchQuery(studentRouteQuery({...input, ...change}));
  }
  assert.equal(urls.length, 4);
  assert.equal(urls[0].searchParams.get("scenar"), "rano");
  assert.equal(urls[0].searchParams.get("lat"), "50.2");
  assert.equal(urls[0].searchParams.get("redizo"), input.redizo);
  assert.equal(studentRouteQuery({...input, lat: NaN}).enabled, false);
  assert.equal(studentRouteQuery({...input, redizo: ""}).enabled, false);
});

test("route proxy allows the endpoint, preserves geometry and forwards OTP failures", async () => {
  const request = new Request("http://frontend?lat=50.2&lon=12.8&redizo=600009084&scenar=rano");
  const body = {features: [{geometry: {type: "LineString", coordinates: [[12.8, 50.2], [12.9, 50.3]]}}]};
  const result = await proxyApi(request, ["student", "trasa"], "http://backend", async (url, options) => {
    assert.equal(url.pathname, "/api/v1/student/trasa");
    assert.equal(url.searchParams.get("redizo"), "600009084");
    assert.equal(options.next.revalidate, 86400);
    return Response.json(body);
  });
  assert.equal(result.status, 200);
  assert.deepEqual(await result.json(), body);
  const error = await proxyApi(request, ["student", "trasa"], "http://backend", async () => Response.json({error: {kod: "otp_nedostupne"}}, {status: 502}));
  assert.equal(error.status, 502);
});
