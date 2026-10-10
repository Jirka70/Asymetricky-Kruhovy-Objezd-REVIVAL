import test from "node:test";
import assert from "node:assert/strict";
import {
  clusterPlaces,
  clusterZoom,
  MAX_MAP_ZOOM,
} from "../src/lib/map-clusters.ts";
const point = (key, kind, x, y) => ({
  key,
  id: key,
  name: key,
  city: "",
  lon: x / 100,
  lat: y / 100,
  kind,
  x,
  y,
});

test("nearby institutions retain independent counts for all three categories", () => {
  const points = [
    point("a", "school-offered", 100, 100),
    point("b", "school-other", 103, 103),
    point("c", "school-other", 105, 100),
    point("d", "employer", 110, 108),
  ];
  const groups = clusterPlaces(points);
  assert.equal(groups.length, 1);
  assert.deepEqual(
    groups[0].categories.map((c) => [c.kind, c.members.length]),
    [
      ["school-offered", 1],
      ["school-other", 2],
      ["employer", 1],
    ],
  );
  assert.deepEqual(clusterPlaces([...points].reverse()), groups);
});
test("zooming separates distinct coordinates, while identical coordinates remain selectable together", () => {
  const nearby = [
    point("a", "employer", 100, 100),
    point("b", "employer", 120, 100),
  ];
  assert.equal(clusterPlaces(nearby).length, 1);
  assert.equal(
    clusterPlaces(nearby.map((p) => ({ ...p, x: p.x * 4, y: p.y * 4 }))).length,
    2,
  );
  const identical = [
    point("a", "school-other", 10, 10),
    point("b", "school-other", 10, 10),
  ];
  assert.equal(clusterPlaces(identical)[0].members.length, 2);
  const zoom = clusterZoom(identical, MAX_MAP_ZOOM, 900, 450);
  assert.equal(zoom.zoom, MAX_MAP_ZOOM);
  assert.ok(zoom.pixelCenter.every(Number.isFinite));
});
test("dense mixed marker groups never overlap and every institution is counted once", () => {
  const points = Array.from({ length: 180 }, (_, i) =>
    point(
      String(i),
      ["school-offered", "school-other", "employer"][i % 3],
      (i * 137) % 980,
      (i * 73) % 410,
    ),
  );
  const groups = clusterPlaces(points);
  assert.equal(
    new Set(groups.flatMap((g) => g.members.map((p) => p.key))).size,
    points.length,
  );
  assert.equal(
    groups.reduce(
      (sum, g) =>
        sum + g.categories.reduce((count, c) => count + c.members.length, 0),
      0,
    ),
    points.length,
  );
  for (let i = 0; i < groups.length; i++)
    for (let j = i + 1; j < groups.length; j++) {
      assert.ok(
        Math.abs(groups[i].x - groups[j].x) >=
          (groups[i].width + groups[j].width) / 2 + 12 ||
          Math.abs(groups[i].y - groups[j].y) >= 52,
      );
    }
});
test("cluster zoom is bounded, advances and centers on all its members", () => {
  const points = [
    point("a", "employer", 200, 100),
    point("b", "employer", 240, 130),
  ];
  const target = clusterZoom(points, 2, 900, 450);
  assert.deepEqual(target.pixelCenter, [220, 115]);
  assert.ok(target.zoom > 2 && target.zoom <= MAX_MAP_ZOOM);
});

test("edge groups stay fully visible on mobile without losing locations or overlapping", () => {
  const points = Array.from({ length: 30 }, (_, i) =>
    point(
      String(i),
      ["school-offered", "school-other", "employer"][i % 3],
      i < 15 ? i * 3 : 365 - (i - 15) * 3,
      (i * 37) % 450,
    ),
  );
  const groups = clusterPlaces(points, { width: 366, height: 450 });
  assert.equal(groups.flatMap((g) => g.members).length, points.length);
  for (const group of groups) {
    assert.ok(group.x - group.width / 2 >= 6);
    assert.ok(group.x + group.width / 2 <= 360);
    assert.ok(group.y >= 27 && group.y <= 423);
    for (const member of group.members)
      assert.deepEqual(
        member,
        points.find((p) => p.key === member.key),
      );
  }
  for (let i = 0; i < groups.length; i++)
    for (let j = i + 1; j < groups.length; j++)
      assert.ok(
        Math.abs(groups[i].x - groups[j].x) >=
          (groups[i].width + groups[j].width) / 2 + 12 ||
          Math.abs(groups[i].y - groups[j].y) >= 52,
      );
});
