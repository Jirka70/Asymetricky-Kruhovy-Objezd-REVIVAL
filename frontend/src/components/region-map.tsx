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
  ListBulletsIcon,
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
  time,
  escapeHtml,
  type Snapshot,
  type Travel,
} from "@/lib/data";
export type MapMode = "current" | "scenario" | "difference";
let geometry: Promise<void> | undefined;
function loadMaps() {
  geometry ??= fetch("/data/zsj.geojson")
    .then(async (response) => {
      if (!response.ok) throw Error("Chybí mapová data");
      echarts.registerMap("zsj", await response.json());
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
  const [failed, setFailed] = useState(false);
  const [retry, setRetry] = useState(0);
  const [schools, setSchools] = useState(true);
  const [employers, setEmployers] = useState(true);
  const [listOpen, setListOpen] = useState(false);
  const chartRef = useRef<EChartsType | null>(null);
  const [mapChart, setMapChart] = useState<EChartsType | null>(null);
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
      .then(() => {
        if (active) {
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
          borderColor: "#f8fafb",
          borderWidth: 0.65,
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
        confine: true,
        backgroundColor: "#fff",
        borderColor: "#d6dce0",
        padding: 12,
        textStyle: {
          fontFamily: '"Helvetica Neue", Arial, sans-serif',
          fontSize: 12,
          color: "#24292f",
        },
        formatter: (p) => {
          const zone = data.zsj.find(
            (z) => z.id === (p as { name: string }).name,
          );
          if (!zone) return "";
          return `<strong>${escapeHtml(zone.name)}</strong><br/>${time(times[zone.id])}<br/><span style="color:#606870">Odhad dětí 10–14 let: ${zone.children ?? "bez dat"}</span>`;
        },
      },
      geo: {
        map: "zsj",
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
          borderWidth: 0.65,
        },
        label: { show: false },
      },
      series: [
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
  }, [data, times, before, mode, mapSize, family]);
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
      <details
        className="map-text-alternative"
        open={listOpen}
        onToggle={(e) => setListOpen(e.currentTarget.open)}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            setListOpen(false);
            e.currentTarget.querySelector("summary")?.focus();
          }
        }}
      >
        <summary>
          <ListBulletsIcon size={14} />
          Seznam míst
        </summary>
        <div className="map-text-columns">
          <section>
            <h3>{family ? "Školy s vybraným oborem" : "Všechny školy"}</h3>
            <ul aria-label="Seznam škol">
              {data.schools
                .filter((s) => !family || schoolIds.has(s.id))
                .map((s) => (
                  <li key={s.id}>
                    <button
                      className="button-link"
                      onClick={() => {
                        setListOpen(false);
                        onSelectSchool?.(s.id);
                      }}
                    >
                      {s.shortName}
                    </button>
                    <span>
                      {s.city} ·{" "}
                      {schoolIds.has(s.id)
                        ? "nabízí obor"
                        : "bez vybraného oboru"}
                    </span>
                  </li>
                ))}
            </ul>
          </section>
          <section>
            <h3>Zaměstnavatelé</h3>
            <ul aria-label="Seznam zaměstnavatelů">
              {data.employers
                .filter((e) => e.field === field)
                .map((e) => (
                  <li key={e.id}>
                    <button
                      className="button-link"
                      onClick={() => {
                        setListOpen(false);
                        onSelectEmployer?.(e.id);
                      }}
                    >
                      {e.name}
                    </button>
                    <span>
                      {e.jobs}{" "}
                      {e.jobs === 1 ? "místo" : e.jobs < 5 ? "místa" : "míst"} ·{" "}
                      {e.city}
                    </span>
                  </li>
                ))}
            </ul>
            {!data.employers.some((e) => e.field === field) && (
              <p>{employersNotice ?? "Pro tento obor nemáme pracoviště se známou polohou."}</p>
            )}
          </section>
        </div>
      </details>
      <div className="map-attribution">
        RÚIAN · uložený výpočet OTP · Číslo = počet institucí
      </div>
    </div>
  );
}
