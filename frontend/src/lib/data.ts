import type { Polygon, MultiPolygon } from "geojson";

export type Zone = {
  id: string;
  name: string;
  municipality: string;
  lon: number;
  lat: number;
  children: number | null;
  boundary?: Polygon | MultiPolygon;
};
export type School = {
  id: string;
  name: string;
  shortName: string;
  city: string;
  address: string;
  lat: number;
  lon: number;
  web: string | null;
};
export type Offering = {
  school: string;
  field: string;
  form: string;
  capacity: number | null;
  applications: number | null;
  accepted: number | null;
};
export type Employer = {
  id: string;
  name: string;
  ico: string;
  city: string;
  municipality: string;
  addresspoint: string | null;
  importedat: string | null;
  lat: number;
  lon: number;
  field: string;
  jobs: number;
  professions: {
    code: string;
    name: string;
    education: string | null;
    jobs: number;
    suitability: 1 | 2;
  }[];
};
export type Snapshot = {
  zsj: Zone[];
  municipalities: {
    id: string;
    name: string;
    children: number;
    nameDerived: boolean;
  }[];
  schools: School[];
  fields: { id: string; name: string; forms?: string[] }[];
  offerings: Offering[];
  demand: { field: string; jobs: number; employers: number }[];
  employers: Employer[];
  routes: Record<string, Record<string, [number, string, string]>>;
  meta: {
    date: string;
    arrivalWindow: string;
    demographyYear: number;
    totalJobs: number;
    totalEmployers: number;
    unlocatedWorkplaces: number;
  };
};
export type Change = { school: string; action: "add" | "remove" };
export type Travel = Record<string, number | null>;
export const DEFAULT_FIELD = "18-20-M/01";
export const TIME_COLORS = [
  "#2e7360",
  "#8cbbb0",
  "#e9cb82",
  "#c78c64",
  "#c8cdc5",
];
export const TIME_LABELS = [
  "Do 30 min",
  "30–45 min",
  "45–60 min",
  "Nad 60 min",
  "Bez spojení",
];
export const number = (n: number) =>
  new Intl.NumberFormat("cs-CZ", { maximumFractionDigits: 0 }).format(n);
export const time = (n: number | null | undefined) =>
  n == null ? "Bez spojení" : `${Math.ceil(n)} min`;
export function schoolIds(
  data: Snapshot,
  field: string,
  form: string,
  changes: Change[] = [],
) {
  const ids = new Set(
    data.offerings
      .filter((o) => o.field === field && o.form === form)
      .map((o) => o.school),
  );
  for (const change of changes) {
    if (change.action === "add") ids.add(change.school);
    else ids.delete(change.school);
  }
  return ids;
}
export function travelTimes(data: Snapshot, ids: Set<string>): Travel {
  return Object.fromEntries(
    data.zsj.map((z) => {
      const values = [...ids]
        .map((id) => data.routes[z.id]?.[id]?.[0])
        .filter((v): v is number => v !== undefined);
      return [z.id, values.length ? Math.min(...values) : null];
    }),
  );
}
export function bucket(t: number | null) {
  return t === null ? 4 : t <= 30 ? 0 : t <= 45 ? 1 : t <= 60 ? 2 : 3;
}
export function distribution(zones: Zone[], times: Travel) {
  const total = zones.reduce((a, z) => a + (z.children ?? 0), 0);
  const counts = [0, 0, 0, 0, 0];
  zones.forEach((z) => (counts[bucket(times[z.id] ?? null)] += z.children ?? 0));
  return {
    total,
    counts,
    percentages: counts.map((v) => (total ? (v / total) * 100 : 0)),
  };
}
export function within(zones: Zone[], times: Travel, threshold: number) {
  const total = zones.reduce((a, z) => a + (z.children ?? 0), 0);
  const accessible = zones.reduce(
    (a, z) =>
      a + (times[z.id] != null && times[z.id]! <= threshold ? z.children ?? 0 : 0),
    0,
  );
  return {
    total,
    accessible,
    percent: total ? (accessible / total) * 100 : null,
  };
}
export function difference(before: number | null, after: number | null) {
  if (before === null && after === null) return 0;
  if (before === null) return -Infinity;
  if (after === null) return Infinity;
  return after - before;
}
export function escapeHtml(s: string) {
  return s.replace(
    /[&<>"']/g,
    (c) =>
      ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[
        c
      ]!,
  );
}

/** Stable illustrative figures, never derived from real capacity/application counts. */
export function exampleAdmission(school: string) {
  const fixed: Record<string, number> = {
    "600009084": 40,
    "600170527": 50,
    "600170462": 60,
  };
  const rate = fixed[school] ?? 40 + (Number(school.slice(-2)) % 5) * 10;
  return { rate, accepted: rate * 0.8, applications: 80 };
}
