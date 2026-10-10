"use client";
import {
  PersonSimpleWalkIcon,
  BusIcon,
  ArrowsLeftRightIcon,
  FlagCheckeredIcon,
  ArrowRightIcon,
} from "@phosphor-icons/react";
import type { EChartsOption } from "echarts";
import Chart from "./chart";
import { clock, minutes, exampleLegs, type Journey } from "@/lib/journeys";
const colors = { walk: "#d9b979", ride: "#75a99a", transfer: "#c9cec5" };
export default function JourneyPanel({
  journeys,
  selected,
  onSelect,
  boundary,
  mode,
  comparisonOnly = false,
}: {
  journeys: Journey[];
  selected: string;
  onSelect: (id: string) => void;
  boundary: string;
  mode: "arrival" | "departure";
  comparisonOnly?: boolean;
}) {
  const rows = journeys.slice(0, 5);
  const chosen = journeys.find((j) => j.school.id === selected) ?? rows[0];
  if (!chosen) return null;
  const min =
    Math.floor(Math.min(...rows.map((j) => minutes(j.departure))) / 30) * 30;
  const max =
    Math.ceil(
      Math.max(...rows.map((j) => minutes(j.arrival)), minutes(boundary)) / 30,
    ) * 30;
  const chartData = rows.flatMap((j, row) =>
    exampleLegs(j).map((leg) => ({
      value: [row, leg.start, leg.end],
      name: leg.label,
      itemStyle: { color: colors[leg.kind] },
      id: j.school.id,
    })),
  );
  const option: EChartsOption = {
    grid: { left: 128, right: 23, top: 35, bottom: 18 },
    tooltip: {
      trigger: "item",
      confine: true,
      formatter: (p) => {
        const v = p as { value: number[]; name: string };
        return `Modelový úsek: ${v.name}<br/>${clock(v.value[1])}–${clock(v.value[2])}`;
      },
    },
    xAxis: {
      type: "value",
      min,
      max,
      interval: max - min > 240 ? 60 : 30,
      position: "top",
      axisLabel: { formatter: (v) => clock(v), fontSize: 11, color: "#6c7b6d" },
      axisLine: { show: false },
      axisTick: { show: false },
      splitLine: { lineStyle: { color: "#e7eae2" } },
    },
    yAxis: {
      type: "category",
      inverse: true,
      data: rows.map((j) => j.school.shortName),
      axisLine: { show: false },
      axisTick: { show: false },
      axisLabel: {
        color: "#23342e",
        fontSize: 11,
        width: 115,
        overflow: "truncate",
      },
    },
    series: [
      {
        type: "custom",
        data: chartData,
        renderItem: (_params, api) => {
          const y = Number(api.value(0)),
            start = api.coord([api.value(1), y]),
            end = api.coord([api.value(2), y]);
          return {
            type: "rect",
            shape: {
              x: start[0],
              y: start[1] - 10,
              width: Math.max(1, end[0] - start[0] - 1),
              height: 20,
              r: 2,
            },
            style: {
              fill: api.visual("color") as string,
              stroke:
                rows[y].school.id === selected ? "#346552" : "transparent",
              lineWidth: 0.6,
            },
          };
        },
        encode: { x: [1, 2], y: 0 },
      },
      {
        type: "scatter",
        symbolSize: 1,
        silent: true,
        data: rows.map((j, i) => [minutes(j.arrival), i]),
        label: {
          show: true,
          position: "bottom",
          distance: 13,
          formatter: (p) =>
            `${rows[p.dataIndex].departure.slice(0, 5)} → ${rows[p.dataIndex].arrival.slice(0, 5)}`,
          fontSize: 11,
          color: "#606870",
        },
        itemStyle: { opacity: 1, color: "transparent" },
        markLine: {
          symbol: "none",
          silent: true,
          label: {
            show: true,
            formatter:
              mode === "arrival"
                ? `Příchod do ${boundary}`
                : `Odchod od ${boundary}`,
            fontSize: 11,
            color: "#9a6a2c",
          },
          lineStyle: { color: "#b18743", type: "dashed" },
          data: [{ xAxis: minutes(boundary) }],
        },
      },
    ],
  };
  return (
    <div className="journey-layout" id="spojeni">
      <section className="section journey-chart">
        <div className="section-title">
          <div>
            <p className="eyebrow">RANNÍ DOJÍŽDĚNÍ</p>
            <h2>Kdy vyrazit do školy?</h2>
          </div>
          <span className="data-badge example">Úseky: modelová ukázka</span>
        </div>
        <div className="journey-legend">
          <span>
            <PersonSimpleWalkIcon size={16} />
            Chůze
          </span>
          <span>
            <BusIcon size={16} />
            Jízda
          </span>
          <span>
            <ArrowsLeftRightIcon size={16} />
            Přestup
          </span>
        </div>
        <Chart
          option={option}
          height={Math.max(210, rows.length * 70 + 55)}
          label={`Uložené odjezdy a příjezdy, úseky jsou ilustrační: ${rows.map((j) => `${j.school.shortName} ${j.departure.slice(0, 5)}–${j.arrival.slice(0, 5)}`).join("; ")}`}
          onClick={(e) => {
            if (e.data?.id) onSelect(e.data.id);
          }}
        />
        <p className="data-note">
          Odjezdy a příjezdy jsou z uloženého OTP výpočtu. Rozdělení na chůzi,
          jízdu a přestupy je pouze ukázkové; data úseky neobsahují.
        </p>
      </section>
      {!comparisonOnly && (
        <section className="section journey-detail">
          <a className="text-link back-to-search" href="#results-heading">
            Zpět na seznam škol
          </a>
          <p className="eyebrow">VYBRANÁ CESTA</p>
          <h2 id="journey-heading" tabIndex={-1}>
            {chosen.school.shortName}
          </h2>
          <p className="section-subtitle">
            {Math.ceil(chosen.duration)} minut celkem{" "}
            <span className="dot-separator">·</span> úseky modelové
          </p>
          <ol className="itinerary">
            {exampleLegs(chosen).map((leg, i) => (
              <li key={i}>
                <time>{clock(leg.start)}</time>
                <span className={`leg-dot ${leg.kind}`} />
                <span className="leg-icon">
                  {leg.kind === "walk" ? (
                    <PersonSimpleWalkIcon size={18} />
                  ) : leg.kind === "ride" ? (
                    <BusIcon size={18} />
                  ) : (
                    <ArrowsLeftRightIcon size={18} />
                  )}
                </span>
                <div>
                  <b>{leg.label}</b>
                  <span>{Math.round(leg.end - leg.start)} min · ukázka</span>
                </div>
              </li>
            ))}
            <li>
              <time>{chosen.arrival.slice(0, 5)}</time>
              <span className="leg-dot arrival" />
              <FlagCheckeredIcon size={18} />
              <div>
                <b>Příchod ke škole</b>
              </div>
            </li>
          </ol>
          <div className="arrival-note">
            {mode === "arrival" ? (
              <>
                Rezerva do {boundary}
                <strong>
                  {Math.floor(minutes(boundary) - minutes(chosen.arrival))} min
                </strong>
              </>
            ) : (
              <>
                Odchod ze ZSJ<strong>{chosen.departure.slice(0, 5)}</strong>
              </>
            )}
            <ArrowRightIcon size={16} />
          </div>
        </section>
      )}
    </div>
  );
}
