import { unstable_cache } from "next/cache";
import { displayZones } from "./zsj-display";

class UpstreamError extends Error {
  constructor(readonly status: number, readonly body: string) {
    super(`Analytical request failed (${status})`);
  }
}

// The full GeoJSON is ~13 MB, above Next's fetch-cache limit. The map already
// has its geometry: cache just the small analytical payload, keyed by full URL.
const loadMapValues = unstable_cache(async (url: string, body?: string) => {
  const response = await fetch(url, {
    cache: "no-store", signal: AbortSignal.timeout(15_000),
    ...(body === undefined ? {} : { method: "POST", body, headers: { "Content-Type": "application/json" } }),
  });
  if (!response.ok) throw new UpstreamError(response.status, await response.text());
  const data = await response.json();
  if (data.type === "FeatureCollection") {
    data.features = data.features.map(({ properties }: { properties: Record<string, unknown> }) => ({ properties }));
  }
  return data;
}, ["map-values-v2"], { revalidate: 86400 });

const loadZsjList = unstable_cache(async (url: string) => {
  const response = await fetch(url, { cache: "no-store", signal: AbortSignal.timeout(15_000) });
  if (!response.ok) throw new UpstreamError(response.status, await response.text());
  return displayZones(await response.json());
}, ["zsj-display-v1"], { revalidate: 86400 });

export const fetchMapValues: typeof fetch = async (input, options) => {
  try {
    return Response.json(new URL(String(input)).pathname.endsWith("/zsj/seznam")
      ? await loadZsjList(String(input))
      : await loadMapValues(String(input), typeof options?.body === "string" ? options.body : undefined));
  } catch (error) {
    // Throwing inside the cached function keeps errors out of the daily cache.
    if (error instanceof UpstreamError) {
      return new Response(error.body, { status: error.status, headers: { "Content-Type": "application/json" } });
    }
    throw error;
  }
};
