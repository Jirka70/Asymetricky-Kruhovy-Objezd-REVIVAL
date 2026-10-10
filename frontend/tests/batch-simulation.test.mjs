import test from "node:test";
import assert from "node:assert/strict";
import {QueryObserver} from "@tanstack/react-query";
import {batchSimulationQuery, createQueryClient} from "../src/lib/api.ts";
const input = {obor: "18-20-M/01", max_min: 45, zmeny: [
  {redizo: "600009271", zmena_kapacity: 30},
  {redizo: "600009084", zmena_kapacity: -77},
]};

test("batch posts every signed change and caches independently of school order", async t => {
  const requests = [];
  t.mock.method(globalThis, "fetch", async (url, options) => {
    requests.push({url, ...options}); return Response.json({features: [], souhrn: {}, meta: {}});
  });
  const client = createQueryClient(); t.after(() => client.clear());
  await client.fetchQuery(batchSimulationQuery(input));
  await client.fetchQuery(batchSimulationQuery({...input, zmeny: [...input.zmeny].reverse()}));
  assert.equal(requests.length, 1);
  assert.equal(requests[0].method, "POST");
  assert.equal(requests[0].url, "/api/backend/simulace/zmeny");
  assert.deepEqual(JSON.parse(requests[0].body), {...input, zmeny: [...input.zmeny].reverse(), scenar: "rano", uroven: "zsj", format: "geojson"});
  await client.fetchQuery(batchSimulationQuery({...input, zmeny: input.zmeny.slice(1)}));
  await client.fetchQuery(batchSimulationQuery({...input, max_min: 60}));
  assert.equal(requests.length, 3);
  for (const delta of [0, 301, -301, NaN, 1.5]) assert.equal(batchSimulationQuery({...input, zmeny: [{redizo: "600009271", zmena_kapacity: delta}]}).enabled, false);
  assert.equal(batchSimulationQuery({...input, zmeny: []}).enabled, false);
});

test("batch retains parameter edits but clears changed schools, actions and cancellation", t => {
  t.mock.method(globalThis, "fetch", () => new Promise(() => {}));
  const client = createQueryClient();
  const data = {features: [], souhrn: {}, meta: {}};
  client.setQueryData(batchSimulationQuery(input).queryKey, data);
  const observer = new QueryObserver(client, batchSimulationQuery(input));
  const unsubscribe = observer.subscribe(() => {});
  t.after(() => {unsubscribe(); client.clear();});
  observer.setOptions(batchSimulationQuery({...input, max_min: 60}));
  assert.equal(observer.getCurrentResult().data, data);
  for (const change of [
    {zmeny: input.zmeny.slice(1)},
    {obor: "23-51-E/01"},
    {zmeny: input.zmeny.map(c => ({...c, zmena_kapacity: -c.zmena_kapacity}))},
    {zmeny: []},
  ]) {
    observer.setOptions(batchSimulationQuery({...input, ...change}));
    assert.equal(observer.getCurrentResult().data, undefined);
  }
  assert.equal(observer.getCurrentResult().isFetching, false);
});
