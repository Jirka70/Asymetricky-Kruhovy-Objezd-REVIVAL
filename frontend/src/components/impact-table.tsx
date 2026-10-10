"use client";
import { useState } from "react";
import Chart from "./chart";
import { difference, time, type Snapshot, type Travel } from "@/lib/data";

export default function ImpactTable({
  data,
  before,
  after,
  hasScenario,
  onSelect,
}: {
  data: Snapshot;
  before: Travel;
  after: Travel;
  hasScenario: boolean;
  onSelect: (municipality: string) => void;
}) {
  const [limit, setLimit] = useState(3);
  const gained = data.zsj.filter(
    (z) => before[z.id] == null && after[z.id] != null,
  );
  const lost = data.zsj.filter(
    (z) => before[z.id] != null && after[z.id] == null,
  );
  const missing = data.zsj.filter(
    (z) => (z.children ?? 0) > 0 && before[z.id] == null,
  );
  const rows = data.zsj
    .filter((z) =>
      hasScenario
        ? before[z.id] != null &&
          after[z.id] != null &&
          difference(before[z.id], after[z.id]) !== 0
        : (z.children ?? 0) > 0 && before[z.id] != null,
    )
    .sort((a, b) => {
      const av = hasScenario
        ? Math.abs(difference(before[a.id], after[a.id]))
        : (before[a.id] ?? Infinity);
      const bv = hasScenario
        ? Math.abs(difference(before[b.id], after[b.id]))
        : (before[b.id] ?? Infinity);
      return av === bv ? (b.children ?? 0) - (a.children ?? 0) : av > bv ? -1 : 1;
    });
  const visible = rows.slice(0, limit);
  const maximum = Math.max(
    60,
    ...visible.flatMap((z) => [before[z.id] ?? 0, after[z.id] ?? 0]),
  );
  const scale = Math.ceil(maximum / 30) * 30;
  return (
    <div className="impact-comparison">
      <div className="impact-heading">
        <h3>
          {hasScenario
            ? "Kde se cesta změní nejvíc"
            : "Odkud je cesta do školy nejsložitější"}
        </h3>
        <span>
          {hasScenario ? "Největší změny času" : "Obydlené části obcí"}
        </span>
      </div>
      {visible.length ? (
        <table className="impact-table">
          <caption className="sr-only">
            {hasScenario
              ? "Části obcí s uloženou cestou před i po simulaci, seřazené podle velikosti změny."
              : "Obydlené lokality s uloženou cestou, seřazené od nejdelší cesty."}
          </caption>
          <thead>
            <tr>
              <th scope="col">Část obce (ZSJ)</th>
              <th scope="col">{hasScenario ? "Před" : "Cesta"}</th>
              {hasScenario && (
                <>
                  <th scope="col">Po</th>
                  <th scope="col">Změna</th>
                </>
              )}
              <th scope="col" className="impact-visual">
                0–{scale} min
              </th>
            </tr>
          </thead>
          <tbody>
            {visible.map((z) => {
              const b = before[z.id],
                a = after[z.id],
                delta = difference(b, a);
              const change =
                delta === -Infinity
                  ? "Nové spojení"
                  : delta === Infinity
                    ? "Bez spojení"
                    : `${delta < 0 ? "−" : "+"}${Math.abs(Math.ceil(a!) - Math.ceil(b!))} min`;
              return (
                <tr key={z.id}>
                  <th scope="row">
                    <button
                      onClick={() => onSelect(z.municipality)}
                      title={`Otevřít detail obce ${data.municipalities.find((m) => m.id === z.municipality)?.name}`}
                    >
                      {z.name}
                      <span>›</span>
                    </button>
                  </th>
                  <td>{time(b)}</td>
                  {hasScenario && (
                    <>
                      <td>{time(a)}</td>
                      <td
                        className={delta < 0 ? "impact-better" : "impact-worse"}
                      >
                        {change}
                      </td>
                    </>
                  )}
                  <td className="impact-visual">
                    <Chart
                      height={28}
                      label={
                        hasScenario
                          ? `${z.name}: před ${time(b)}, po ${time(a)}`
                          : `${z.name}: ${time(b)}`
                      }
                      option={{
                        grid: { left: 5, right: 5, top: 3, bottom: 3 },
                        xAxis: {
                          type: "value",
                          min: 0,
                          max: scale,
                          show: false,
                        },
                        yAxis: { type: "value", min: 0, max: 1, show: false },
                        series: [
                          {
                            type: "line",
                            symbol: "none",
                            silent: true,
                            data:
                              hasScenario && b != null && a != null
                                ? [
                                    [b, 0.5],
                                    [a, 0.5],
                                  ]
                                : [],
                            lineStyle: { color: "#a2adb5", width: 2 },
                          },
                          {
                            type: "scatter",
                            symbolSize: 7,
                            silent: true,
                            data: b == null ? [] : [[b, 0.5]],
                            itemStyle: {
                              color: hasScenario ? "#8797a6" : "#23634f",
                            },
                          },
                          {
                            type: "scatter",
                            symbolSize: 8,
                            silent: true,
                            data: hasScenario && a != null ? [[a, 0.5]] : [],
                            itemStyle: {
                              color: delta > 0 ? "#b36730" : "#23634f",
                            },
                          },
                        ],
                      }}
                    />
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      ) : (
        <p className="comparison-note">
          {hasScenario
            ? "Žádné porovnatelné cesty se nezměnily. Změny dostupnosti spojení jsou uvedené níže."
            : "Pro tento obor a formu studia nemáme uložené cesty. Vyzkoušejte jiný obor nebo změňte nabídku škol."}
        </p>
      )}
      <div className="impact-foot">
        <span>
          {hasScenario
            ? "● Šedá: před · zelená: zlepšení · hnědá: zhoršení"
            : "Cesty v uloženém ranním okně"}
        </span>
        {rows.length > 3 && (
          <button
            className="button-link"
            onClick={() => setLimit(limit > 3 ? 3 : 12)}
          >
            {limit > 3
              ? "Zobrazit méně"
              : `Zobrazit ${Math.min(12, rows.length)} lokalit`}
          </button>
        )}
      </div>
      {hasScenario
        ? (gained.length > 0 || lost.length > 0) && (
            <details className="connection-changes">
              <summary>
                Nově se spojením: {gained.length} částí obcí · nově bez spojení:{" "}
                {lost.length}
              </summary>
              <ul>
                {[...gained, ...lost].map((z) => (
                  <li key={z.id}>
                    <button
                      className="button-link"
                      onClick={() => onSelect(z.municipality)}
                    >
                      {z.name}
                    </button>
                    <span>
                      {time(before[z.id])} → {time(after[z.id])}
                    </span>
                  </li>
                ))}
              </ul>
            </details>
          )
        : missing.length > 0 && (
            <details className="connection-changes">
              <summary>
                V dalších {missing.length} obydlených částech obcí chybí uložené
                spojení
              </summary>
              <ul>
                {missing.map((z) => (
                  <li key={z.id}>
                    <button
                      className="button-link"
                      onClick={() => onSelect(z.municipality)}
                    >
                      {z.name}
                    </button>
                    <span>Odhad {z.children} dětí</span>
                  </li>
                ))}
              </ul>
            </details>
          )}
    </div>
  );
}
