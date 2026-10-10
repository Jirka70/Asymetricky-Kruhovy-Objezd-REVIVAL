import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  schoolIds,
  travelTimes,
  within,
  distribution,
  difference,
  bucket,
  exampleAdmission,
} from "../src/lib/data.ts";
import { findJourneys, exampleLegs, minutes } from "../src/lib/journeys.ts";
const snapshot = JSON.parse(
  readFileSync(
    new URL("../public/data/snapshot.json", import.meta.url),
    "utf8",
  ),
);

test("snapshot covers every ZSJ and school; geometry identifiers match", () => {
  assert.equal(snapshot.zsj.length, 839);
  assert.equal(snapshot.schools.length, 34);
  assert.equal(snapshot.municipalities.length, 134);
  assert.equal(
    snapshot.zsj.reduce((a, z) => a + z.children, 0),
    15837,
  );
  for (const [file, items] of [
    ["zsj.geojson", snapshot.zsj],
    ["municipalities.geojson", snapshot.municipalities],
  ]) {
    const geo = JSON.parse(
      readFileSync(new URL("../public/data/" + file, import.meta.url), "utf8"),
    );
    assert.deepEqual(
      new Set(geo.features.map((f) => f.properties.name)),
      new Set(items.map((i) => i.id)),
    );
  }
});
test("adding a field cannot worsen a trip; removing cannot improve it", () => {
  const ids = schoolIds(snapshot, "18-20-M/01", "den");
  const before = travelTimes(snapshot, ids);
  const added = travelTimes(
    snapshot,
    schoolIds(snapshot, "18-20-M/01", "den", [
      { school: "600009271", action: "add" },
    ]),
  );
  const removed = travelTimes(
    snapshot,
    schoolIds(snapshot, "18-20-M/01", "den", [
      { school: "600009084", action: "remove" },
    ]),
  );
  for (const z of snapshot.zsj) {
    assert.ok((added[z.id] ?? Infinity) <= (before[z.id] ?? Infinity));
    assert.ok((removed[z.id] ?? Infinity) >= (before[z.id] ?? Infinity));
  }
  assert.equal(added["167941"], 67.38);
  assert.equal(before["167941"], 83.03);
});
test("empty school set is no connection, never zero-minute access", () => {
  const times = travelTimes(snapshot, new Set());
  assert.ok(Object.values(times).every((t) => t === null));
  assert.equal(within(snapshot.zsj, times, 45).percent, 0);
});
test("coverage weights children, not polygons, and preserves missing-population state", () => {
  const zones = [
    { id: "a", children: 100 },
    { id: "b", children: 5 },
  ];
  assert.equal(within(zones, { a: 20, b: 80 }, 45).percent, (100 / 105) * 100);
  assert.equal(within([{ id: "z", children: 0 }], { z: 20 }, 45).percent, null);
  assert.deepEqual(
    distribution(zones, { a: 20, b: null }).counts,
    [100, 0, 0, 0, 5],
  );
  assert.equal(bucket(30), 0);
  assert.equal(bucket(45), 1);
  assert.equal(bucket(60), 2);
  assert.equal(bucket(null), 4);
});
test("reachability changes distinguish new and lost connections", () => {
  assert.equal(difference(null, 30), -Infinity);
  assert.equal(difference(30, null), Infinity);
  assert.equal(difference(null, null), 0);
});
test("arrival and departure filters use exact stored times including seconds", () => {
  const ids = schoolIds(snapshot, "18-20-M/01", "den");
  const all = findJourneys(snapshot, "063550", ids, "arrival", "07:50");
  assert.equal(all.length, 3);
  assert.equal(
    findJourneys(snapshot, "063550", ids, "arrival", "07:43").length,
    1,
  );
  assert.equal(
    findJourneys(snapshot, "063550", ids, "departure", "07:09").length,
    1,
  );
  assert.equal(
    findJourneys(snapshot, "063550", ids, "arrival", "06:00").length,
    0,
  );
  for (const j of all) {
    const legs = exampleLegs(j);
    assert.equal(legs[0].start, minutes(j.departure));
    assert.equal(legs.at(-1).end, minutes(j.arrival));
    for (let i = 1; i < legs.length; i++)
      assert.equal(legs[i].start, legs[i - 1].end);
  }
});
test("illustrative admissions are stable across both pages and separate from actual counts", () => {
  assert.deepEqual(exampleAdmission("600009084"), {
    rate: 40,
    accepted: 32,
    applications: 80,
  });
  assert.ok(snapshot.offerings.every((o) => o.accepted === null));
});

test("employer details retain exact bigint identities and profession-level suitability", () => {
  const keys = new Set();
  for (const employer of snapshot.employers) {
    assert.equal(typeof employer.id, "string");
    assert.match(employer.id, /^\d+$/);
    const key = `${employer.id}:${employer.field}`;
    assert.ok(!keys.has(key), `duplicate workplace/field ${key}`);
    keys.add(key);
    assert.equal(employer.jobs, employer.professions.reduce((sum, p) => sum + p.jobs, 0));
    assert.ok(employer.professions.every(p => p.suitability === 1 || p.suitability === 2));
    assert.ok(Number.isFinite(Date.parse(employer.importedat)));
  }
  const witte = snapshot.employers.find(e => e.ico === "02183765" && e.field === "18-20-M/01");
  assert.equal(witte.id, "694625177794571187");
  assert.equal(witte.jobs, 8);
  assert.equal(witte.professions.find(p => p.code === "351").suitability, 1);
  assert.equal(witte.professions.find(p => p.code === "351").jobs, 2);
  assert.equal(witte.professions.find(p => p.code === "311").suitability, 2);
});

test("unlimited coverage accepts all known times without making unknown journeys reachable", () => {
  const zones = [{id: "a", children: 10}, {id: "b", children: 20}, {id: "c", children: 30}];
  assert.equal(within(zones, {a: 240, b: null}, 0).accessible, 10);
  assert.equal(within(zones, {a: 240, b: null}, 180).accessible, 0);
});
