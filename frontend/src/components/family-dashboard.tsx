"use client";
import { useMemo, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import dynamic from "next/dynamic";
import { ArrowLeftIcon, MagnifyingGlassIcon, MapPinIcon, GraduationCapIcon, CaretDownIcon, ChartBarIcon } from "@phosphor-icons/react";
import SearchSelect from "./search-select";
import { EmployerDetail } from "./place-details";
import SchoolInformation, { SchoolAdmissions } from "./school-information";
import StudentSchoolCard from "./student-school";
import StudentRoute from "./student-route";
import ProgramDetail from "./program-detail";
import { focusSection } from "@/lib/focus";
import { DataProvider } from "./data-provider";
import { QueryBoundary, QueryStatus, DataSkeleton } from "./query-state";
import { useSelectionData } from "@/lib/use-selection-data";
import { accessibilityQuery, studentSchoolsQuery } from "@/lib/api";
import { accessibilityData } from "@/lib/accessibility-data";
import { DEFAULT_FIELD, type Snapshot } from "@/lib/data";
const RegionMap = dynamic(() => import("./region-map"), { ssr: false, loading: () => <DataSkeleton label="Načítání mapy…" /> });
const DemandPanel = dynamic(() => import("./analytics").then((m) => m.DemandPanel));
type Selection = { kind: "school" | "employer"; id: string } | null;
export default function FamilyDashboard() {
  return <DataProvider>{(data) => <Dashboard data={data} />}</DataProvider>;
}
function Dashboard({ data: baseData }: { data: Snapshot }) {
  const [field, setField] = useState(DEFAULT_FIELD), [origin, setOrigin] = useState("063550"), [form, setForm] = useState("den"), [limit, setLimit] = useState(120);
  const [search, setSearch] = useState({ field: DEFAULT_FIELD, origin: "063550", form: "den", limit: 120 });
  const selection = useSelectionData(baseData, search.field, search.form);
  const { data } = selection;
  const zone = data.zsj.find((z) => z.id === search.origin);
  const location = data.municipalities.find((m) => m.id === zone?.municipality);
  const students = useQuery(studentSchoolsQuery({ lat: zone?.lat ?? NaN, lon: zone?.lon ?? NaN, obor: search.field, forma: search.form, max_min: search.limit }));
  const accessibility = useQuery(accessibilityQuery(search.field, search.form, search.limit));
  const map = useMemo(() => accessibility.data ? accessibilityData(accessibility.data) : undefined, [accessibility.data]);
  const ids = useMemo(() => new Set(students.data?.data.map((school) => school.redizo) ?? []), [students.data]);
  const [panel, setPanel] = useState<Selection>(null);
  const [notice, setNotice] = useState("");
  const [discoveryOpen, setDiscoveryOpen] = useState(false);
  const returnFocus = useRef<HTMLElement | null>(null);
  const selectedField = data.fields.find((f) => f.id === search.field)?.name ?? search.field;
  const dirty = search.field !== field || search.origin !== origin || search.form !== form || search.limit !== limit;
  const zoneOptions = useMemo(() => {
    const names = new Map(data.municipalities.map((m) => [m.id, m.name]));
    return data.zsj.map((z) => ({value: z.id, label: `${names.get(z.municipality) ?? (z.municipality || "Obec neuvedena")} · ${z.name}`})).sort((a, b) => a.label.localeCompare(b.label, "cs"));
  }, [data]);
  const otherForm = search.form === "den" ? "dal" : "den";
  const hasAlternateForm = data.fields.find((f) => f.id === search.field)?.forms?.includes(otherForm);
  function recommend(id: string) {
    setField(id); setDiscoveryOpen(false); setNotice("Obor je vybraný. Potvrďte hledání."); focusSection("search-heading");
  }
  function applySearch(next: typeof search) {
    setSearch(next); setPanel(null); setNotice("Zadání hledání bylo aktualizováno."); focusSection("results-heading");
  }
  function openPlace(kind: "school" | "employer", id: string) {
    returnFocus.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    setPanel({kind, id}); focusSection("detail-heading");
  }
  function closeDetail() {
    setPanel(null);
    requestAnimationFrame(() => {
      if (returnFocus.current?.isConnected && returnFocus.current.checkVisibility()) returnFocus.current.focus({preventScroll: true});
      else focusSection("results-heading");
    });
  }
  const employer = panel?.kind === "employer" ? data.employers.find((e) => e.id === panel.id && e.field === search.field) : undefined;
  const school = panel?.kind === "school" ? data.schools.find((s) => s.id === panel.id) : undefined;
  const student = panel?.kind === "school" ? students.data?.data.find((s) => s.redizo === panel.id) : undefined;
  return <>
    <form id="school-search" className="compact-search" onSubmit={(e) => {e.preventDefault(); applySearch({field, origin, form, limit});}} aria-labelledby="search-heading">
      <h2 id="search-heading" tabIndex={-1} className="sr-only">Hledání škol a dojezdů</h2>
      <div className="compact-search-fields">
        <SearchSelect label="Odkud" icon={<MapPinIcon size={15} />} value={origin} onChange={setOrigin} options={zoneOptions} placeholder="Např. Toužim nebo Rybáře" />
        <SearchSelect label="Obor" icon={<GraduationCapIcon size={16} />} value={field} onChange={setField} options={data.fields.map((f) => ({value: f.id, label: `${f.name} · ${f.id}`}))} />
        <label className="field"><span>Maximální dojezd</span><select value={limit} onChange={(e) => setLimit(Number(e.target.value))}>{[30,45,60,90,120].map((value) => <option key={value} value={value}>Do {value} min</option>)}</select></label>
        <label className="field compact-form"><span>Forma studia</span><select value={form} onChange={(e) => setForm(e.target.value)}><option value="den">Denní</option><option value="dal">Dálková</option></select></label>
        <button className="primary compact-submit" type="submit"><MagnifyingGlassIcon size={18} />Najít školy</button>
      </div>
    </form>
    <div className="workspace-caption">
      <span>Odhad ranního dojezdu ze ZSJ · příjezdy 7:00–8:00</span>
      <button className="button-link" aria-expanded={discoveryOpen} aria-controls="field-discovery" onClick={() => setDiscoveryOpen(!discoveryOpen)}><ChartBarIcon size={16} />{discoveryOpen ? "Skrýt porovnání oborů" : "Pomoc s výběrem oboru"}<CaretDownIcon size={13} /></button>
    </div>
    <p className="search-status" role="status" aria-atomic="true">{notice}</p>
    {dirty && <div className="pending-search" role="status"><span>Zadání je změněné. Výsledky zatím odpovídají předchozímu hledání.</span><button type="submit" form="school-search">Aktualizovat výsledky</button></div>}
    <div id="field-discovery" hidden={!discoveryOpen}>{discoveryOpen && <div className="discovery-grid compact-analytics"><DemandPanel data={data} field={field} onSelect={recommend} /><ProgramDetail field={field} limit={limit} /></div>}</div>
    <QueryStatus queries={[selection.employers]} />
    <QueryStatus queries={[students, accessibility].filter((query) => query.data !== undefined)} />
    <QueryBoundary queries={[students, accessibility]}>
      <div className="map-workspace family-workspace">
        <section className="workspace-map" aria-label="Dostupnost oboru na mapě">
          <RegionMap data={data} times={map?.times ?? {}} nearestSchools={map?.nearestSchools} schoolIds={ids} field={search.field} employersNotice={selection.employersNotice} onSelectSchool={(id) => openPlace("school", id)} onSelectEmployer={(id) => openPlace("employer", id)} selectedEmployerId={panel?.kind === "employer" ? panel.id : undefined} selectedSchoolId={panel?.kind === "school" ? panel.id : undefined} family />
        </section>
        <aside className="workspace-detail family-detail" aria-label="Spojení a podrobnosti" onKeyDown={(e) => {if (e.key === "Escape" && panel) closeDetail();}}>
          <div className="detail-navigation"><button className="button-link" onClick={panel ? closeDetail : () => focusSection("search-heading")}><ArrowLeftIcon size={18} />{panel ? "Zpět na školy" : "Zpět k hledání"}</button></div>
          {panel?.kind === "school" ? <div className="place-detail-content">
            <h2 id="detail-heading" tabIndex={-1}>{school?.shortName ?? student?.nazev ?? panel.id}</h2>
            <p>{selectedField} · {search.form === "den" ? "Denní" : "Dálkové"} studium</p>
            <SchoolAdmissions redizo={panel.id} field={search.field} form={search.form} />
            {student ? <StudentSchoolCard result={student} school={school} showAdmissions={false} /> : <p>Pro tuto školu nejsou výsledky tohoto hledání dostupné.</p>}
            {zone && <StudentRoute key={`${zone.id}:${panel.id}`} lat={zone.lat} lon={zone.lon} redizo={panel.id} />}
            <section className="detail-section"><h3>O škole</h3><SchoolInformation redizo={panel.id} field={search.field} form={search.form} showAdmissions={false} /></section>
          </div> : employer ? <EmployerDetail employer={employer} fieldName={selectedField} /> : <>
            <div className="journey-list-heading">
              <h2 id="results-heading" tabIndex={-1}>Nalezené školy ({students.data?.data.length ?? 0})</h2>
              <p>Do {search.limit} minut · {location?.name} · {zone?.name}</p>
              {students.data?.meta.den && <p className="data-note">Den spojení: {students.data.meta.den.split("-").reverse().join(". ")}</p>}
            </div>
            {students.data?.data.length ? <div className="journey-results">{students.data.data.map((result) => <StudentSchoolCard key={result.redizo} result={result} school={data.schools.find((s) => s.id === result.redizo)} onSelect={() => openPlace("school", result.redizo)} />)}</div> : <div className="empty-state">
              <strong>Tento obor v této formě nemá v nabídce žádná škola.</strong>
              <p>Zkuste jiný obor nebo formu studia.</p>
              {hasAlternateForm && <button onClick={() => {setForm(otherForm); applySearch({...search, form: otherForm});}}>Zkusit {otherForm === "den" ? "denní" : "dálkové"} studium</button>}
            </div>}
          </>}
          <p className="detail-note results-caveat">Spojení vychází z výchozího bodu ZSJ, nikoli z konkrétní adresy. Ranní příjezdy 7:00–8:00 · časy v Europe/Prague. Výsledky se uchovávají až 24 hodin.</p>
        </aside>
      </div>
    </QueryBoundary>
  </>;
}
