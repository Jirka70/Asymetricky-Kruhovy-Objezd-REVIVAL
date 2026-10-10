"use client";
import { useMemo, useRef, useState } from "react";
import dynamic from "next/dynamic";
import {
  ArrowLeftIcon,
  MagnifyingGlassIcon,
  MapPinIcon,
  GraduationCapIcon,
  ClockIcon,
  InfoIcon,
  CaretDownIcon,
  ChartBarIcon,
} from "@phosphor-icons/react";
import SearchSelect from "./search-select";
import { JourneySummary, SchoolDetail, EmployerDetail } from "./place-details";
import { focusSection } from "@/lib/focus";
import { DataProvider } from "./data-provider";
import { QueryBoundary, QueryStatus, DataSkeleton } from "./query-state";
import { useSelectionData } from "@/lib/use-selection-data";
import {
  DEFAULT_FIELD,
  schoolIds,
  travelTimes,
  type Snapshot,
} from "@/lib/data";
import { findJourneys } from "@/lib/journeys";
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
const JourneyPanel = dynamic(() => import("./journey-panel"));
type Selection = { kind: "school" | "employer"; id: string } | null;
export default function FamilyDashboard() {
  return <DataProvider>{(data) => <Dashboard data={data} />}</DataProvider>;
}
function Dashboard({ data: baseData }: { data: Snapshot }) {
  const [field, setField] = useState(DEFAULT_FIELD),
    [origin, setOrigin] = useState("063550"),
    [form, setForm] = useState("den");
  const [requestedTime, setRequestedTime] = useState("07:50"),
    [mode, setMode] = useState<"arrival" | "departure">("arrival");
  const [search, setSearch] = useState({
    field: DEFAULT_FIELD,
    origin: "063550",
    form: "den",
    time: "07:50",
    mode: "arrival" as "arrival" | "departure",
  });
  const [selected, setSelected] = useState("600170527");
  const selection = useSelectionData(baseData, search.field, search.form);
  const { data } = selection;
  const [panel, setPanel] = useState<Selection>(null);
  const [notice, setNotice] = useState("");
  const [discoveryOpen, setDiscoveryOpen] = useState(false),
    [timelineOpen, setTimelineOpen] = useState(false);
  const returnFocus = useRef<HTMLElement | null>(null);
  const ids = useMemo(
    () => schoolIds(data, search.field, search.form),
    [data, search.field, search.form],
  );
  const times = useMemo(() => travelTimes(data, ids), [data, ids]);
  const journeys = useMemo(
    () => findJourneys(data, search.origin, ids, search.mode, search.time),
    [data, search, ids],
  );
  const selectedId = journeys.some((j) => j.school.id === selected)
    ? selected
    : (journeys[0]?.school.id ?? "");
  const zone = data.zsj.find((z) => z.id === search.origin)!;
  const location = data.municipalities.find((m) => m.id === zone.municipality)!;
  const selectedField =
    data.fields.find((f) => f.id === search.field)?.name ?? search.field;
  const dirty =
    search.field !== field ||
    search.origin !== origin ||
    search.form !== form ||
    search.time !== requestedTime ||
    search.mode !== mode;
  const zoneOptions = useMemo(() => {
    const names = new Map(data.municipalities.map((m) => [m.id, m.name]));
    return data.zsj
      .map((z) => ({
        value: z.id,
        label: `${names.get(z.municipality)} · ${z.name}`,
      }))
      .sort((a, b) => a.label.localeCompare(b.label, "cs"));
  }, [data]);
  const fieldOptions = data.fields.map((f) => ({
    value: f.id,
    label: `${f.name} · ${f.id}`,
  }));
  const otherForm = search.form === "den" ? "dal" : "den";
  const hasAlternateForm = data.fields.find((f) => f.id === search.field)?.forms?.includes(otherForm);
  const fullWindow = findJourneys(data, search.origin, ids, "arrival", "08:00");
  function recommend(id: string) {
    setField(id);
    setNotice(
      `Vybrán obor ${data.fields.find((f) => f.id === id)?.name}. Potvrďte hledání.`,
    );
    setDiscoveryOpen(false);
    focusSection("search-heading");
  }
  function applySearch(next: typeof search) {
    setSearch(next);
    setPanel(null);
    setTimelineOpen(false);
    setNotice("Zadání hledání bylo aktualizováno.");
    focusSection("results-heading");
  }
  function submit(e: React.FormEvent) {
    e.preventDefault();
    applySearch({ field, origin, form, time: requestedTime, mode });
  }
  function recover(kind: "time" | "form") {
    const next =
      kind === "time"
        ? { ...search, mode: "arrival" as const, time: "08:00" }
        : { ...search, form: otherForm };
    setField(next.field);
    setOrigin(next.origin);
    setForm(next.form);
    setMode(next.mode);
    setRequestedTime(next.time);
    applySearch(next);
  }
  function openPlace(kind: "school" | "employer", id: string) {
    returnFocus.current =
      document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;
    setPanel({ kind, id });
    if (kind === "school") setSelected(id);
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
      else focusSection("results-heading");
    });
  }
  const employer =
    panel?.kind === "employer"
      ? data.employers.find(
          (e) => e.id === panel.id && e.field === search.field,
        )
      : undefined;
  return (
    <>
      <form
        id="school-search"
        className="compact-search"
        onSubmit={submit}
        aria-labelledby="search-heading"
      >
        <h2 id="search-heading" tabIndex={-1} className="sr-only">
          Hledání škol a dojezdů
        </h2>
        <div className="compact-search-fields">
          <SearchSelect
            label="Odkud"
            icon={<MapPinIcon size={15} />}
            value={origin}
            onChange={setOrigin}
            options={zoneOptions}
            placeholder="Např. Toužim nebo Rybáře"
          />
          <SearchSelect
            label="Obor"
            icon={<GraduationCapIcon size={16} />}
            value={field}
            onChange={setField}
            options={fieldOptions}
            placeholder="Název nebo kód oboru"
          />
          <div className="field compact-time">
            <label htmlFor="journey-time">
              <ClockIcon size={15} />{" "}
              {mode === "arrival" ? "Příchod do školy" : "Odchod z místa"}
            </label>
            <div className="time-row">
              <select
                aria-label="Časová podmínka"
                value={mode}
                onChange={(e) =>
                  setMode(e.target.value as "arrival" | "departure")
                }
              >
                <option value="arrival">Příchod do</option>
                <option value="departure">Odchod od</option>
              </select>
              <input
                id="journey-time"
                type="time"
                required
                value={requestedTime}
                onChange={(e) => setRequestedTime(e.target.value)}
              />
            </div>
          </div>
          <label className="field compact-form">
            <span>Forma studia</span>
            <select value={form} onChange={(e) => setForm(e.target.value)}>
              <option value="den">Denní</option>
              <option value="dal">Dálková</option>
            </select>
          </label>
          <button className="primary compact-submit" type="submit">
            <MagnifyingGlassIcon size={18} />
            Najít školy
          </button>
        </div>
      </form>
      <div className="workspace-caption">
        <span>Uložené ranní cesty · Po 12. 10. 2026 · příjezdy 7:00–8:00</span>
        <button
          className="button-link"
          aria-expanded={discoveryOpen}
          aria-controls="field-discovery"
          onClick={() => setDiscoveryOpen(!discoveryOpen)}
        >
          <ChartBarIcon size={16} />
          {discoveryOpen ? "Skrýt porovnání oborů" : "Pomoc s výběrem oboru"}
          <CaretDownIcon size={13} />
        </button>
      </div>
      <p className="search-status" role="status" aria-atomic="true">
        {notice}
      </p>
      {dirty && (
        <div className="pending-search" role="status">
          <span>
            Zadání je změněné. Výsledky zatím odpovídají předchozímu hledání.
          </span>
          <button type="submit" form="school-search">
            Aktualizovat výsledky
          </button>
        </div>
      )}
      <div id="field-discovery" hidden={!discoveryOpen}>
        <h2 id="field-discovery-heading" className="sr-only" tabIndex={-1}>
          Poptávka po oborech a ukázka přijímání
        </h2>
        {discoveryOpen && (
          <div className="discovery-grid compact-analytics">
            <DemandPanel data={data} field={field} onSelect={recommend} />
            <AdmissionPanel data={data} onSelect={recommend} />
          </div>
        )}
      </div>
      <QueryStatus queries={selection.schools.data ? [selection.schools, selection.employers] : [selection.employers]} />
      <QueryBoundary queries={[selection.schools]}>
      <div className="map-workspace family-workspace">
        <section
          className="workspace-map"
          aria-label="Dostupnost oboru na mapě"
        >
          <RegionMap
            data={data}
            times={times}
            schoolIds={ids}
            field={search.field}
            employersNotice={selection.employersNotice}
            onSelectSchool={(id) => openPlace("school", id)}
            onSelectEmployer={(id) => openPlace("employer", id)}
            selectedEmployerId={
              panel?.kind === "employer" ? panel.id : undefined
            }
            selectedSchoolId={panel?.kind === "school" ? panel.id : selectedId}
            family
          />
        </section>
        <aside
          className="workspace-detail family-detail"
          aria-label="Spojení a podrobnosti"
          onKeyDown={(e) => {
            if (e.key === "Escape" && panel) closeDetail();
          }}
        >
          <div className="detail-navigation">
            <button
              className="button-link"
              onClick={
                panel ? closeDetail : () => focusSection("search-heading")
              }
            >
              <ArrowLeftIcon size={18} />
              {panel ? "Zpět na spojení" : "Zpět k hledání"}
            </button>
          </div>
          {panel?.kind === "school" ? (
            <SchoolDetail
              data={data}
              schoolId={panel.id}
              field={search.field}
              form={search.form}
              journey={journeys.find((j) => j.school.id === panel.id)}
              boundary={search.time}
              mode={search.mode}
            />
          ) : employer ? (
            <EmployerDetail employer={employer} fieldName={selectedField} />
          ) : (
            <>
              <div className="journey-list-heading">
                <h2 id="results-heading" tabIndex={-1}>
                  {journeys.length}{" "}
                  {journeys.length === 1
                    ? "škola"
                    : journeys.length > 1 && journeys.length < 5
                      ? "školy"
                      : "škol"}
                  {journeys.length > 0 &&
                    ` · od ${Math.ceil(journeys[0].duration)} min`}
                </h2>
                <p>
                  {search.mode === "arrival" ? "Příchod do" : "Odchod od"}{" "}
                  {search.time}
                  <span className="result-origin">
                    {" "}
                    · {location.name} · {zone.name}
                  </span>
                </p>
              </div>
              {journeys.length ? (
                <>
                  <div className="journey-results">
                    {journeys.map((j) => (
                      <JourneySummary
                        key={j.school.id}
                        journey={j}
                        data={data}
                        field={search.field}
                        form={search.form}
                        active={j.school.id === selectedId}
                        onSelect={() => openPlace("school", j.school.id)}
                      />
                    ))}
                  </div>
                  <details
                    className="timeline-disclosure"
                    open={timelineOpen}
                    onToggle={(e) => setTimelineOpen(e.currentTarget.open)}
                  >
                    <summary>Porovnat cesty na časové ose</summary>
                    {timelineOpen && (
                      <JourneyPanel
                        comparisonOnly
                        journeys={journeys}
                        selected={selectedId}
                        onSelect={(id) => openPlace("school", id)}
                        boundary={search.time}
                        mode={search.mode}
                      />
                    )}
                  </details>
                  <p className="detail-note results-caveat">
                    <InfoIcon size={14} />
                    Úseky cest a podíly přijatých jsou ukázkové. Odjezdy a
                    příjezdy jsou z uloženého výpočtu; kapacita a přihlášky z
                    dat škol.
                  </p>
                </>
              ) : (
                <div className="empty-state">
                  <strong>
                    {ids.size === 0
                      ? "Tento obor v této formě nemá v nabídce žádná škola."
                      : "Pro toto místo a čas nemáme uloženou cestu."}
                  </strong>
                  <p>
                    {ids.size === 0
                      ? "Zkuste jiný obor nebo formu studia."
                      : "Zkuste jiný čas nebo místo. Skutečné spojení může existovat i mimo náš snímek."}
                  </p>
                  <div className="empty-actions">
                    {ids.size === 0 && hasAlternateForm && (
                      <button onClick={() => recover("form")}>
                        Zkusit {otherForm === "den" ? "denní" : "dálkové"}{" "}
                        studium
                      </button>
                    )}
                    {ids.size > 0 && fullWindow.length > 0 && (
                      <button onClick={() => recover("time")}>
                        Zobrazit příjezdy do 08:00 ({fullWindow.length})
                      </button>
                    )}
                    <button
                      className="button-link"
                      onClick={() => focusSection("search-heading")}
                    >
                      Upravit hledání
                    </button>
                  </div>
                </div>
              )}
              {ids.size > journeys.length && (
                <p className="detail-note results-caveat">
                  Další školy bez uložené cesty pro toto zadání:{" "}
                  {ids.size - journeys.length}. Jejich detail otevřete v mapě.
                </p>
              )}
              <p className="detail-footnote">
                Klikněte na školu nebo zaměstnavatele v mapě pro detail.
              </p>
            </>
          )}
        </aside>
      </div>
      </QueryBoundary>
    </>
  );
}
