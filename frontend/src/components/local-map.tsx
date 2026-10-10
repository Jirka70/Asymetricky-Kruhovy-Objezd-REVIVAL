"use client";
import { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { zsjListQuery } from "@/lib/api";
import type { EChartsOption } from "echarts";
import {
  bucket,
  TIME_COLORS,
  time,
  escapeHtml,
  type Zone,
  type Travel,
} from "@/lib/data";
import Chart from "./chart";
export default function LocalMap({
  id,
  zones,
  times,
}: {
  id: string;
  zones: Zone[];
  times: Travel;
}) {
  const [ready, setReady] = useState("");
  const query = useQuery(zsjListQuery);
  const mapName = `local-${id}`;
  const ids = zones.map((z) => z.id).join(",");
  useEffect(() => {
    if (!query.data) return;
    const available = query.data;
    let active = true;
    void import("@/lib/echarts").then(({ echarts }) => {
      if (!active) return;
      const selected = new Set(ids.split(","));
      echarts.registerMap(mapName, {
        type: "FeatureCollection",
        features: available.filter((zone) => selected.has(zone.kod)).map((zone) => ({
          type: "Feature", properties: { name: zone.kod }, geometry: zone.boundary,
        })),
      });
      setReady(mapName);
    });
    return () => { active = false; };
  }, [mapName, ids, query.data]);
  const option = useMemo<EChartsOption>(
    () => ({
      tooltip: {
        confine: true,
        formatter: (p) => {
          const z = zones.find((z) => z.id === (p as { name: string }).name);
          return z ? `${escapeHtml(z.name)}<br/>${time(times[z.id])}` : "";
        },
      },
      geo: {
        map: mapName,
        layoutCenter: ["50%", "50%"],
        layoutSize: "90%",
        aspectScale: 0.65,
        silent: false,
        itemStyle: { borderColor: "#fff", borderWidth: 0.8 },
        emphasis: {
          label: { show: false },
          itemStyle: { borderColor: "#254934", borderWidth: 2 },
        },
        regions: zones.map((z) => ({
          name: z.id,
          itemStyle: { areaColor: TIME_COLORS[bucket(times[z.id] ?? null)] },
        })),
      },
    }),
    [mapName, zones, times],
  );
  return (
    <div className="local-map">
      {ready === mapName ? (
        <Chart
          height={172}
          option={option}
          label={`Detail ZSJ vybrané obce. ${zones.map((z) => `${z.name}: ${time(times[z.id])}`).join("; ")}`}
        />
      ) : (
        <div className="local-map-loading">Načítání detailu…</div>
      )}
      <span>Dojezdy po jednotlivých ZSJ</span>
    </div>
  );
}
