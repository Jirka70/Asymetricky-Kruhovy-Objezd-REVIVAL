import test from "node:test";
import assert from "node:assert/strict";
import { QueryObserver } from "@tanstack/react-query";
import { createQueryClient, simulationQuery, accessibilityQuery } from "../src/lib/api.ts";

test("pending simulation retains a map only for the same school and program; cancellation clears it", (t) => {
  t.mock.method(globalThis, "fetch", () => new Promise(() => {}));
  const client = createQueryClient();
  const input = { redizo: "600009271", obor: "18-20-M/01", kapacita: 30, max_min: 45 };
  const data = { features: [], souhrn: { potencialni_uchazeci: 17 }, meta: {} };
  client.setQueryData(simulationQuery(input).queryKey, data);
  const observer = new QueryObserver(client, simulationQuery(input));
  const unsubscribe = observer.subscribe(() => {});
  t.after(() => { unsubscribe(); client.clear(); });
  for (const change of [{ kapacita: 60 }, { max_min: 60 }]) {
    observer.setOptions(simulationQuery({ ...input, ...change }));
    assert.equal(observer.getCurrentResult().isFetching, true);
    assert.equal(observer.getCurrentResult().isPlaceholderData, true);
    assert.deepEqual(observer.getCurrentResult().data, data);
  }
  for (const change of [{ redizo: "600009084" }, { obor: "23-51-E/01" }, { redizo: "" }]) {
    observer.setOptions(simulationQuery({ ...input, ...change }));
    assert.equal(observer.getCurrentResult().data, undefined);
  }
  assert.equal(observer.getCurrentResult().isFetching, false);
});

test("changing travel limit retains the base map, changing program or study form does not", (t) => {
  t.mock.method(globalThis, "fetch", () => new Promise(() => {}));
  const client = createQueryClient();
  const data = { features: [{ properties: { kod: "001261", cas_min: 80 } }] };
  client.setQueryData(accessibilityQuery("18-20-M/01", "den", 45).queryKey, data);
  const observer = new QueryObserver(client, accessibilityQuery("18-20-M/01", "den", 45));
  const unsubscribe = observer.subscribe(() => {});
  t.after(() => { unsubscribe(); client.clear(); });
  observer.setOptions(accessibilityQuery("18-20-M/01", "den", 60));
  assert.deepEqual(observer.getCurrentResult().data, data);
  assert.equal(observer.getCurrentResult().isPlaceholderData, true);
  for (const query of [accessibilityQuery("18-20-M/01", "dal", 60), accessibilityQuery("23-51-E/01", "den", 60)]) {
    observer.setOptions(query);
    assert.equal(observer.getCurrentResult().data, undefined);
  }
});
