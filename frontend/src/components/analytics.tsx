"use client";
import type { EChartsOption } from "echarts";
import { ArrowRightIcon, InfoIcon } from "@phosphor-icons/react";
import Chart from "./chart";
import {
  distribution,
  exampleAdmission,
  number,
  TIME_COLORS,
  TIME_LABELS,
  type Snapshot,
  type Travel,
  type Zone,
  escapeHtml,
} from "@/lib/data";
const gridColor = "#e8ece5";
const xAxis = {
  type: "value" as const,
  splitNumber: 3,
  axisLabel: { fontSize: 11, color: "#606870", hideOverlap: true },
  axisLine: { show: false },
  axisTick: { show: false },
  splitLine: { lineStyle: { color: gridColor } },
};
export function ChartLegend() {
  return (
    <div className="chart-legend">
      {TIME_LABELS.map((l, i) => (
        <span key={l}>
          <i style={{ background: TIME_COLORS[i] }} />
          {l}
        </span>
      ))}
    </div>
  );
}
export function DistributionChart({
  zones,
  before,
  after,
  compact = false,
}: {
  zones: Zone[];
  before: Travel;
  after?: Travel;
  compact?: boolean;
}) {
  const rows = [
    distribution(zones, before),
    ...(after ? [distribution(zones, after)] : []),
  ];
  const labels = after ? ["Současný stav", "Po simulaci"] : ["Podíl dětí"];
  const option: EChartsOption = {
    grid: { left: after ? 95 : 8, right: 8, top: 12, bottom: 27 },
    tooltip: {
      trigger: "item",
      confine: true,
      valueFormatter: (v) => `${Number(v).toFixed(1)} %`,
    },
    xAxis: {
      ...xAxis,
      max: 100,
      interval: 25,
      axisLabel: { ...xAxis.axisLabel, formatter: "{value} %" },
    },
    yAxis: {
      type: "category",
      inverse: true,
      data: labels,
      show: !!after,
      axisTick: { show: false },
      axisLine: { show: false },
      axisLabel: { fontSize: 11 },
    },
    series: TIME_LABELS.map((name, i) => ({
      name,
      type: "bar",
      stack: "total",
      barWidth: compact ? 14 : 26,
      data: rows.map((r) => +r.percentages[i].toFixed(2)),
      itemStyle: { color: TIME_COLORS[i] },
      label: {
        show: true,
        position: "inside",
        color: i === 0 ? "#fff" : "#334537",
        fontSize: 11,
        formatter: (p) =>
          Number(p.value) >= 7 ? `${Math.round(Number(p.value))} %` : "",
      },
    })),
  };
  return (
    <>
      <Chart
        option={option}
        height={compact ? (after ? 72 : 66) : after ? 115 : 86}
        label={`Rozložení dojezdů: ${rows.map((r) => r.percentages.map((v, i) => `${TIME_LABELS[i]} ${Math.round(v)} %`).join(", ")).join("; po simulaci ")}`}
      />
      <ChartLegend />
      {rows[0].total === 0 && (
        <p className="data-note">
          Pro toto území není k dispozici odhad počtu dětí.
        </p>
      )}
    </>
  );
}
export function DemandPanel({
  data,
  field,
  onSelect,
}: {
  data: Snapshot;
  field: string;
  onSelect?: (id: string) => void;
}) {
  const ranked = data.demand
    .filter((d) => data.fields.some((f) => f.id === d.field))
    .slice(0, 4);
  const option: EChartsOption = {
    grid: { left: 145, right: 36, top: 8, bottom: 25 },
    xAxis: { ...xAxis },
    yAxis: {
      type: "category",
      inverse: true,
      data: ranked.map(
        (d) => data.fields.find((f) => f.id === d.field)?.name ?? d.field,
      ),
      axisLine: { show: false },
      axisTick: { show: false },
      axisLabel: {
        fontSize: 11,
        color: "#23342e",
        width: 132,
        overflow: "truncate",
      },
    },
    tooltip: {
      trigger: "item",
      confine: true,
      formatter: (p) => {
        const v = p as { dataIndex: number };
        const d = ranked[v.dataIndex];
        return `${escapeHtml(data.fields.find((f) => f.id === d.field)?.name ?? "")}<br/>${d.jobs} míst · ${d.employers} zaměstnavatelů`;
      },
    },
    series: [
      {
        type: "bar",
        barWidth: 15,
        data: ranked.map((d) => ({
          value: d.jobs,
          itemStyle: { color: d.field === field ? "#174c3c" : "#639786" },
        })),
        label: {
          show: true,
          position: "right",
          fontSize: 11,
          color: "#23342e",
        },
      },
    ],
  };
  return (
    <section className="section demand-panel">
      <div className="section-title">
        <div>
          <p className="eyebrow">POPTÁVKA V REGIONU</p>
          <h2>Po kterých oborech je poptávka?</h2>
        </div>
        <span className="data-badge">Pracovní místa</span>
      </div>
      <p className="section-subtitle">
        Podívejte se, kolik míst zaměstnavatelé nabízejí v příbuzných profesích.
      </p>
      {ranked.length > 0 ? <Chart
        option={option}
        height={164}
        label={ranked
          .map(
            (d) =>
              `${data.fields.find((f) => f.id === d.field)?.name}: ${d.jobs} míst`,
          )
          .join(", ")}
        onClick={
          onSelect ? (e) => onSelect(ranked[e.dataIndex].field) : undefined
        }
      /> : <p className="empty-state">Pro nabízené obory nemáme údaje o pracovní poptávce.</p>}
      {onSelect && ranked.length > 0 && (
        <button
          className="button-link"
          onClick={() => onSelect(ranked[0].field)}
        >
          Vybrat nejžádanější obor <ArrowRightIcon size={16} />
        </button>
      )}
      <p className="data-note">
        Propojení oborů s profesemi je orientační. Stejná nabídka může souviset
        s více obory, proto počty nesčítáme.
      </p>
    </section>
  );
}
export const admissionExamples = [
  {
    field: "23-51-E/01",
    name: "Strojírenské práce",
    accepted: 84,
    applications: 100,
  },
  { field: "33-56-H/01", name: "Truhlář", accepted: 76, applications: 100 },
  { field: "26-51-H/01", name: "Elektrikář", accepted: 68, applications: 100 },
];
export function AdmissionPanel({
  data,
  ids,
  onSelect,
}: {
  data: Snapshot;
  ids?: Set<string>;
  onSelect?: (id: string) => void;
}) {
  const rows = ids
    ? data.schools
        .filter((s) => ids.has(s.id))
        .slice(0, 5)
        .map((s) => ({
          id: s.id,
          name: s.shortName,
          ...exampleAdmission(s.id),
        }))
    : admissionExamples.map((a) => ({ ...a, id: a.field }));
  rows.sort(
    (a, b) => b.accepted / b.applications - a.accepted / a.applications,
  );
  const option: EChartsOption = {
    grid: { left: 145, right: 48, top: 8, bottom: 24 },
    xAxis: {
      ...xAxis,
      min: 0,
      max: 100,
      interval: 25,
      axisLabel: { ...xAxis.axisLabel, formatter: "{value} %" },
    },
    yAxis: {
      type: "category",
      inverse: true,
      data: rows.map((r) => r.name),
      axisTick: { show: false },
      axisLine: { show: false },
      axisLabel: {
        color: "#23342e",
        fontSize: 11,
        width: 135,
        overflow: "truncate",
      },
    },
    tooltip: {
      trigger: "item",
      confine: true,
      formatter: (p) => {
        const r = rows[(p as { dataIndex: number }).dataIndex];
        return `${escapeHtml(r.name)}<br/>Model: ${r.accepted} přijatých / ${r.applications} přihlášek`;
      },
    },
    series: [
      {
        type: "scatter",
        symbolSize: 9,
        itemStyle: { color: "#23634f", opacity: 1 },
        data: rows.map((r, i) => [(r.accepted / r.applications) * 100, i]),
        label: {
          show: true,
          position: "right",
          distance: 9,
          color: "#23342e",
          fontSize: 11,
          formatter: (p) =>
            `${Math.round((rows[p.dataIndex].accepted / rows[p.dataIndex].applications) * 100)} %`,
        },
      },
    ],
  };
  return (
    <section className="section admissions-panel">
      <div className="section-title">
        <div>
          <p className="eyebrow">PŘIJÍMACÍ ŘÍZENÍ</p>
          <h2>
            {ids
              ? "Podíl přijatých na školách"
              : "Podíl přijatých podle oborů · ukázka"}
          </h2>
        </div>
        <span className="data-badge example">Modelová ukázka</span>
      </div>
      <p className="section-subtitle">
        Porovnejte podíly přijatých z přihlášek. Čísla jsou zatím jen ukázková,
        při výběru oboru se jimi neřiďte.
      </p>
      {rows.length ? (
        <Chart
          option={option}
          height={164}
          label={`Ilustrační podíl přijatých: ${rows.map((r) => `${r.name} ${r.accepted}/${r.applications}`).join(", ")}`}
          onClick={onSelect ? (e) => onSelect(rows[e.dataIndex].id) : undefined}
        />
      ) : (
        <div className="empty-state">
          Pro tento obor a formu nejsou v nabídce školy.
        </div>
      )}
      {onSelect && (
        <div className="admission-picks">
          {rows.map((r) => (
            <button key={r.id} onClick={() => onSelect(r.id)}>
              {r.name} <ArrowRightIcon size={13} />
            </button>
          ))}
        </div>
      )}
      <p className="data-note">
        <InfoIcon size={14} />
        Počty přijatých v datech chybí. Ukázka není odhadem osobní šance na
        přijetí.
      </p>
    </section>
  );
}
export function ImpactChart({
  data,
  before,
  after,
}: {
  data: Snapshot;
  before: Travel;
  after: Travel;
}) {
  const changed = data.zsj
    .filter((z) => (before[z.id] ?? Infinity) !== (after[z.id] ?? Infinity))
    .sort(
      (a, b) =>
        Math.abs((before[b.id] ?? 240) - (after[b.id] ?? 240)) -
        Math.abs((before[a.id] ?? 240) - (after[a.id] ?? 240)),
    );
  const rows = changed
    .filter((z) => before[z.id] !== null && after[z.id] !== null)
    .slice(0, 6);
  const gained = changed.filter((z) => before[z.id] === null).length;
  const lost = changed.filter((z) => after[z.id] === null).length;
  const max =
    Math.ceil(
      Math.max(
        60,
        ...rows.flatMap((z) => [before[z.id] ?? 0, after[z.id] ?? 0]),
      ) / 30,
    ) * 30;
  const option: EChartsOption = {
    grid: { left: 150, right: 35, top: 10, bottom: 28 },
    xAxis: {
      ...xAxis,
      max,
      name: "min",
      nameTextStyle: { fontSize: 10 },
      axisLabel: { ...xAxis.axisLabel },
    },
    yAxis: {
      type: "category",
      inverse: true,
      data: rows.map((z) => z.name),
      axisTick: { show: false },
      axisLine: { show: false },
      axisLabel: {
        fontSize: 11,
        color: "#23342e",
        width: 135,
        overflow: "truncate",
      },
    },
    series: [
      ...rows
        .filter((z) => before[z.id] != null && after[z.id] != null)
        .map((z) => ({
          type: "line" as const,
          symbol: "none",
          silent: true,
          lineStyle: { color: "#bbc7bb", width: 2 },
          data: [
            [before[z.id], rows.indexOf(z)],
            [after[z.id], rows.indexOf(z)],
          ],
        })),
      {
        type: "scatter",
        name: "Před",
        symbolSize: 8,
        itemStyle: { color: "#909e94", opacity: 1 },
        data: rows.map((z, i) =>
          before[z.id] != null ? [before[z.id], i] : null,
        ),
        label: {
          show: true,
          position: "top",
          fontSize: 11,
          color: "#6f7b70",
          formatter: (p) => String(Math.ceil(Number((p.value as number[])[0]))),
        },
      },
      {
        type: "scatter",
        name: "Po",
        symbolSize: 9,
        data: rows.map((z, i) =>
          after[z.id] != null
            ? {
                value: [after[z.id], i],
                itemStyle: {
                  color:
                    (after[z.id] ?? Infinity) < (before[z.id] ?? Infinity)
                      ? "#23634f"
                      : "#bb753d",
                  opacity: 1,
                },
              }
            : null,
        ),
        label: {
          show: true,
          position: "bottom",
          fontSize: 11,
          color: "#23342e",
          formatter: (p) => String(Math.ceil(Number((p.value as number[])[0]))),
        },
      },
    ],
  };
  return (
    <section className="section impact-panel">
      <div className="section-title">
        <div>
          <p className="eyebrow">DOPADY SCÉNÁŘE</p>
          <h2>Komu se cesta do školy změní?</h2>
        </div>
        <div className="mini-legend">
          <span>
            <i style={{ background: "#909e94" }} />
            Před
          </span>
          <span>
            <i style={{ background: "#23634f" }} />
            Po
          </span>
          <span>
            <i style={{ background: "#bb753d" }} />
            Zhoršení
          </span>
        </div>
      </div>
      {rows.length ? (
        <>
          <Chart
            option={option}
            height={230}
            label={`Největší změny dojezdů: ${rows.map((z) => `${z.name}: ${before[z.id] ?? "bez spojení"} → ${after[z.id] ?? "bez spojení"}`).join("; ")}`}
          />
          <p className="data-note">
            {rows.length} největších změn mezi cestami dostupnými před i po
            simulaci.
          </p>
        </>
      ) : (
        <div className="empty-state">
          {changed.length
            ? "Změny se týkají pouze vzniku nebo zániku spojení, nikoli změny času existující cesty."
            : "Přidejte změnu do scénáře. Zde uvidíte, komu se cesta zkrátí nebo prodlouží."}
        </div>
      )}
      {changed.length > 0 && (
        <div className="impact-reachability">
          <span>
            <strong>{number(changed.length)}</strong> změněných ZSJ celkem
          </span>
          <span>
            <strong>{gained}</strong> nově se spojením
          </span>
          <span>
            <strong>{lost}</strong> nově bez spojení
          </span>
        </div>
      )}
    </section>
  );
}
