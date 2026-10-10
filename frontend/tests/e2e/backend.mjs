// Deterministic HTTP backend for the production Next.js integration tests.
import { createServer } from "node:http";
import { responseFor } from "../fixtures/api.mjs";

const counts = {};
createServer((request, response) => {
  const url = new URL(request.url, "http://127.0.0.1:8107");
  response.setHeader("Content-Type", "application/json");
  if (url.pathname === "/__counts") return response.end(JSON.stringify(counts));
  const body = responseFor(url);
  if (["/api/v1/simulace", "/api/v1/zsj"].includes(url.pathname) && body?.features.length) {
    // Exercise the real large-GeoJSON cache boundary, not just a tiny mock.
    body.features[0].geometry = {type: "Polygon", coordinates: [Array(350000).fill([12, 50])]};
  }
  counts[url.pathname + url.search] = (counts[url.pathname + url.search] ?? 0) + 1;
  if (url.pathname === "/api/v1/simulace" && url.searchParams.get("redizo") === "600000000") {
    response.writeHead(404);
    return response.end(JSON.stringify({error: {kod: "skola_nenalezena"}}));
  }
  response.writeHead(body ? 200 : 404);
  response.end(JSON.stringify(body ?? { error: { zprava: "Nenalezeno" } }));
}).listen(8107, "127.0.0.1");
