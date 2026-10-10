"use client";
import { useMemo, useState } from "react";
import dynamic from "next/dynamic";
import { useQuery } from "@tanstack/react-query";
import { studentRouteQuery } from "@/lib/api";
import { number } from "@/lib/data";
import ConnectionSummary from "./connection-summary";
import { DataSkeleton, QueryStatus } from "./query-state";

const modes = { WALK: "Chůze", BUS: "Autobus", RAIL: "Vlak", TRAM: "Tramvaj", TROLLEYBUS: "Trolejbus" };

const RouteMap = dynamic(() => import("./student-route-map"), {
  ssr: false, loading: () => <DataSkeleton compact label="Načítání mapy trasy…" />,
});

export default function StudentRoute({ lat, lon, redizo }: { lat: number; lon: number; redizo: string }) {
  const query = useQuery(studentRouteQuery({ lat, lon, redizo }));
  const [selected, setSelected] = useState<number | null>(null);
  const chosen = query.data?.spoje.find((journey) => journey.spoj === selected) ?? query.data?.spoje[0];
  const legs = useMemo(() => query.data?.features.filter((leg) => leg.properties.spoj === chosen?.spoj)
    .sort((left, right) => left.properties.usek - right.properties.usek) ?? [], [query.data, chosen?.spoj]);
  return <section className="detail-section student-route" aria-label="Spojení do školy">
    <h3>Spojení do školy</h3>
    <QueryStatus queries={[query]} />
    {query.data === undefined ? query.error
      ? <p className="detail-note">Podrobnosti trasy nejsou dostupné. Souhrn dojezdu je uveden výše.</p>
      : <DataSkeleton compact label="Načítání spojení…" />
      : <>
        {query.data.meta.den && <p className="data-note">Den spojení: {query.data.meta.den.split("-").reverse().join(". ")} · místní čas</p>}
        {query.data.spoje.length === 0 ? <p>Pro tuto cestu nebylo v ranním okně nalezeno spojení.</p> : <>
          <div className="route-options" role="group" aria-label="Varianty spojení">
            {query.data.spoje.map((journey, index) => <button key={journey.spoj} type="button"
              aria-pressed={journey.spoj === chosen?.spoj} onClick={() => setSelected(journey.spoj)}>
              Spoj {index + 1} · {journey.cas_min} min · {journey.odjezd}–{journey.prijezd}
            </button>)}
          </div>
          {chosen && <ConnectionSummary connection={{ ...chosen, stav: "ok" }} />}
          {legs.some((leg) => leg.geometry.coordinates.length > 1) && <RouteMap legs={legs} />}
          <ol className="route-legs" aria-label="Úseky vybraného spojení">
            {legs.map(({ properties: leg }) => <li key={leg.usek}>
              <strong>{modes[leg.druh]}{leg.linka ? ` · ${leg.linka}` : ""}</strong>
              <span>{leg.od} → {leg.do}</span>
              <small>{leg.odjezd}–{leg.prijezd}{leg.chuze_m == null ? "" : ` · ${number(leg.chuze_m)} m`}</small>
            </li>)}
          </ol>
        </>}
      </>}
  </section>;
}
