/** Cache only the public, implemented catalog reads, never future live routing. */
function supportedPath(path: string[]) {
  return (
    (path.length === 1 && ["skoly", "obory"].includes(path[0])) ||
    (path.length === 2 && path[0] === "skoly" && /^\d{9}$/.test(path[1])) ||
    (path.length === 3 && path[0] === "obory" &&
      /^\d{2}-\d{2}-[A-Z]\/\d{2}$/.test(path[1]) && path[2] === "zamestnavatele")
  );
}

export async function proxyApi(
  request: Request,
  path: string[],
  backend = process.env.BACKEND_URL ?? "http://127.0.0.1:8000",
  fetcher: typeof fetch = fetch,
) {
  if (!supportedPath(path)) {
    return Response.json({ error: { zprava: "Endpoint není dostupný." } }, { status: 404 });
  }
  const url = new URL(`/api/v1/${path.map(encodeURIComponent).join("/")}`, backend);
  url.search = new URL(request.url).search;
  url.searchParams.sort();
  try {
    const response = await fetcher(url, {
      next: { revalidate: 86400 },
      signal: AbortSignal.timeout(15_000),
    });
    return new Response(await response.text(), {
      status: response.status,
      headers: {
        "Content-Type": response.headers.get("Content-Type") ?? "application/json",
        // Browser freshness is managed by TanStack Query; Next caches the upstream fetch.
        "Cache-Control": "no-store",
      },
    });
  } catch {
    return Response.json(
      { error: { zprava: "Backend není dostupný." } },
      { status: 502, headers: { "Cache-Control": "no-store" } },
    );
  }
}
