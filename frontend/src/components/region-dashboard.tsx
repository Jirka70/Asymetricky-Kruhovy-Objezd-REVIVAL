"use client";
import { useMemo, useRef, useState } from "react";
import dynamic from "next/dynamic";
import {
  PlusIcon,
  MinusIcon,
  ArrowLeftIcon,
  CaretRightIcon,
  ArrowCounterClockwiseIcon,
  FlaskIcon,
  CheckCircleIcon,
  WarningCircleIcon,
  GraduationCapIcon,
  XIcon,
} from "@phosphor-icons/react";
import { DataProvider } from "./data-provider";
import SearchSelect from "./search-select";
import { EmployerDetail } from "./place-details";
import SchoolInformation from "./school-information";
import { QueryBoundary, QueryStatus, DataSkeleton } from "./query-state";
import { useSelectionData } from "@/lib/use-selection-data";
import { focusSection } from "@/lib/focus";
import {
  DEFAULT_FIELD,
  schoolIds,
  travelTimes,
  within,
  difference,
  number,
  type Snapshot,
  type Change,
} from "@/lib/data";
import type { MapMode } from "./region-map";
const RegionMap = dynamic(() => import("./region-map"), {
  ssr: false,
  loading: () => <DataSkeleton label="Načítání mapy…" />,
});
const DemandPanel = dynamic(() =>
  import("./analytics").then((m) => m.DemandPanel),
);
const AdmissionPanel = dynamic(() =>
  import("./analytics").then((m) => m.AdmissionPanel),
);
const DistributionChart = dynamic(() =>
  import("./analytics").then((m) => m.DistributionChart),
);
type Selection = { kind: "school" | "employer"; id: string } | null;
export default function RegionDashboard() {
  return <DataProvider>{(data) => <Dashboard data={data} />}</DataProvider>;
}
function Dashboard({ data: baseData }: { data: Snapshot }) {
  const [field, setField] = useState(DEFAULT_FIELD),
    [form, setForm] = useState("den"),
    [threshold, setThreshold] = useState(45);
  const [mode, setMode] = useState<MapMode>("current"),
    [changes, setChanges] = useState<Change[]>([]);
  const [candidate, setCandidate] = useState("600009271"),
    [notice, setNotice] = useState("");
  const selection = useSelectionData(baseData, field, form);
  const { data } = selection;
  const [panel, setPanel] = useState<Selection>(null);
  const returnFocus = useRef<HTMLElement | null>(null);
  const [undo, setUndo] = useState<{
    field: string;
    form: string;
    changes: Change[];
    mode: MapMode;
  } | null>(null);
  const fieldName = data.fields.find((f) => f.id === field)?.name ?? field;
  const ids = useMemo(() => schoolIds(data, field, form), [data, field, form]);
  const scenarioIds = useMemo(
    () => schoolIds(data, field, form, changes),
    [data, field, form, changes],
  );
  const before = useMemo(() => travelTimes(data, ids), [data, ids]);
  const after = useMemo(
    () => travelTimes(data, scenarioIds),
    [data, scenarioIds],
  );
  const base = within(data.zsj, before, threshold),
    next = within(data.zsj, after, threshold);
  const better = data.zsj.filter(
    (z) => difference(before[z.id], after[z.id]) < 0,
  ).length;
  const worse = data.zsj.filter(
    (z) => difference(before[z.id], after[z.id]) > 0,
  ).length;
  const hasScenario = changes.length > 0,
    additionalChildren = next.accessible - base.accessible;
  function changeSelection(nextField: string, nextForm: string) {
    if (nextField === field && nextForm === form) return;
    if (changes.length) {
      setUndo({ field, form, changes, mode });
      setNotice(
        "Po změně oboru nebo formy začínáte nový scénář. Předchozí můžete vrátit.",
      );
    } else setNotice("Nabídka škol a mapa byly aktualizovány.");
    setField(nextField);
    setForm(nextForm);
    setChanges([]);
    setMode("current");
    setPanel(null);
  }
  function openPlace(kind: "school" | "employer", id: string) {
    returnFocus.current =
      document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;
    if (kind === "school") setCandidate(id);
    setPanel({ kind, id });
    focusSection("detail-heading");
  }
  function closeDetail() {
    setPanel(null);
    requestAnimationFrame(() => {
      if (
        returnFocus.current?.isConnected &&
        returnFocus.current !== document.body &&
        returnFocus.current.checkVisibility()
      )
        returnFocus.current.focus({ preventScroll: true });
      else focusSection("regional-map-heading");
    });
  }
  function removeChange(id: string) {
    const remaining = changes.filter((c) => c.school !== id);
    setChanges(remaining);
    setMode(remaining.length ? "scenario" : "current");
    setNotice("Změna byla odstraněna ze scénáře.");
  }
  function addChange(action: Change["action"]) {
    const other = changes.filter((c) => c.school !== candidate),
      desired = action === "add";
    const updated =
      ids.has(candidate) === desired
        ? other
        : [...other, { school: candidate, action }];
    setChanges(updated);
    setMode(updated.length ? "scenario" : "current");
    setPanel({ kind: "school", id: candidate });
    setNotice(
      `${action === "add" ? "Přidáno" : "Odebráno"}: ${fieldName}, ${data.schools.find((s) => s.id === candidate)?.shortName}. Mapa a dopady jsou aktualizované.`,
    );
    focusSection("detail-heading");
  }
  function resetScenario() {
    setUndo({ field, form, changes, mode });
    setChanges([]);
    setMode("current");
    setNotice("Scénář byl zrušen. Můžete jej vrátit.");
  }
  function example() {
    setField(DEFAULT_FIELD);
    setForm("den");
    setCandidate("600009271");
    setChanges([{ school: "600009271", action: "add" }]);
    setMode("scenario");
    setPanel({ kind: "school", id: "600009271" });
    setNotice("Ukázkový scénář: nový obor v Žluticích.");
    focusSection("detail-heading");
  }
  const school =
    panel?.kind === "school"
      ? data.schools.find((s) => s.id === panel.id)
      : undefined;
  const employer =
    panel?.kind === "employer"
      ? data.employers.find((e) => e.id === panel.id && e.field === field)
      : undefined;
  const schoolChange = school
    ? changes.find((c) => c.school === school.id)
    : undefined;
  return (
    <>
      <section
        className="analysis-controls"
        aria-label="Nastavení analýzy a simulace"
      >
        <SearchSelect
          label="Obor"
          value={field}
          onChange={(id) => changeSelection(id, form)}
          placeholder="Název nebo kód oboru"
          options={data.fields.map((f) => ({
            value: f.id,
            label: `${f.name} · ${f.id}`,
          }))}
        />
        <label className="field">
          <span>Forma studia</span>
          <select
            value={form}
            onChange={(e) => changeSelection(field, e.target.value)}
          >
            <option value="den">Denní</option>
            <option value="dal">Dálková</option>
          </select>
        </label>
        <label className="field">
          <span>Hranice dostupnosti</span>
          <select
            value={threshold}
            onChange={(e) => setThreshold(+e.target.value)}
            aria-describedby="threshold-hint"
          >
            {[30, 45, 60].map((t) => (
              <option key={t} value={t}>
                Do {t} min
              </option>
            ))}
          </select>
        </label>
        <SearchSelect
          label="Škola pro změnu nabídky"
          value={candidate}
          onChange={(id) => openPlace("school", id)}
          options={data.schools.map((s) => ({
            value: s.id,
            label: `${s.shortName} · ${s.city}`,
          }))}
        />
        <div className="compact-scenario-actions">
          <button
            className="primary"
            disabled={!selection.schools.data || scenarioIds.has(candidate)}
            onClick={() => addChange("add")}
          >
            <PlusIcon size={17} />
            Přidat obor
          </button>
          <button
            disabled={!selection.schools.data || !scenarioIds.has(candidate)}
            onClick={() => addChange("remove")}
          >
            <MinusIcon size={17} />
            Odebrat
          </button>
        </div>
      </section>
      <div className="workspace-caption">
        <span>2026/27 · Po 12. 10. 2026 · příjezdy 7:00–8:00</span>
        <span id="threshold-hint">
          Limit určuje podíl dětí ve statistikách.
        </span>
      </div>
      <p className="sr-only" role="status" aria-atomic="true">
        {notice}
      </p>
      {undo && (
        <div className="compact-undo">
          <span>Předchozí scénář je uložený.</span>
          <button
            className="button-link"
            onClick={() => {
              setField(undo.field);
              setForm(undo.form);
              setChanges(undo.changes);
              setMode(undo.mode);
              setUndo(null);
              setPanel(null);
              setNotice("Předchozí scénář byl vrácen.");
            }}
          >
            <ArrowCounterClockwiseIcon size={14} />
            Vrátit předchozí scénář
          </button>
          <button aria-label="Zavřít upozornění" onClick={() => setUndo(null)}>
            <XIcon size={14} />
          </button>
        </div>
      )}
      <QueryStatus queries={selection.schools.data ? [selection.schools, selection.employers] : [selection.employers]} />
      <QueryBoundary queries={[selection.schools]}>
      <div
        className={`map-workspace region-workspace${panel ? " has-detail" : ""}`}
      >
        <section
          className="workspace-map"
          aria-labelledby="regional-map-heading"
        >
          <h2 className="sr-only" id="regional-map-heading" tabIndex={-1}>
            Dostupnost oboru na mapě
          </h2>
          <RegionMap
            data={data}
            times={mode === "current" ? before : after}
            before={before}
            schoolIds={mode === "current" ? ids : scenarioIds}
            field={field}
            employersNotice={selection.employersNotice}
            onSelectSchool={(id) => openPlace("school", id)}
            onSelectEmployer={(id) => openPlace("employer", id)}
            selectedEmployerId={
              panel?.kind === "employer" ? panel.id : undefined
            }
            selectedSchoolId={school?.id}
            mode={mode}
            onModeChange={setMode}
            hasScenario={hasScenario}
          />
        </section>
        {panel && (
          <aside
            className="workspace-detail region-detail"
            aria-label="Detail vybraného místa"
            onKeyDown={(e) => {
              if (e.key === "Escape") closeDetail();
            }}
          >
            <div className="detail-navigation">
              <button className="button-link" onClick={closeDetail}>
                <ArrowLeftIcon size={16} />
                Zpět na mapu
              </button>
              <button className="button-link" onClick={closeDetail}>
                Zavřít detail
                <CaretRightIcon size={16} />
              </button>
            </div>
            {school && (
              <div className="place-detail-content">
                <div className="place-title">
                  <span className="place-symbol school-symbol">
                    <GraduationCapIcon size={22} weight="fill" />
                  </span>
                  <div>
                    <h2 id="detail-heading" tabIndex={-1}>
                      {school.shortName}
                    </h2>
                    <p>{school.city}</p>
                  </div>
                </div>
                <p className="detail-program">
                  {fieldName}
                  <small>
                    {field} · {form === "den" ? "Denní" : "Dálkové"} studium
                  </small>
                </p>
                <p
                  className={`offering-status ${ids.has(school.id) ? "offered" : "absent"}`}
                >
                  {ids.has(school.id) ? (
                    <CheckCircleIcon size={18} weight="fill" />
                  ) : (
                    <WarningCircleIcon size={18} weight="fill" />
                  )}
                  Současná nabídka:{" "}
                  {ids.has(school.id) ? "obor je nabízen" : "obor chybí"}
                </p>
                {schoolChange && (
                  <p
                    className={`offering-status ${schoolChange.action === "add" ? "offered" : "absent"}`}
                  >
                    {schoolChange.action === "add" ? (
                      <CheckCircleIcon size={18} weight="fill" />
                    ) : (
                      <MinusIcon size={18} />
                    )}
                    Ve scénáři: obor{" "}
                    {schoolChange.action === "add" ? "přidán" : "odebrán"}
                  </p>
                )}
                <p className="detail-note">
                  Změna platí pouze pro tento scénář.
                </p>
                <div className="detail-actions">
                  {schoolChange ? (
                    <button onClick={() => removeChange(school.id)}>
                      <ArrowCounterClockwiseIcon size={16} />
                      Vrátit{" "}
                      {schoolChange.action === "add"
                        ? "přidání"
                        : "odebrání"}{" "}
                      oboru
                    </button>
                  ) : (
                    <button
                      className={ids.has(school.id) ? "" : "primary"}
                      onClick={() =>
                        addChange(ids.has(school.id) ? "remove" : "add")
                      }
                    >
                      {ids.has(school.id) ? (
                        <MinusIcon size={16} />
                      ) : (
                        <PlusIcon size={16} />
                      )}{" "}
                      {ids.has(school.id)
                        ? "Odebrat obor ze simulované nabídky"
                        : "Přidat obor do simulace"}
                    </button>
                  )}
                </div>
                {hasScenario && (
                  <section className="detail-section">
                    <h3>Dopad celého scénáře</h3>
                    <div className="scenario-impact">
                      <div>
                        <strong>{better}</strong>
                        <span>částí obcí lépe</span>
                      </div>
                      <div>
                        <strong>{worse}</strong>
                        <span>částí obcí hůře</span>
                      </div>
                      <div>
                        <span>Počet škol</span>
                        <strong>
                          {ids.size} → {scenarioIds.size}
                        </strong>
                      </div>
                    </div>
                  </section>
                )}
                <details className="technical-details">
                  <summary>Informace o škole</summary>
                  <SchoolInformation redizo={school.id} field={field} form={form} />
                </details>
              </div>
            )}
            {employer && (
              <EmployerDetail employer={employer} fieldName={fieldName} />
            )}
          </aside>
        )}
      </div>
      <div className="regional-statistics">
        <section
          className="coverage-summary"
          aria-labelledby="regional-result-heading"
        >
          <div>
            <h2 id="regional-result-heading">
              {hasScenario ? (
                <>
                  <span>
                    {Math.round(base.percent ?? 0)} % →{" "}
                    {Math.round(next.percent ?? 0)} %
                  </span>
                  <small>dětí má školu do {threshold} minut</small>
                </>
              ) : (
                <>
                  {Math.round(base.percent ?? 0)} % dětí má školu do {threshold}{" "}
                  minut
                </>
              )}
            </h2>
            {hasScenario && (
              <p
                className={`coverage-gain${additionalChildren < 0 ? " negative" : ""}`}
              >
                {additionalChildren > 0 ? "+" : ""}
                {number(additionalChildren)} dětí v dosahu
              </p>
            )}
            <p className="data-note">
              {number(base.accessible)}
              {hasScenario && ` → ${number(next.accessible)}`} z{" "}
              {number(base.total)} dětí (10–14 let)
              {!hasScenario && ` · ${ids.size} školy`}
            </p>
            <button
              className="button-link"
              onClick={hasScenario ? resetScenario : example}
            >
              {hasScenario ? (
                <ArrowCounterClockwiseIcon size={14} />
              ) : (
                <FlaskIcon size={14} />
              )}{" "}
              {hasScenario ? "Zrušit scénář" : "Ukázkový scénář"}
            </button>
          </div>
          {hasScenario && (
            <div className="changes-summary">
              <strong>
                {changes.length}{" "}
                {changes.length === 1 ? "změna v nabídce" : "změny v nabídce"}
              </strong>
              {changes.map((c) => (
                <div key={c.school}>
                  <button
                    className="button-link"
                    onClick={() => openPlace("school", c.school)}
                  >
                    {c.action === "add" ? (
                      <PlusIcon size={13} />
                    ) : (
                      <MinusIcon size={13} />
                    )}{" "}
                    {data.schools.find((s) => s.id === c.school)?.shortName}
                  </button>
                  <button
                    className="remove-change"
                    aria-label={`Vrátit změnu ${data.schools.find((s) => s.id === c.school)?.shortName}`}
                    onClick={() => removeChange(c.school)}
                  >
                    <XIcon size={13} />
                  </button>
                </div>
              ))}
              <small>Modelová simulace</small>
            </div>
          )}
        </section>
        <section className="distribution-summary">
          <h2>Kolik dětí má školu v dosahu?</h2>
          <DistributionChart
            compact
            zones={data.zsj}
            before={before}
            after={hasScenario ? after : undefined}
          />
          <p className="data-note">
            Odhad dětí 10–14 let · SLDB 2021. Podíly podle dětí, nikoli rozlohy.
          </p>
        </section>
      </div>
      <div className="analytics-grid compact-analytics regional-analytics">
        <DemandPanel
          data={data}
          field={field}
          onSelect={(id) => changeSelection(id, form)}
        />
        <AdmissionPanel data={data} ids={ids} />
      </div>
      </QueryBoundary>
    </>
  );
}
