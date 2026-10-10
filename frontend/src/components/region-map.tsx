"use client";
import { useEffect, useMemo, useRef, useState } from "react";
import type { EChartsType } from "echarts/core";
import type { EChartsOption } from "echarts";
import {
  PlusIcon,
  MinusIcon,
  CornersOutIcon,
  ArrowUpIcon,
  GraduationCapIcon,
  BriefcaseIcon,
  CheckIcon,
  XIcon,
} from "@phosphor-icons/react";
import Chart from "./chart";
import MapInstitutions from "./map-institutions";
import { MAX_MAP_ZOOM, type MapPlace } from "@/lib/map-clusters";
import { echarts } from "@/lib/echarts";
import {
  TIME_COLORS,
  TIME_LABELS,
  bucket,
  number,
  time,
  escapeHtml,
  type Snapshot,
  type Travel,
} from "@/lib/data";
export type MapMode = "current" | "scenario" | "difference";
type Boundary = { coords: number[][] };
type MunicipalityGeometry = {
  features: { geometry:
    | { type: "Polygon"; coordinates: number[][][] }
    | { type: "MultiPolygon"; coordinates: number[][][][] }
  }[];
};
let geometry: Promise<Boundary[]> | undefined;
function loadMaps() {
  geometry ??= Promise.all(
    ["zsj", "municipalities"].map(async (name) => {
      const response = await fetch(`/data/${name}.geojson`);
      if (!response.ok) throw Error("Chybí mapová data");
      return response.json();
    }),
  )
    .then(([zones, municipalityData]) => {
      const municipalities = municipalityData as MunicipalityGeometry;
      echarts.registerMap("zsj", zones);
      return municipalities.features.flatMap(({ geometry }) => {
        const polygons = geometry.type === "Polygon" ? [geometry.coordinates] : geometry.coordinates;
        return polygons.flatMap((rings) => rings.map((coords) => ({ coords })));
      });
    })
    .catch((error) => {
      geometry = undefined;
      throw error;
    });
  return geometry;
}
const cities = [
  ["Aš", 12.194, 50.227],
  ["Cheb", 12.368, 50.08],
  ["Sokolov", 12.645, 50.183],
  ["Karlovy Vary", 12.882, 50.23],
  ["Ostrov", 12.945, 50.31],
  ["Mariánské Lázně", 12.701, 49.974],
  ["Toužim", 12.985, 50.063],
  ["Žlutice", 13.16, 50.096],
] as const;
export default function RegionMap({
  data,
  times,
  before,
  schoolIds,
  field,
  employersNotice,
  onSelectSchool,
  selectedSchoolId,
  selectedEmployerId,
  onSelectEmployer,
  compact = false,
  family = false,
  mode = "current",
  onModeChange,
  hasScenario = false,
}: {
  data: Snapshot;
  times: Travel;
  before?: Travel;
  schoolIds: Set<string>;
  field: string;
  employersNotice?: string;
  onSelectSchool?: (id: string) => void;
  selectedSchoolId?: string;
  selectedEmployerId?: string;
  onSelectEmployer?: (id: string) => void;
  compact?: boolean;
  family?: boolean;
  hasScenario?: boolean;
  mode?: MapMode;
  onModeChange?: (mode: MapMode) => void;
}) {
  const [ready, setReady] = useState(false);
  const [boundaries, setBoundaries] = useState<Boundary[]>([]);
  const [failed, setFailed] = useState(false);
  const [retry, setRetry] = useState(0);
  const [schools, setSchools] = useState(true);
  const [employers, setEmployers] = useState(true);
  const chartRef = useRef<EChartsType | null>(null);
  const [mapChart, setMapChart] = useState<EChartsType | null>(null);
  const hoveredZone = useRef<string | null>(null);
  useEffect(() => {
    if (!mapChart) return;
    // Clear the previous area's tooltip while the next area's delay runs.
    const rememberZone = (event: { componentType?: string; name?: string }) => {
      hoveredZone.current = event.componentType === "geo" ? event.name ?? null : null;
    };
    const hideTooltip = () => {
      hoveredZone.current = null;
      mapChart.dispatchAction({ type: "hideTip" });
    };
    mapChart.on("mousemove", rememberZone);
    mapChart.on("mouseout", hideTooltip);
    mapChart.on("georoam", hideTooltip);
    return () => {
      hoveredZone.current = null;
      mapChart.off("mousemove", rememberZone);
      mapChart.off("mouseout", hideTooltip);
      mapChart.off("georoam", hideTooltip);
    };
  }, [mapChart]);
  const frameRef = useRef<HTMLDivElement>(null);
  const [mapSize, setMapSize] = useState({ width: 900, height: 600 });
  useEffect(() => {
    if (!frameRef.current) return;
    const observer = new ResizeObserver(([entry]) => {
      if (entry.contentRect.width > 0)
        setMapSize({
          width: entry.contentRect.width,
          height: entry.contentRect.height,
        });
    });
    observer.observe(frameRef.current);
    return () => observer.disconnect();
  }, []);
  useEffect(() => {
    let active = true;
    loadMaps()
      .then((boundaries) => {
        if (active) {
          setBoundaries(boundaries);
          setReady(true);
          setFailed(false);
        }
      })
      .catch(() => {
        if (active) setFailed(true);
      });
    return () => {
      active = false;
    };
  }, [retry]);
  const places = useMemo<MapPlace[]>(
    () => [
      ...(schools
        ? data.schools
            .filter((s) => !family || schoolIds.has(s.id))
            .map((s) => ({
              key: `school:${s.id}`,
              id: s.id,
              kind: schoolIds.has(s.id)
                ? ("school-offered" as const)
                : ("school-other" as const),
              name: s.shortName,
              city: s.city,
              lon: s.lon,
              lat: s.lat,
              selected: s.id === selectedSchoolId,
            }))
        : []),
      ...(employers
        ? data.employers
            .filter((e) => e.field === field)
            .map((e) => ({
              key: `employer:${e.id}`,
              id: e.id,
              kind: "employer" as const,
              name: e.name,
              city: e.city,
              lon: e.lon,
              lat: e.lat,
              selected: e.id === selectedEmployerId,
              hint: e.professions.some((p) => p.suitability === 1)
                ? "Obor: nejvhodnější shoda"
                : "Obor: vhodná shoda",
            }))
        : []),
    ],
    [
      data,
      schools,
      employers,
      family,
      schoolIds,
      field,
      selectedSchoolId,
      selectedEmployerId,
    ],
  );
  function selectPlace(place: MapPlace) {
    if (place.kind === "employer") onSelectEmployer?.(place.id);
    else onSelectSchool?.(place.id);
  }
  const option = useMemo<EChartsOption>(() => {
    const regions = data.zsj.map((zone) => {
      let color = TIME_COLORS[bucket(times[zone.id] ?? null)];
      if (mode === "difference" && before) {
        const current = times[zone.id] ?? Infinity,
          previous = before[zone.id] ?? Infinity;
        color =
          current < previous
            ? "#428976"
            : current > previous
              ? "#c78963"
              : "#e1e4d9";
      }
      return {
        name: zone.id,
        itemStyle: {
          areaColor: color,
          borderColor: "#fff",
          borderWidth: 0.3,
        },
        emphasis: {
          itemStyle: {
            areaColor: color,
            borderColor: "#1c3c2e",
            borderWidth: 2,
          },
        },
      };
    });
    return {
      tooltip: {
        trigger: "item",
        showDelay: 500,
        hideDelay: 0,
        transitionDuration: 0,
        className: "zsj-tooltip",
        confine: true,
        backgroundColor: "#fff",
        borderColor: "#d6dce0",
        padding: 12,
        textStyle: {
          fontFamily: '"Helvetica Neue", Arial, sans-serif',
          fontSize: 12,
          color: "#24292f",
        },
      },
      geo: {
        map: "zsj",
        // Geo regions do not inherit the global series tooltip formatter.
        tooltip: {
          formatter: ({ name }) => {
            // ECharts may finish its showDelay timer after the pointer left.
            if (hoveredZone.current !== name) return "";
            const zone = data.zsj.find((z) => z.id === name);
            if (!zone) return "";
            const municipality = data.municipalities.find((m) => m.id === zone.municipality);
            const duration = (value: number | null | undefined) =>
              value == null ? "Bez uloženého spojení" : time(value);
            const journey = mode === "current"
              ? `Dojezd: <strong>${duration(times[zone.id])}</strong>`
              : `Současný stav: <strong>${duration(before?.[zone.id])}</strong><br/>Scénář: <strong>${duration(times[zone.id])}</strong>`;
            return `<div style="max-width:280px;white-space:normal;line-height:1.5">
              <strong>${escapeHtml(zone.name)}</strong><br/>
              Obec: ${escapeHtml(municipality?.name ?? zone.municipality)}<br/>
              <span style="color:#606870">ZSJ ${escapeHtml(zone.id)}</span>
              <div style="margin-top:8px">Odhad dětí 10–14 let: <strong>${zone.children == null ? "bez dat" : number(zone.children)}</strong><br/>
              <span style="color:#606870">Demografický podklad: ${data.meta.demographyYear}</span></div>
              <div style="margin-top:8px">K nejbližší škole s vybraným oborem<br/>${journey}</div>
            </div>`;
          },
        },
        roam: true,
        layoutCenter: ["50%", "48%"],
        layoutSize: Math.min(
          mapSize.width * 0.91,
          Math.max(240, mapSize.height - (family ? 125 : 90)) *
            (family ? 1.36 : 1.65),
        ),
        aspectScale: 0.65,
        scaleLimit: { min: 1, max: MAX_MAP_ZOOM },
        zoom: 1,
        regions,
        selectedMode: false,
        emphasis: { label: { show: false } },
        itemStyle: {
          areaColor: "#c8cdc5",
          borderColor: "#fff",
          borderWidth: 0.1,
        },
        label: { show: false },
      },
      series: [
        {
          // Reuse the ZSJ projection: pan, zoom and resize stay in sync.
          type: "lines",
          name: "Hranice obcí",
          coordinateSystem: "geo",
          geoIndex: 0,
          polyline: true,
          data: boundaries,
          silent: true,
          tooltip: { show: false },
          emphasis: { disabled: true },
          lineStyle: { color: "#fff", width: 0.7, opacity: 1 },
          progressive: 0,
          z: 2,
        },
        {
          type: "scatter",
          name: "Města",
          coordinateSystem: "geo",
          silent: true,
          symbolSize: 3,
          itemStyle: { color: "#566552" },
          label: {
            show: true,
            position: "bottom",
            formatter: "{b}",
            fontSize: 11,
            fontFamily: '"Helvetica Neue", Arial, sans-serif',
            fontWeight: 500,
            color: "#343d45",
            backgroundColor: "rgba(255,255,255,.92)",
            padding: [2, 4],
            borderRadius: 2,
          },
          data: cities.map((c) => ({ name: c[0], value: [c[1], c[2]] })),
          labelLayout: { hideOverlap: true },
          z: 3,
        },
      ],
    };
  }, [data, times, before, mode, mapSize, family, boundaries]);
  const legend =
    mode === "difference"
      ? [
          ["#428976", "Zlepšení"],
          ["#e1e4d9", "Beze změny"],
          ["#c78963", "Zhoršení"],
        ]
      : TIME_COLORS.map((color, i) => [color, TIME_LABELS[i]]);
  const layers = (
    <div className="map-layers">
      <label>
        <input
          type="checkbox"
          checked={schools}
          onChange={(e) => setSchools(e.target.checked)}
        />
        <GraduationCapIcon size={17} />
        Školy <span>{family ? schoolIds.size : data.schools.length}</span>
      </label>
      <label>
        <input
          type="checkbox"
          checked={employers}
          onChange={(e) => setEmployers(e.target.checked)}
        />
        <BriefcaseIcon size={16} />
        Zaměstnavatelé
        {employersNotice && <span title={employersNotice}>· {employersNotice}</span>}
      </label>
    </div>
  );
  return (
    <div
      className={`map-frame workspace-map-frame${family ? " is-family-map" : " is-region-map"}${compact ? " map-compact" : ""}`}
      ref={frameRef}
    >
      <div className="map-canvas">
        <span className="map-country">NĚMECKO</span>
        <span className="map-neighbor">Ústecký kraj</span>
        <span className="map-neighbor-south">Plzeňský kraj</span>
        {ready ? (
          <Chart
            label="Mapa dojezdů ze základních sídelních jednotek"
            option={option}
            height={500}
            chartRef={chartRef}
            onReady={setMapChart}
          />
        ) : (
          <div className="map-stage" role="status">
            {failed ? (
              <button onClick={() => setRetry(retry + 1)}>
                Znovu načíst mapu
              </button>
            ) : (
              "Načítání mapy…"
            )}
          </div>
        )}
        {mapChart && (
          <MapInstitutions
            chart={mapChart}
            places={places}
            onSelect={selectPlace}
          />
        )}
        <div className="map-toolbar">
          {family ? (
            layers
          ) : (
            <>
              <div className="segmented" aria-label="Pohled na scénář">
                {(
                  [
                    ["current", "Současný stav"],
                    ["scenario", "Scénář"],
                    ["difference", "Změna"],
                  ] as const
                ).map(([v, l]) => (
                  <button
                    key={v}
                    aria-pressed={mode === v}
                    disabled={v !== "current" && !hasScenario}
                    title={
                      v !== "current" && !hasScenario
                        ? "Nejprve přidejte změnu do simulace."
                        : undefined
                    }
                    onClick={() => onModeChange?.(v)}
                  >
                    {l}
                  </button>
                ))}
              </div>
              {layers}
            </>
          )}
        </div>
        <div className="map-zoom">
          <button
            aria-label="Přiblížit mapu"
            onClick={() =>
              chartRef.current?.dispatchAction({
                type: "geoRoam",
                zoom: 1.4,
                geoIndex: 0,
                originX: chartRef.current.getWidth() / 2,
                originY: chartRef.current.getHeight() / 2,
              })
            }
          >
            <PlusIcon size={18} />
          </button>
          <button
            aria-label="Oddálit mapu"
            onClick={() =>
              chartRef.current?.dispatchAction({
                type: "geoRoam",
                zoom: 1 / 1.4,
                geoIndex: 0,
                originX: chartRef.current.getWidth() / 2,
                originY: chartRef.current.getHeight() / 2,
              })
            }
          >
            <MinusIcon size={18} />
          </button>
          <button
            aria-label="Zobrazit celý kraj"
            onClick={() =>
              chartRef.current?.setOption({ geo: { zoom: 1, center: null } })
            }
          >
            <CornersOutIcon size={18} />
          </button>
        </div>
        <div className="map-north">
          <ArrowUpIcon size={22} weight="fill" />
          <span>S</span>
        </div>
        <div className="map-legend">
          <strong>
            {mode === "difference"
              ? "Změna dostupnosti po simulaci"
              : "Dojezd k nejbližší škole (ZSJ)"}
          </strong>
          <div>
            {legend.map(([c, l]) => (
              <span key={l}>
                <i style={{ background: c }} />
                {l}
              </span>
            ))}
          </div>
          <div className="point-legend">
            <span>
              <GraduationCapIcon
                size={16}
                weight="fill"
                className="legend-school"
              />
              {family ? (
                "Škola s oborem"
              ) : (
                <>
                  <CheckIcon size={12} />
                  Škola s oborem
                </>
              )}
            </span>
            {!family && (
              <span>
                <GraduationCapIcon
                  size={16}
                  weight="fill"
                  className="legend-no-field"
                />
                <XIcon size={12} />
                Škola bez oboru
              </span>
            )}
            <span>
              <BriefcaseIcon
                size={15}
                weight="fill"
                className="legend-employer"
              />
              Zaměstnavatel
            </span>
          </div>
        </div>
      </div>
      <div className="map-attribution">
        RÚIAN · uložený výpočet OTP · Číslo = počet institucí
      </div>
    </div>
  );
}
