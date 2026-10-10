export type PlaceKind = "school-offered" | "school-other" | "employer";
export const PLACE_KINDS: PlaceKind[] = [
  "school-offered",
  "school-other",
  "employer",
];
export const MAX_MAP_ZOOM = 32;
export type MapPlace = {
  key: string;
  id: string;
  kind: PlaceKind;
  name: string;
  city: string;
  lon: number;
  lat: number;
  selected?: boolean;
  hint?: string;
};
export type ProjectedPlace = MapPlace & { x: number; y: number };
export type PlaceGroup = {
  key: string;
  x: number;
  y: number;
  width: number;
  members: ProjectedPlace[];
  categories: { kind: PlaceKind; members: ProjectedPlace[]; width: number }[];
};

type Viewport = { width: number; height: number };

function makeGroup(members: ProjectedPlace[], viewport?: Viewport): PlaceGroup {
  const categories = PLACE_KINDS.flatMap((kind) => {
    const points = members.filter((p) => p.kind === kind);
    return points.length
      ? [{ kind, members: points, width: members.length > 1 ? 54 : 36 }]
      : [];
  });
  const width =
    categories.reduce((sum, c) => sum + c.width, 0) +
    (categories.length - 1) * 4 +
    (categories.length > 1 ? 6 : 0);
  const x = members.reduce((sum, p) => sum + p.x, 0) / members.length;
  const y = members.reduce((sum, p) => sum + p.y, 0) / members.length;
  return {
    key: members
      .map((p) => p.key)
      .sort()
      .join("|"),
    // Keep the entire control reachable at map edges; member coordinates stay
    // unchanged so zoom still targets the actual geographic location.
    x: viewport
      ? Math.max(width / 2 + 6, Math.min(viewport.width - width / 2 - 6, x))
      : x,
    y: viewport ? Math.max(27, Math.min(viewport.height - 27, y)) : y,
    width,
    members,
    categories,
  };
}

/** Merge colliding screen footprints, preserving a separate count per category.
 * Screen coordinates keep the density consistent across zoom and viewport sizes.
 */
export function clusterPlaces(
  points: ProjectedPlace[],
  viewport?: Viewport,
): PlaceGroup[] {
  const groups = points
    .filter((p) => Number.isFinite(p.x) && Number.isFinite(p.y))
    .toSorted((a, b) => a.key.localeCompare(b.key))
    .map((p) => makeGroup([p], viewport));
  // Recheck after each merge: a wider mixed group may touch a third group.
  for (let i = 0; i < groups.length; i++) {
    let merged = false;
    for (let j = i + 1; j < groups.length; j++) {
      const a = groups[i],
        b = groups[j];
      if (
        Math.abs(a.x - b.x) < (a.width + b.width) / 2 + 12 &&
        Math.abs(a.y - b.y) < 52
      ) {
        groups[i] = makeGroup([...a.members, ...b.members], viewport);
        groups.splice(j, 1);
        merged = true;
        break;
      }
    }
    if (merged) i = -1;
  }
  return groups;
}

export function clusterZoom(
  points: ProjectedPlace[],
  zoom: number,
  width: number,
  height: number,
) {
  const xs = points.map((p) => p.x),
    ys = points.map((p) => p.y);
  const minX = Math.min(...xs),
    maxX = Math.max(...xs),
    minY = Math.min(...ys),
    maxY = Math.max(...ys);
  const scale = Math.min(
    Math.max(120, width - 180) / Math.max(1, maxX - minX),
    Math.max(100, height - 180) / Math.max(1, maxY - minY),
  );
  return {
    pixelCenter: [(minX + maxX) / 2, (minY + maxY) / 2],
    zoom: Math.min(MAX_MAP_ZOOM, zoom * Math.max(1.8, Math.min(4, scale))),
  };
}
