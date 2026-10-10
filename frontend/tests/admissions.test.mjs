import test from "node:test";
import assert from "node:assert/strict";
import { admissionStats } from "../src/lib/admissions.ts";

test("admission rate uses accepted students, never capacity, and weights specializations by applications", () => {
  assert.deepEqual(admissionStats([
    { prihlasky: 40, prijati: 20, kapacita: 30 },
    { prihlasky: 160, prijati: 30, kapacita: 47 },
  ]), { applications: 200, accepted: 50, rate: 0.25 });
  assert.equal(admissionStats([{ prihlasky: 95, prijati: 30 }]).rate, 30 / 95);
});

test("zero accepted is a valid zero rate but zero applications has no rate", () => {
  assert.deepEqual(admissionStats([{ prihlasky: 10, prijati: 0 }]), { applications: 10, accepted: 0, rate: 0 });
  assert.deepEqual(admissionStats([{ prihlasky: 0, prijati: 0 }]), { applications: 0, accepted: 0, rate: null });
});

test("missing offers and partially missing accepted counts cannot become a misleading rate", () => {
  assert.equal(admissionStats([]), null);
  for (const missing of [{ prihlasky: 10, prijati: null }, { prihlasky: 10 }]) {
    assert.deepEqual(admissionStats([{ prihlasky: 100, prijati: 30 }, missing]), {
      applications: 110, accepted: null, rate: null,
    });
  }
});
