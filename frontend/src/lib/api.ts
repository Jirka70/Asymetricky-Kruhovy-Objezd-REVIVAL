import { QueryClient, queryOptions } from "@tanstack/react-query";
import type { Snapshot } from "./data";

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
        staleTime: 5 * 60 * 1000,
        gcTime: typeof window === "undefined" ? Infinity : 24 * 60 * 60 * 1000,
        retry: (count, error) =>
          count < 1 && !(error instanceof ApiError && error.status < 500),
      },
    },
  });
}

async function getJson<T>(url: string, signal: AbortSignal): Promise<T> {
  const response = await fetch(url, { signal });
  if (!response.ok) throw new ApiError(response.status);
  return response.json();
}

export const snapshotQuery = queryOptions({
  queryKey: ["snapshot"],
  queryFn: ({ signal }) => getJson<Snapshot>("/data/snapshot.json", signal),
  staleTime: Infinity,
});

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
