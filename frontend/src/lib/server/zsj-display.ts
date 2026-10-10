import { simplify } from "@turf/simplify";
import type { ZsjRecord } from "../api";

// Display geometry only. Keep the original representative point for API queries.
// Match the existing map's tolerance (~10 m); never simplify on each render.
export function displayZones(zones: ZsjRecord[]): ZsjRecord[] {
  return zones.map((zone) => {
    const boundary = simplify(zone.boundary, { tolerance: 0.00012, highQuality: true });
    const polygons = boundary.type === "Polygon" ? [boundary.coordinates] : boundary.coordinates;
    for (const rings of polygons) for (const ring of rings) for (const point of ring) {
      point[0] = Number(point[0].toFixed(5));
      point[1] = Number(point[1].toFixed(5));
    }
    return { ...zone, boundary };
  });
}
