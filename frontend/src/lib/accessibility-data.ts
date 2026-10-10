import type { AccessibilityResponse } from "./api";
import type { Travel, Zone } from "./data";

export function accessibilityData(response: AccessibilityResponse) {
  const times: Travel = {};
  const zones: Zone[] = [];
  let total = 0, accessible = 0;
  for (const { properties: area } of response.features) {
    times[area.kod] = area.cas_min ?? null;
    zones.push({id: area.kod, name: area.nazev, municipality: "", lon: 0, lat: 0, children: area.deti});
    total += area.deti;
    // API decides reachability before rounding the displayed travel time.
    accessible += area.v_dosahu ? area.deti : 0;
  }
  return { times, zones, coverage: { total, accessible, percent: total ? accessible / total * 100 : null } };
}
