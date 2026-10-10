"use client";
import { useMemo, useRef, useState } from "react";
import dynamic from "next/dynamic";
import { useQuery } from "@tanstack/react-query";
import { accessibilityQuery, simulationQuery, batchSimulationQuery, type SimulationResponse, type RemovalResponse } from "@/lib/api";
import ProgramDetail from "./program-detail";
import { accessibilityData } from "@/lib/accessibility-data";
import { simulationData } from "@/lib/simulation-data";
import {
  PlusIcon,
  MinusIcon,
  ArrowLeftIcon,
  CaretRightIcon,
  ArrowCounterClockwiseIcon,
  CheckCircleIcon,
  WarningCircleIcon,
  GraduationCapIcon,
  XIcon,
} from "@phosphor-icons/react";
import { DataProvider } from "./data-provider";
import SearchSelect from "./search-select";
import { EmployerDetail } from "./place-details";
import SchoolInformation, { SchoolAdmissions } from "./school-information";
import { QueryBoundary, QueryStatus, DataSkeleton } from "./query-state";
import { useSelectionData } from "@/lib/use-selection-data";
import { focusSection } from "@/lib/focus";
import {
  DEFAULT_FIELD,
  schoolIds,
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

const DistributionChart = dynamic(() =>
  import("./analytics").then((m) => m.DistributionChart),
);
type Selection = { kind: "school" | "employer"; id: string } | null;
type ScenarioChanges = (Change & { capacity?: string })[];
export default function RegionDashboard() {
  return <DataProvider>{(data) => <Dashboard data={data} />}</DataProvider>;
}
function Dashboard({ data: baseData }: { data: Snapshot }) {
  const [field, setField] = useState(DEFAULT_FIELD),
    [form, setForm] = useState("den"),
    [threshold, setThreshold] = useState(45);
  const [mode, setMode] = useState<MapMode>("current"),
    [changes, setChanges] = useState<ScenarioChanges>([]);
  const [candidate, setCandidate] = useState("600009271"),
    [notice, setNotice] = useState("");
  const [capacity, setCapacity] = useState("30");
  const validCapacity = Number.isInteger(Number(capacity)) && Number(capacity) >= 1 && Number(capacity) <= 300;
  const addition = changes.length === 1 && changes[0].action === "add";
  const selection = useSelectionData(baseData, field, form);
  const { data } = selection;
  const additionSimulation = useQuery(simulationQuery({
    redizo: addition && form === "den" ? changes[0].school : "",
    obor: field, kapacita: Number(changes[0]?.capacity), max_min: threshold,
  }));
  const batchSimulation = useQuery(batchSimulationQuery({
    obor: field,
    zmeny: !addition && form === "den" ? changes.map(change => ({
      redizo: change.school,
      zmena_kapacity: change.action === "add" ? Number(change.capacity) :
        -(data.offerings.find(offer => offer.school === change.school)?.capacity ?? NaN),
    })) : [],
    max_min: threshold,
  }));
  const simulation = addition ? additionSimulation : batchSimulation;
  const result = useMemo(() => simulation.data?.souhrn ? simulationData(simulation.data) : undefined, [simulation.data]);
  const canRemove = (id: string) => {
    const places = data.offerings.find((offer) => offer.school === id)?.capacity;
    return form === "den" && places != null && Number.isInteger(places) && places >= 1 && places <= 300;
  };
  const scenarioValid = form === "den" && changes.length > 0 && changes.length <= 100 && changes.every(change =>
    change.action === "add" ? Number.isInteger(Number(change.capacity)) && Number(change.capacity) >= 1 && Number(change.capacity) <= 300 : canRemove(change.school));
  const [panel, setPanel] = useState<Selection>(null);
  const returnFocus = useRef<HTMLElement | null>(null);
  const [undo, setUndo] = useState<{
    field: string;
    form: string;
    changes: ScenarioChanges;
    capacity: string;
    mode: MapMode;
  } | null>(null);
  const fieldName = data.fields.find((f) => f.id === field)?.name ?? field;
  const ids = useMemo(() => schoolIds(data, field, form), [data, field, form]);
  const scenarioIds = useMemo(
    () => schoolIds(data, field, form, changes),
    [data, field, form, changes],
  );
  const accessibility = useQuery(accessibilityQuery(field, form, threshold));
  const current = useMemo(() => accessibility.data ? accessibilityData(accessibility.data) : undefined, [accessibility.data]);
  const before = result?.before ?? current?.times ?? {};
  const after = result?.after ?? before;
  const zones = result?.zones ?? current?.zones ?? [];
  const beforeCoverage = result ? within(zones, before, threshold) : current?.coverage ?? within([], {}, threshold);
  const afterCoverage = within(zones, after, threshold);
  const removalSummary = !addition ? batchSimulation.data?.souhrn : undefined;
  // API summaries use unrounded times when deciding whether a ZSJ is reachable.
  const coverage = (value: typeof beforeCoverage, accessible: number | undefined) => accessible === undefined ? value : {
    ...value, accessible, percent: value.total ? accessible / value.total * 100 : null,
  };
  const base = coverage(beforeCoverage, removalSummary?.deti_v_dosahu_pred);
  const next = coverage(afterCoverage, removalSummary?.deti_v_dosahu_po);
  const better = simulation.data?.souhrn?.zlepsenych_jednotek ?? zones.filter(
    (z) => difference(before[z.id], after[z.id]) < 0,
  ).length;
  const worse = removalSummary?.zhorsenych_jednotek ?? zones.filter(
    (z) => difference(before[z.id], after[z.id]) > 0,
  ).length;
  const hasScenario = changes.length > 0 && Boolean(result),
    additionalChildren = next.accessible - base.accessible;
  function changeSelection(nextField: string, nextForm: string) {
    if (nextField === field && nextForm === form) return;
    if (changes.length) {
      setUndo({ field, form, changes, mode, capacity });
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
    if (kind === "school") {
      setCandidate(id);
      const change = changes.find(change => change.school === id);
      if (change?.action === "add") setCapacity(change.capacity ?? "30");
    }
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
    const remaining = changes.filter(change => change.school !== id);
    setChanges(remaining);
    setMode(remaining.length ? "scenario" : "current");
    setNotice("Změna byla odstraněna ze scénáře.");
  }
  function addChange(action: Change["action"]) {
    if (action === "add" && !ids.has(candidate) && (form !== "den" || !validCapacity)) return;
    if (action === "remove" && ids.has(candidate) && !canRemove(candidate)) return;
    const desired = action === "add";
    const other = changes.filter(change => change.school !== candidate);
    if (ids.has(candidate) !== desired && other.length >= 100) {
      setNotice("Scénář může obsahovat nejvýše 100 změn.");
      return;
    }
    const updated: ScenarioChanges = ids.has(candidate) === desired ? other :
      [...other, { school: candidate, action, ...(action === "add" ? { capacity } : {}) }];
    setChanges(updated);
    setMode(updated.length ? "scenario" : "current");
    setPanel({ kind: "school", id: candidate });
    setNotice(
      `${action === "add" ? "Přidáno" : "Odebráno"}: ${fieldName}, ${data.schools.find((s) => s.id === candidate)?.shortName}. Scénář byl změněn.`,
    );
    focusSection("detail-heading");
  }
  function resetScenario() {
    setUndo({ field, form, changes, mode, capacity });
    setChanges([]);
    setMode("current");
    setNotice("Scénář byl zrušen. Můžete jej vrátit.");
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
            <option value={0}>Bez limitu</option>
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
            disabled={!selection.schools.data || scenarioIds.has(candidate) || (!ids.has(candidate) && (form !== "den" || !validCapacity))}
            onClick={() => addChange("add")}
          >
            <PlusIcon size={17} />
            Přidat obor
          </button>
          <button
            disabled={!selection.schools.data || !scenarioIds.has(candidate) || (ids.has(candidate) && !canRemove(candidate))}
            onClick={() => addChange("remove")}
          >
            <MinusIcon size={17} />
            Odebrat
          </button>
        </div>
      </section>
      <div className="simulation-controls">
        <label className="field">
          <span>Kapacita nového oboru</span>
          <input type="number" min={1} max={300} step={1} value={capacity}
            aria-invalid={!validCapacity} aria-describedby="simulation-hint"
            onChange={(event) => {
              const value = event.target.value;
              setCapacity(value);
              setChanges(previous => previous.map(change => change.school === candidate && change.action === "add" ? { ...change, capacity: value } : change));
            }} />
        </label>
        <span id="simulation-hint">1–300 míst v prvním ročníku. Simulace je dostupná pouze pro denní studium.</span>
        {changes.length > 0 && !hasScenario && (
          <button onClick={resetScenario}>Zrušit scénář</button>
        )}
      </div>
      {!changes.length && !validCapacity && <p role="alert">Zadejte celou kapacitu od 1 do 300 míst.</p>}
      <div className="workspace-caption">
        <span>2026/27 · Po 12. 10. 2026 · příjezdy 7:00–8:00</span>
        <span id="threshold-hint">
          Limit určuje podíl dětí ve statistikách.
        </span>
      </div>
      <p className="workspace-caption">
        Ve scénáři lze změnit nabídku až na 100 školách. Kapacitu přidaného oboru upravíte výběrem příslušné školy.
      </p>
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
              setCapacity(undo.capacity);
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
      {accessibility.data && <QueryStatus queries={[accessibility]} />}
      <QueryBoundary queries={[selection.schools, accessibility]}>
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
            times={mode === "current" || !hasScenario ? before : after}
            before={before}
            nearestSchools={mode === "current" || !hasScenario ? current?.nearestSchools : result?.nearestSchools}
            beforeSchools={current?.nearestSchools}
            schoolIds={mode === "current" || !hasScenario ? ids : scenarioIds}
            field={field}
            employersNotice={selection.employersNotice}
            onSelectSchool={(id) => openPlace("school", id)}
            onSelectEmployer={(id) => openPlace("employer", id)}
            selectedEmployerId={
              panel?.kind === "employer" ? panel.id : undefined
            }
            selectedSchoolId={school?.id}
            mode={hasScenario ? mode : "current"}
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
                <SchoolAdmissions redizo={school.id} field={field} form={form} />
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
                    {hasScenario ? "Ve scénáři: obor" : "Požadavek: obor"}{" "}
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
                      disabled={ids.has(school.id) ? !canRemove(school.id) : form !== "den" || !validCapacity}
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
                {ids.has(school.id) && !canRemove(school.id) && <p className="detail-note">Odebrání vyžaduje denní nabídku se známou kapacitou 1–300 míst.</p>}
                {changes.length > 0 && (
                  <section className="detail-section simulation-feedback" aria-label="Výpočet simulace" aria-busy={Boolean(scenarioValid && simulation.isFetching)}>
                    <h3>Dopad celého scénáře</h3>
                    {!scenarioValid ? (
                      <p role="alert">Zadejte celou kapacitu od 1 do 300 míst pro každou změnu. Simulace vyžaduje denní studium. Scénář může obsahovat nejvýše 100 změn.</p>
                    ) : simulation.isFetching ? (
                      <DataSkeleton compact label="Počítám simulaci… Mapa zůstává na posledním zobrazeném stavu." />
                    ) : <>
                    <QueryStatus queries={[simulation]} />
                    {addition && additionSimulation.data?.souhrn === null && (
                      <p role="status" className="query-notice">
                        {additionSimulation.data.meta.duvod === "kapacita_staci"
                          ? "Současná kapacita podle modelu stačí. Simulace neprovedla změnu."
                          : "API nevrátilo vyhodnocení scénáře."}
                      </p>
                    )}
                    {hasScenario && <>
                    {addition && additionSimulation.data?.souhrn && (
                      <SimulationSummary summary={additionSimulation.data.souhrn} />
                    )}
                    {removalSummary && <RemovalSummary summary={removalSummary} multiple={changes.length > 1} />}
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
                    </>}
                    </>}
                  </section>
                )}
                <section className="technical-details">
                  <h3>Informace o škole</h3>
                  <SchoolInformation redizo={school.id} field={field} form={form} showAdmissions={false} />
                </section>
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
                  <small>dětí má školu {threshold === 0 ? "bez časového limitu" : `do ${threshold} minut`}</small>
                </>
              ) : (
                <>
                  {Math.round(base.percent ?? 0)} % dětí má školu {threshold === 0 ? "bez časového limitu" : `do ${threshold} minut`}
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
              {number(base.total)} dětí v odhadovaném ročníku
              {!hasScenario && ` · ${ids.size} školy`}
            </p>
            {hasScenario && (
              <button className="button-link" onClick={resetScenario}>
                <ArrowCounterClockwiseIcon size={14} /> Zrušit scénář
              </button>
            )}
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
              <small>Modelová simulace z API</small>
            </div>
          )}
        </section>
        <section className="distribution-summary">
          <h2>Kolik dětí má školu v dosahu?</h2>
          <DistributionChart
            compact
            zones={zones}
            before={before}
            after={hasScenario ? after : undefined}
          />
          <p className="data-note">
            Odhad jednoho ročníku: děti 10–14 let / 5 · SLDB 2021. Podíly podle dětí, nikoli rozlohy.
          </p>
        </section>
      </div>
      <div className="analytics-grid compact-analytics regional-analytics">
        <DemandPanel
          data={data}
          field={field}
          onSelect={(id) => changeSelection(id, form)}
        />
        <ProgramDetail field={field} limit={threshold} onSelectSchool={(id) => openPlace("school", id)} />
      </div>
      </QueryBoundary>
    </>
  );
}

function RemovalSummary({ summary, multiple }: { summary: RemovalResponse["souhrn"]; multiple: boolean }) {
  return <div className="simulation-result">
    <p><strong>{multiple ? "Dopad celého scénáře" : "Dopad odebrání oboru"}</strong></p>
    <p>Kapacita oboru v kraji: {number(summary.kapacita_pred)} → {number(summary.kapacita_po)} míst</p>
    <p>Děti, které ztratí dostupnost oboru: <strong>{number(summary.ztracene_deti)}</strong></p>
    <p>Děti nově v dosahu: {number(summary.nove_dosazene_deti)}</p>
    <p className="data-note">Odhad jednoho ročníku z API (10–14 let / 5). Skutečná nabídka školy se nemění.</p>
  </div>;
}

function SimulationSummary({ summary }: { summary: NonNullable<SimulationResponse["souhrn"]> }) {
  const decimal = (value: number | undefined) => value == null ? "Neuvedeno" : new Intl.NumberFormat("cs-CZ", { maximumFractionDigits: 2 }).format(value);
  const verdicts = { dobre_misto: "Dobré místo", spatne_misto: "Nevhodné místo", neutralni: "Neutrální" };
  return (
    <div className="simulation-result">
      <p><strong>{summary.verdikt ? verdicts[summary.verdikt] : "Bez hodnocení"}</strong></p>
      <p>Odhad potenciálních uchazečů: <strong>{decimal(summary.potencialni_uchazeci)}</strong></p>
      <p>Odlehčení školám s nedostatkem míst: {decimal(summary.odlehceni)}</p>
      <p>Přetažení od ostatních škol: {decimal(summary.pretazeni)}</p>
      <p>Nově v dosahu: {decimal(summary.novi_v_dosahu)}</p>
      <p>Využití přidané kapacity: {summary.vyuziti == null ? "Neuvedeno" : `${decimal(summary.vyuziti * 100)} %`}</p>
      <p className="data-note">Modelový odhad uchazečů, nikoliv skutečné přihlášky nebo přijetí.</p>
    </div>
  );
}
