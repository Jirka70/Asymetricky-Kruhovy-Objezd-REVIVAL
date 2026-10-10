import { proxyApi } from "@/lib/server/api-proxy";

export async function GET(
  request: Request,
  context: { params: Promise<{ path: string[] }> },
) {
  return proxyApi(request, (await context.params).path);
}
