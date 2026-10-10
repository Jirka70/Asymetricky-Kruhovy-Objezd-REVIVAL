import { QueryClient, queryOptions } from "@tanstack/react-query";
import type { Snapshot } from "./data";
import type { Polygon, MultiPolygon } from "geojson";

export type ZsjRecord = {
  kod: string; nazev: string; lat: number; lon: number; kod_obce: string | null;
  boundary: Polygon | MultiPolygon;
};

export type Forma = "den" | "dal";
type Points<T> = {
  type: "FeatureCollection";
  features: {
    type: "Feature";
    geometry: { type: "Point"; coordinates: [number, number] };
    properties: T;
  }[];
};
export type SchoolsResponse = Points<{
  redizo: string;
  nazev: string;
  obec?: string;
  pocet_nabidek: number;
  kapacita?: number;
  prihlasky?: number;
}>;
export type ProgramsResponse = {
  data: {
    kod: string;
    nazev: string;
    pocet_skol: number;
    kapacita: number;
    prihlasky: number;
    volna_mista: number | null;
    zamestnavatelu: number | null;
  }[];
};
export type SchoolResponse = {
  redizo: string;
  nazev: string;
  adresa?: string;
  web?: string | null;
  lat: number;
  lon: number;
  nabidky: {
    kod_oboru: string;
    nazev_oboru: string;
    forma: Forma;
    kapacita: number;
    prihlasky: number;
    prijati?: number | null;
  }[];
};
export type EmployersResponse = Points<{
  id: string;
  ico: string;
  nazev: string;
  kod_obce: string;
  pocet_mist: number;
  profese: {
    cz_isco3: string;
    nazev: string;
    vhodnost: 1 | 2;
    pocet_mist: number;
  }[];
}> & {
  meta: {
    mapovani: boolean;
    existuje: boolean | null;
    importovano?: string;
    bez_souradnic?: { kod_obce: string; pracovist: number; pocet_mist: number }[];
  };
};

export class ApiError extends Error {
  status: number;
  constructor(status: number) {
    super(`Načtení dat selhalo (${status}).`);
    this.status = status;
  }
}

export function createQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: 86400 * 1000,
        gcTime: typeof window === "undefined" ? Infinity : 24 * 60 * 60 * 1000,
        retry: (count, error) =>
          count < 1 && !(error instanceof ApiError && error.status < 500),
      },
    },
  });
}

async function getJson<T>(url: string, signal: AbortSignal, init?: RequestInit): Promise<T> {
  const response = await fetch(url, { ...init, signal });
  if (!response.ok) throw new ApiError(response.status);
  return response.json();
}

export const snapshotQuery = queryOptions({
  queryKey: ["snapshot"],
  queryFn: ({ signal }) => getJson<Snapshot>("/data/snapshot.json", signal),
  staleTime: Infinity,
});

export const zsjListQuery = queryOptions({
  queryKey: ["zsj-seznam"],
  queryFn: ({ signal }) => getJson<ZsjRecord[]>("/api/backend/zsj/seznam", signal),
});

export type RemovalResponse = Omit<SimulationResponse, "souhrn"> & {
  souhrn: {
    zlepsenych_jednotek: number; zhorsenych_jednotek: number;
    deti_v_dosahu_pred: number; deti_v_dosahu_po: number;
    ztracene_deti: number; nove_dosazene_deti: number;
    kapacita_pred: number; kapacita_po: number;
  };
};

export function removalQuery(input: SimulationParams) {
  return queryOptions({
    queryKey: ["simulace-odebrani", input] as const,
    queryFn: ({ signal }) => getJson<RemovalResponse>("/api/backend/simulace/zmeny", signal, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        obor: input.obor,
        zmeny: [{ redizo: input.redizo, zmena_kapacity: -input.kapacita }],
        max_min: input.max_min, scenar: "rano", uroven: "zsj", format: "geojson",
      }),
    }),
    placeholderData: (previous, query) =>
      query?.queryKey[1].redizo === input.redizo && query.queryKey[1].obor === input.obor
        ? previous : undefined,
    enabled: Boolean(input.redizo && input.obor) && Number.isInteger(input.kapacita) && input.kapacita >= 1 && input.kapacita <= 300,
  });
}

export function schoolsQuery(obor?: string, forma: string = "den") {
  const params = new URLSearchParams({ forma });
  if (obor) params.set("obor", obor);
  return queryOptions({
    queryKey: ["skoly", { obor: obor ?? null, forma }],
    queryFn: ({ signal }) =>
      getJson<SchoolsResponse>(`/api/backend/skoly?${params}`, signal),
  });
}

export function programsQuery(forma: Forma) {
  return queryOptions({
    queryKey: ["obory", forma],
    queryFn: ({ signal }) =>
      getJson<ProgramsResponse>(`/api/backend/obory?forma=${forma}`, signal),
  });
}

export function schoolQuery(redizo: string) {
  return queryOptions({
    queryKey: ["skola", redizo],
    queryFn: ({ signal }) =>
      getJson<SchoolResponse>(`/api/backend/skoly/${encodeURIComponent(redizo)}`, signal),
    enabled: Boolean(redizo),
  });
}

export function employersQuery(obor: string) {
  return queryOptions({
    queryKey: ["zamestnavatele", obor],
    queryFn: ({ signal }) =>
      getJson<EmployersResponse>(
        `/api/backend/obory/${encodeURIComponent(obor)}/zamestnavatele?jen_ss=true&vhodnost=2`,
        signal,
      ),
    enabled: Boolean(obor),
  });
}

export type SimulationParams = {
  redizo: string;
  obor: string;
  kapacita: number;
  max_min: number;
};
export type SimulationResponse = {
  type: "FeatureCollection";
  features: {
    properties: {
      kod: string;
      nazev: string;
      cas_min_puvodni?: number | null;
      cas_min?: number | null;
      nejblizsi_redizo?: string | null;
      deti: number;
    };
  }[];
  souhrn: {
    zlepsenych_jednotek?: number;
    potencialni_uchazeci?: number;
    odlehceni?: number;
    pretazeni?: number;
    novi_v_dosahu?: number;
    vyuziti?: number;
    verdikt?: "dobre_misto" | "spatne_misto" | "neutralni";
  } | null;
  meta: { duvod?: string };
};

export function simulationQuery(input: SimulationParams) {
  const params = new URLSearchParams({
    redizo: input.redizo,
    obor: input.obor,
    kapacita: String(input.kapacita),
    max_min: String(input.max_min),
    uroven: "zsj",
    format: "geojson",
    scenar: "rano",
  });
  return queryOptions({
    queryKey: ["simulace", { ...input, uroven: "zsj", format: "geojson", scenar: "rano" }] as const,
    queryFn: ({ signal }) => getJson<SimulationResponse>(`/api/backend/simulace?${params}`, signal),
    // Keep the last map while recalculating capacity/limit for this offering.
    // A different school or program must never inherit its result.
    placeholderData: (previous, query) =>
      query?.queryKey[1].redizo === input.redizo && query.queryKey[1].obor === input.obor
        ? previous : undefined,
    enabled: Boolean(input.redizo && input.obor) && Number.isInteger(input.kapacita) && input.kapacita >= 1 && input.kapacita <= 300,
  });
}

export type AccessibilityResponse = {
  features: { properties: {
    kod: string; nazev: string; cas_min?: number; deti: number;
    v_dosahu: boolean; deti_v_dosahu?: number; pasmo: string;
    nejblizsi_redizo?: string | null;
  } }[];
};
export type ConnectionSummary = {
  stav: "ok" | "bez_spojeni" | "data_nedostupna";
  cas_min?: number; odjezd?: string; prijezd?: string;
  prestupy?: number; chuze_m?: number; linky?: string[]; vzdalenost_m?: number;
};
export type StudentRoute = {
  type: "FeatureCollection";
  features: {
    type: "Feature";
    geometry: { type: "LineString"; coordinates: [number, number][] };
    properties: {
      spoj: number; usek: number; druh: "WALK" | "BUS" | "RAIL" | "TRAM" | "TROLLEYBUS";
      linka?: string; od: string; do: string; odjezd: string; prijezd: string; chuze_m?: number;
    };
  }[];
  spoje: { spoj: number; odjezd: string; prijezd: string; cas_min: number; prestupy: number;
    chuze_m?: number; rezerva_min?: number; vzdalenost_m?: number }[];
  meta: { den?: string; scenar?: string; okno_prijezdu?: string[] };
};
export function studentRouteQuery(input: { lat: number; lon: number; redizo: string }) {
  const params = new URLSearchParams(Object.entries({ ...input, scenar: "rano" }).map(([key, value]) => [key, String(value)]));
  return queryOptions({
    queryKey: ["student-trasa", { ...input, scenar: "rano" }],
    queryFn: ({ signal }) => getJson<StudentRoute>(`/api/backend/student/trasa?${params}`, signal),
    enabled: Boolean(input.redizo) && Number.isFinite(input.lat) && Number.isFinite(input.lon),
  });
}
export type StudentSchool = {
  redizo: string; nazev: string; lat: number; lon: number; v_dosahu: boolean;
  spoj: ConnectionSummary;
  nabidky: SchoolResponse["nabidky"];
};
export type StudentSchoolsResponse = {
  data: StudentSchool[];
  meta: { zsj?: string; max_min: number; presnost?: string; den?: string };
};
export type ProgramResponse = {
  obor: { kod: string; nazev: string; pocet_skol: number; kapacita: number; prihlasky: number;
    prihlasky_na_misto: number | null; deti_v_dosahu: number; deti_bez_oboru: number;
    podil_deti_v_dosahu: number | null };
  nabidky: (SchoolResponse["nabidky"][number] & { redizo?: string; nazev_skoly?: string; deti_v_dosahu_skoly?: number })[];
  kandidati: { redizo: string; nazev: string; nove_dosazene_deti: number; ma_pribuzny_obor: boolean }[];
  trh_prace: { volna_mista?: number; zamestnavatelu?: number } | null;
};

export function accessibilityQuery(obor: string, forma: string, max_min: number) {
  const params = new URLSearchParams({ obor, forma, max_min: String(max_min), uroven: "zsj", scenar: "rano" });
  return queryOptions({
    queryKey: ["zsj", { obor, forma, max_min, uroven: "zsj", scenar: "rano" }] as const,
    queryFn: ({ signal }) => getJson<AccessibilityResponse>(`/api/backend/zsj?${params}`, signal),
    placeholderData: (previous, query) =>
      query?.queryKey[1].obor === obor && query.queryKey[1].forma === forma
        ? previous : undefined,
  });
}
export function programQuery(kod: string, max_min: number, kandidatu = 5) {
  const params = new URLSearchParams({ max_min: String(max_min), kandidatu: String(kandidatu), scenar: "rano" });
  return queryOptions({
    queryKey: ["obor", { kod, max_min, kandidatu, scenar: "rano" }],
    queryFn: ({ signal }) => getJson<ProgramResponse>(`/api/backend/obory/${encodeURIComponent(kod)}?${params}`, signal),
    enabled: Boolean(kod),
  });
}
export function studentSchoolsQuery(input: { lat: number; lon: number; obor: string; forma: string; max_min: number }) {
  const params = new URLSearchParams(Object.entries({ ...input, scenar: "rano" }).map(([key, value]) => [key, String(value)]));
  return queryOptions({
    queryKey: ["student-skoly", { ...input, scenar: "rano" }],
    queryFn: ({ signal }) => getJson<StudentSchoolsResponse>(`/api/backend/student/skoly?${params}`, signal),
    enabled: Number.isFinite(input.lat) && Number.isFinite(input.lon),
  });
}
