import { proxyApi } from "@/lib/server/api-proxy";
import { fetchMapValues } from "@/lib/server/map-values-cache";

export async function GET(
  request: Request,
  context: { params: Promise<{ path: string[] }> },
) {
  const { path } = await context.params;
  return proxyApi(request, path, undefined, path.length === 1 && ["simulace", "zsj"].includes(path[0]) ? fetchMapValues : fetch);
}
