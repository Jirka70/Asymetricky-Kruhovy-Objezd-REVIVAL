// Deterministic HTTP backend for the production Next.js integration tests.
import { createServer } from "node:http";
import { responseFor } from "../fixtures/api.mjs";

const counts = {};
createServer((request, response) => {
  const url = new URL(request.url, "http://127.0.0.1:8107");
  response.setHeader("Content-Type", "application/json");
  if (url.pathname === "/__counts") return response.end(JSON.stringify(counts));
  const body = responseFor(url);
  counts[url.pathname + url.search] = (counts[url.pathname + url.search] ?? 0) + 1;
  response.writeHead(body ? 200 : 404);
  response.end(JSON.stringify(body ?? { error: { zprava: "Nenalezeno" } }));
}).listen(8107, "127.0.0.1");
