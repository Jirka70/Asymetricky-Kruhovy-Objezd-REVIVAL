"use client";
import { useState } from "react";
import { ArrowRightIcon } from "@phosphor-icons/react";
import LocalMap from "./local-map";
import SearchSelect from "./search-select";
import { DistributionChart } from "./analytics";
import {
  number,
  time,
  within,
  difference,
  type Snapshot,
  type Travel,
} from "@/lib/data";
export default function MunicipalityDetail({
  data,
  id,
  onChange,
  before,
  after,
  threshold = 45,
}: {
  data: Snapshot;
  id: string;
  onChange: (id: string) => void;
  before: Travel;
  after?: Travel;
  threshold?: number;
}) {
  const [expanded, setExpanded] = useState(false);
  const municipality = data.municipalities.find((m) => m.id === id)!;
  const zones = data.zsj
    .filter((z) => z.municipality === id)
    .sort((a, b) => (b.children ?? 0) - (a.children ?? 0));
  const base = within(zones, before, threshold);
  const next = after ? within(zones, after, threshold) : base;
  return (
    <section className="detail-section">
      <div className="detail-header">
        <div>
          <p className="eyebrow">PODROBNĚ / {municipality.name}</p>
          <h2>Odkud trvá cesta do školy nejdéle?</h2>
          <p className="section-subtitle">
            Porovnejte jednotlivé části obce a zjistěte, kde by kratší
            dojíždění pomohlo.
          </p>
        </div>
        <SearchSelect
          className="compact"
          label="Vybraná obec"
          value={id}
          onChange={(value) => {
            onChange(value);
            setExpanded(false);
          }}
          options={[...data.municipalities]
            .sort((a, b) => a.name.localeCompare(b.name, "cs"))
            .map((m) => ({ value: m.id, label: `${m.name} · ${m.id}` }))}
        />
      </div>
      <div className="municipality-grid">
        <LocalMap id={id} zones={zones} times={after ?? before} />
        <div className="local-metric">
          <span>Do {threshold} minut se dostane</span>
          <strong>
            {base.percent === null ? "—" : Math.round(base.percent)}
            <small> %</small>
            {after && (
              <>
                <ArrowRightIcon size={21} />
                {next.percent === null ? "—" : Math.round(next.percent)}
                <small> %</small>
              </>
            )}
          </strong>
          <p>z {number(base.total)} odhadovaných dětí</p>
          <div className="local-count">
            <b>{zones.length}</b> ZSJ v obci
          </div>
        </div>
        <div>
          <h3>Jak dlouho trvá cesta do školy</h3>
          <DistributionChart zones={zones} before={before} after={after} />
        </div>
      </div>
      <div className="table-scroll">
        <table className="data-table">
          <thead>
            <tr>
              <th>Část obce (ZSJ)</th>
              <th>Odhad dětí</th>
              <th>Současný dojezd</th>
              {after && (
                <>
                  <th>Po simulaci</th>
                  <th>Změna</th>
                </>
              )}
            </tr>
          </thead>
          <tbody>
            {(expanded ? zones : zones.slice(0, 4)).map((z) => {
              const d = difference(before[z.id], after?.[z.id] ?? null);
              return (
                <tr key={z.id}>
                  <td>{z.name}</td>
                  <td>{z.children}</td>
                  <td>{time(before[z.id])}</td>
                  {after && (
                    <>
                      <td className="strong">{time(after[z.id])}</td>
                      <td
                        className={
                          d < 0 ? "positive" : d > 0 ? "negative" : "muted"
                        }
                      >
                        {d === 0
                          ? "—"
                          : !isFinite(d)
                            ? d < 0
                              ? "Nové spojení"
                              : "Bez spojení"
                            : Math.ceil(after[z.id]!) ===
                                Math.ceil(before[z.id]!)
                              ? "Pod 1 min"
                              : `${d > 0 ? "+" : "−"}${Math.abs(Math.ceil(after[z.id]!) - Math.ceil(before[z.id]!))} min`}
                      </td>
                    </>
                  )}
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
      {zones.length > 4 && (
        <button
          className="button-link"
          aria-expanded={expanded}
          onClick={() => setExpanded(!expanded)}
        >
          {expanded ? "Zobrazit méně" : `Zobrazit všech ${zones.length} ZSJ`}{" "}
          <ArrowRightIcon size={14} />
        </button>
      )}
      <p className="data-note">
        Vážené odhadem počtu dětí 10–14 let (SLDB 2021), nikoli rozlohou území.
        Časy jsou zaokrouhlené nahoru; změna odpovídá rozdílu zobrazených minut.
      </p>
    </section>
  );
}
