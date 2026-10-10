import test from "node:test";
import assert from "node:assert/strict";
import { simulationData } from "../src/lib/simulation-data.ts";

test("simulation map uses API times and annual cohort, preserving null and fractional values", () => {
  const result = simulationData({features: [
    {properties: {kod: "001261", nazev: "Bečov", cas_min_puvodni: null, cas_min: 12.75, deti: 9.6}},
    {properties: {kod: "000019", nazev: "Abertamy", cas_min_puvodni: 80.25, cas_min: null, deti: 0}},
  ]});
  assert.deepEqual(result.before, {"001261": null, "000019": 80.25});
  assert.deepEqual(result.after, {"001261": 12.75, "000019": null});
  assert.equal(result.zones[0].children, 9.6);
  assert.equal(result.after["missing"], undefined);
});
