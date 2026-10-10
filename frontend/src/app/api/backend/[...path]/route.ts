import { proxyApi } from "@/lib/server/api-proxy";
import { fetchMapValues } from "@/lib/server/map-values-cache";

export async function GET(
  request: Request,
  context: { params: Promise<{ path: string[] }> },
) {
  const { path } = await context.params;
  const compact = (path.length === 1 && ["simulace", "zsj"].includes(path[0])) || (path.length === 2 && path[0] === "zsj" && path[1] === "seznam");
  return proxyApi(request, path, undefined, compact ? fetchMapValues : fetch);
}

export async function POST(request: Request, context: { params: Promise<{ path: string[] }> }) {
  const { path } = await context.params;
  return proxyApi(request, path, undefined, fetchMapValues);
}
