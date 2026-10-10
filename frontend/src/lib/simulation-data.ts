import type { SimulationResponse } from "./api";
import type { Travel, Zone } from "./data";

/** Full GeoJSON contains unchanged areas too; never fill API results from local journeys. */
export function simulationData(response: SimulationResponse) {
  const before: Travel = {};
  const after: Travel = {};
  const nearestSchools: Record<string, string | null> = {};
  const zones: Zone[] = [];
  for (const { properties: area } of response.features) {
    nearestSchools[area.kod] = area.nejblizsi_redizo ?? null;
    before[area.kod] = area.cas_min_puvodni ?? null;
    after[area.kod] = area.cas_min ?? null;
    zones.push({
      id: area.kod, name: area.nazev, municipality: "", lon: 0, lat: 0,
      children: area.deti,
    });
  }
  return { before, after, nearestSchools, zones };
}
