"use client";
import { useEffect, useMemo, useState } from "react";
import type { EChartsOption } from "echarts";
import { echarts } from "@/lib/echarts";
import {
  bucket,
  TIME_COLORS,
  time,
  escapeHtml,
  type Zone,
  type Travel,
} from "@/lib/data";
import Chart from "./chart";
type Geo = {
  type: "FeatureCollection";
  features: {
    type: "Feature";
    properties: { name: string };
    geometry: unknown;
  }[];
};
let source: Promise<Geo> | undefined;
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
  const mapName = `local-${id}`;
  const ids = zones.map((z) => z.id).join(",");
  useEffect(() => {
    let active = true;
    source ??= fetch("/data/zsj.geojson").then((r) => {
      if (!r.ok) throw Error();
      return r.json();
    });
    source
      .then((geo) => {
        const selected = new Set(ids.split(","));
        if (!echarts.getMap(mapName))
          echarts.registerMap(mapName, {
            ...geo,
            features: geo.features.filter((f) =>
              selected.has(f.properties.name),
            ),
          } as Parameters<typeof echarts.registerMap>[1]);
        if (active) setReady(mapName);
      })
      .catch(() => {
        source = undefined;
      });
    return () => {
      active = false;
    };
  }, [mapName, ids]);
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
