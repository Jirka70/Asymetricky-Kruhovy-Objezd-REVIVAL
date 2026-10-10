export const points = (features) => ({ type: "FeatureCollection", features });
export const schoolFeature = (redizo, nazev, count = 1) => ({
  type: "Feature",
  geometry: { type: "Point", coordinates: [12.95, 50.3] },
  properties: { redizo, nazev, pocet_nabidek: count, kapacita: 77, prihlasky: 999 },
});
export const schools = points([
  schoolFeature("600009084", "SPŠ Ostrov z API"),
  schoolFeature("600170527", "ISŠTE Sokolov z API"),
  schoolFeature("600009271", "SLŠ Žlutice z API", 0),
]);
export const program = (kod, nazev, jobs) => ({
  kod, nazev, pocet_skol: 2, kapacita: 154, prihlasky: 1998,
  volna_mista: jobs, zamestnavatelu: jobs === null ? null : 1,
});
export const daily = { data: [
  program("18-20-M/01", "Informační technologie", 321),
  program("23-51-E/01", "Strojírenské práce", null),
] };
export const distance = { data: [program("26-41-M/01", "Elektrotechnika", 0)] };
export const detail = {
  redizo: "600009084", nazev: "SPŠ Ostrov z API", adresa: "Adresa z API 123",
  web: "https://example.com", lat: 50.3, lon: 12.95,
  nabidky: [
    { kod_oboru: "18-20-M/01", nazev_oboru: "Informační technologie", forma: "den", kapacita: 30, prihlasky: 400 },
    { kod_oboru: "18-20-M/01", nazev_oboru: "Informační technologie", forma: "den", kapacita: 47, prihlasky: 599 },
  ],
};
export const employers = {
  ...points([{
    type: "Feature", geometry: { type: "Point", coordinates: [12.9, 50.2] },
    properties: {
      id: "694625177794571187", ico: "02183765", nazev: "Zaměstnavatel z API",
      kod_obce: "554961", pocet_mist: 321,
      profese: [{ cz_isco3: "351", nazev: "Technici z API", vhodnost: 1, pocet_mist: 321 }],
    },
  }]),
  meta: { mapovani: true, existuje: true, importovano: "2026-10-10T08:00:00Z", bez_souradnic: [] },
};
export function responseFor(url) {
  const path = decodeURIComponent(url.pathname);
  if (path === "/api/v1/skoly") {
    const field = url.searchParams.get("obor");
    if (!field) return schools;
    if (field === "18-20-M/01" && url.searchParams.get("forma") === "den") {
      return points(schools.features.slice(0, 2));
    }
    return points([]);
  }
  if (path === "/api/v1/obory") return url.searchParams.get("forma") === "dal" ? distance : daily;
  if (path.startsWith("/api/v1/skoly/")) return { ...detail, redizo: path.split("/").at(-1) };
  if (path.endsWith("/zamestnavatele")) {
    if (path.includes("18-20-M/01")) return employers;
    return { ...points([]), meta: { mapovani: false, existuje: null } };
  }
  return null;
}
