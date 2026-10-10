import { readFileSync } from "node:fs";
const zones = JSON.parse(readFileSync(new URL("../../public/data/snapshot.json", import.meta.url))).zsj;
const zoneGeometry = JSON.parse(readFileSync(new URL("../../public/data/zsj.geojson", import.meta.url)));
export const zsjList = zones.map((zone, index) => ({
  kod: zone.id, nazev: zone.name, lat: zone.lat, lon: zone.lon, kod_obce: zone.municipality,
  boundary: zoneGeometry.features[index].geometry,
}));
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
    { kod_oboru: "18-20-M/01", nazev_oboru: "Informační technologie", forma: "den", kapacita: 30, prihlasky: 400, prijati: 20 },
    { kod_oboru: "18-20-M/01", nazev_oboru: "Informační technologie", forma: "den", kapacita: 47, prihlasky: 599, prijati: 30 },
    { kod_oboru: "18-20-M/01", nazev_oboru: "Informační technologie", forma: "dal", kapacita: 100, prihlasky: 200, prijati: 100 },
    { kod_oboru: "26-41-M/01", nazev_oboru: "Elektrotechnika", forma: "den", kapacita: 10, prihlasky: 20, prijati: 10 },
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
export function responseFor(url, requestBody) {
  const path = decodeURIComponent(url.pathname);
  if (path === "/api/v1/zsj/seznam") return zsjList;
  if (path === "/api/v1/simulace/zmeny") return removal(requestBody);
  if (path === "/api/v1/zsj") return accessibility(url.searchParams);
  if (path === "/api/v1/student/trasa") return studentRoute();
  if (path === "/api/v1/student/skoly") return studentSchools(url.searchParams);
  if (/^\/api\/v1\/obory\/\d{2}-\d{2}-[A-Z]\/\d{2}$/.test(path)) return programDetail();
  if (path === "/api/v1/simulace") return simulation(url.searchParams);
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

export function simulation(params = new URLSearchParams()) {
  return {
    type: "FeatureCollection",
    features: zones.map((zone) => ({
      type: "Feature",
      properties: {
        kod: zone.id, nazev: zone.name, deti: 10,
        cas_min_puvodni: null,
        cas_min: zone.id === "001261" ? 12.5 : null,
        nejblizsi_redizo: zone.id === "001261" ? params.get("redizo") ?? "600009271" : null,
      },
    })),
    souhrn: {
      zlepsenych_jednotek: 1, potencialni_uchazeci: 17.25,
      odlehceni: 12.5, pretazeni: 3, novi_v_dosahu: 4.75,
      vyuziti: 17.25 / Number(params.get("kapacita") ?? 30), verdikt: "dobre_misto",
    },
    meta: {},
  };
}

export function accessibility(params = new URLSearchParams()) {
  return {type: "FeatureCollection", features: zones.map((zone, index) => ({
    properties: {kod: zone.id, nazev: zone.name, cas_min: params.get("forma") === "dal" ? undefined : index === 0 ? 45 : 80,
      nejblizsi_redizo: params.get("forma") === "dal" ? undefined : zone.id === "001261" ? "600170527" : "600009084",
      deti: 10, v_dosahu: params.get("max_min") === "0" && params.get("forma") !== "dal", deti_v_dosahu: 0, pasmo: "nad_60"},
  })), meta: {}};
}
export function studentSchools(params = new URLSearchParams()) {
  return {data: params.get("forma") === "dal" ? [] : schools.features.map((feature, i) => ({
    redizo: feature.properties.redizo, nazev: feature.properties.nazev,
    lat: feature.geometry.coordinates[1], lon: feature.geometry.coordinates[0],
    v_dosahu: i === 0 || (params.get("max_min") === "0" && i < 2), spoj: i === 2 ? {stav: "data_nedostupna"} : {stav: "ok", cas_min: i === 0 ? 17 : 180, odjezd: i === 0 ? "07:12" : "04:29", prijezd: "07:29", prestupy: 0, chuze_m: 250, vzdalenost_m: 4100, linky: ["421"]},
    nabidky: [{kod_oboru: "18-20-M/01", nazev_oboru: "Informační technologie", forma: "den", kapacita: 77, prihlasky: 999, prijati: 50}],
  })), meta: {max_min: Number(params.get("max_min") ?? 120), zsj: "063550", den: "2026-10-12"}};
}
export function programDetail() {
  return {
    obor: {kod: "18-20-M/01", nazev: "Detail IT z API", pocet_skol: 2, kapacita: 432, prihlasky: 876,
      prihlasky_na_misto: 2.03, deti_v_dosahu: 123, deti_bez_oboru: 456, podil_deti_v_dosahu: 21.24},
    nabidky: [{...detail.nabidky[0], redizo: detail.redizo, nazev_skoly: detail.nazev}],
    kandidati: [{redizo: "600009271", nazev: "SLŠ Žlutice z API", nove_dosazene_deti: 321, ma_pribuzny_obor: true}],
    trh_prace: null,
  };
}

export function removal(input = {max_min: 45, zmeny: [{redizo: "600009084", zmena_kapacity: -77}]}) {
  const lost = input.max_min < 60;
  return {
    type: "FeatureCollection",
    features: zones.map((zone, index) => ({
      type: "Feature",
      properties: {
        kod: zone.id, nazev: zone.name, deti: 10,
        cas_min_puvodni: index === 0 ? 35 : 80,
        // Omitted time is intentional: removal of the last accessible school.
        ...(index === 0 ? {} : {cas_min: 80}),
      },
    })),
    souhrn: {zlepsenych_jednotek: 0, zhorsenych_jednotek: 1,
      deti_v_dosahu_pred: 10, deti_v_dosahu_po: lost ? 0 : 10,
      ztracene_deti: lost ? 10 : 0, nove_dosazene_deti: 0,
      kapacita_pred: 154, kapacita_po: 154 + input.zmeny[0].zmena_kapacity,
    }, meta: {},
  };
}

export function studentRoute() {
  const leg = (spoj, usek, druh, od, to, odjezd, prijezd, coordinates) => ({
    type: "Feature", geometry: {type: "LineString", coordinates},
    properties: {spoj, usek, druh, od, do: to, odjezd, prijezd,
      ...(druh === "WALK" ? {chuze_m: 250} : {linka: "421"})},
  });
  return {type: "FeatureCollection", features: [
    // Deliberately mixed order: the UI must group alternatives and sort legs.
    leg(0, 1, "BUS", "Zastávka z API", "Škola z API", "07:15", "07:29", [[12.81, 50.205], [12.83, 50.21], [12.95, 50.3]]),
    leg(1, 0, "RAIL", "Nádraží z API", "Cílové nádraží", "07:00", "07:30", [[12.8, 50.2], [12.95, 50.3]]),
    leg(0, 0, "WALK", "Výchozí bod ZSJ", "Zastávka z API", "07:12", "07:15", [[12.8, 50.2], [12.805, 50.201], [12.81, 50.205]]),
  ], spoje: [
    {spoj: 0, odjezd: "07:12", prijezd: "07:29", cas_min: 17, prestupy: 0, chuze_m: 250, vzdalenost_m: 4100},
    {spoj: 1, odjezd: "07:00", prijezd: "07:30", cas_min: 30, prestupy: 0, chuze_m: 0},
  ], meta: {den: "2026-10-12", scenar: "rano", okno_prijezdu: ["07:00", "08:00"]}};
}
