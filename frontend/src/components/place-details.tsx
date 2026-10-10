"use client";
import {
  ArrowUpRightIcon,
  BriefcaseIcon,
  GraduationCapIcon,
  InfoIcon,
  PersonSimpleWalkIcon,
  BusIcon,
  ArrowsLeftRightIcon,
  CaretRightIcon,
} from "@phosphor-icons/react";
import {
  exampleAdmission,
  number,
  type Employer,
  type Snapshot,
} from "@/lib/data";
import { exampleLegs, type Journey } from "@/lib/journeys";
import InlineJourney from "./inline-journey";

export function JourneySummary({
  journey,
  data,
  field,
  form,
  active,
  onSelect,
}: {
  journey: Journey;
  data: Snapshot;
  field: string;
  form: string;
  active?: boolean;
  onSelect?: () => void;
}) {
  const offer = data.offerings.find(
    (o) =>
      o.school === journey.school.id && o.field === field && o.form === form,
  );
  const legs = exampleLegs(journey);
  const { rate } = exampleAdmission(journey.school.id);
  const heading = (
    <>
      <span className="place-symbol school-symbol">
        <GraduationCapIcon size={20} weight="fill" />
      </span>
      <span className="journey-school-name">
        {journey.school.shortName}
        <small>{journey.school.city}</small>
      </span>
      {onSelect && <CaretRightIcon size={14} />}
      <span className="journey-times">
        <strong>
          {Math.ceil(journey.duration)} <small>min</small>
        </strong>
        <span>
          {journey.departure.slice(0, 5)} → {journey.arrival.slice(0, 5)}
        </span>
      </span>
    </>
  );
  return (
    <article className={`journey-result${active ? " is-selected" : ""}`}>
      {onSelect ? (
        <button
          id={`journey-choice-${journey.school.id}`}
          className="journey-select"
          onClick={onSelect}
          aria-label={`Detail školy ${journey.school.shortName}, cesta ${Math.ceil(journey.duration)} minut`}
        >
          {heading}
        </button>
      ) : (
        <div className="journey-select">{heading}</div>
      )}
      <div className="journey-segments" aria-label="Ukázkové rozdělení cesty">
        {legs.map((leg, i) => (
          <span
            key={i}
            className={`journey-segment ${leg.kind}`}
            style={{ flexGrow: leg.end - leg.start }}
            title={`${leg.label}: ${Math.round(leg.end - leg.start)} min`}
          />
        ))}
      </div>
      <div className="journey-leg-labels" aria-hidden="true">
        {legs.map((leg, i) => {
          const Icon =
            leg.kind === "walk"
              ? PersonSimpleWalkIcon
              : leg.kind === "ride"
                ? BusIcon
                : ArrowsLeftRightIcon;
          return (
            <span key={i}>
              <Icon size={15} />
              {Math.round(leg.end - leg.start)} min
            </span>
          );
        })}
      </div>
      <div className="journey-result-bottom">
        {active && (
          <span className="journey-departure">
            Odjezd <b>{journey.departure.slice(0, 5)}</b>
            <br />
            Příjezd <b>{journey.arrival.slice(0, 5)}</b>
          </span>
        )}
        <div className="journey-admission">
          <span>Přijato · ukázka</span>
          <strong>{rate} %</strong>
        </div>
        <div className="journey-capacity">
          <span>Kapacita / přihlášky</span>
          <strong>
            {offer?.capacity ?? "—"} / {offer?.applications ?? "—"}
          </strong>
        </div>
      </div>
    </article>
  );
}

export function SchoolDetail({
  data,
  schoolId,
  field,
  form,
  journey,
  boundary,
  mode,
}: {
  data: Snapshot;
  schoolId: string;
  field: string;
  form: string;
  journey?: Journey;
  boundary: string;
  mode: "arrival" | "departure";
}) {
  const school = data.schools.find((s) => s.id === schoolId);
  if (!school) return null;
  const offer = data.offerings.find(
    (o) => o.school === schoolId && o.field === field && o.form === form,
  );
  return (
    <div className="place-detail-content">
      <div className="place-title">
        <span className="place-symbol school-symbol">
          <GraduationCapIcon size={23} weight="fill" />
        </span>
        <div>
          <h2 id="detail-heading" tabIndex={-1}>
            {school.shortName}
          </h2>
          <p>{school.city}</p>
        </div>
      </div>
      <p className="detail-program">
        {data.fields.find((f) => f.id === field)?.name}
        <small>
          {field} · {form === "den" ? "Denní" : "Dálkové"} studium
        </small>
      </p>
      {journey ? (
        <>
          <JourneySummary
            journey={journey}
            data={data}
            field={field}
            form={form}
          />
          <InlineJourney journey={journey} boundary={boundary} mode={mode} />
        </>
      ) : (
        <p className="detail-callout">
          Škola nabízí vybraný obor, ale pro vaše místo a čas nemáme uloženou
          cestu. Zkuste upravit hledání.
        </p>
      )}
      <section className="detail-section">
        <h3>O škole</h3>
        <p>{school.name}</p>
        <p>{school.address}</p>
        {offer && !journey && (
          <p>
            Kapacita: {offer.capacity ?? "neuvedena"} · Přihlášky:{" "}
            {offer.applications ?? "neuvedeny"}
          </p>
        )}
        {school.web && (
          <a
            className="text-link"
            href={
              school.web.startsWith("http")
                ? school.web
                : `https://${school.web}`
            }
            target="_blank"
            rel="noreferrer"
          >
            Web školy <ArrowUpRightIcon size={15} />
          </a>
        )}
      </section>
      <p className="detail-note">
        <InfoIcon size={14} />
        Podíl přijatých je pouze modelová ukázka. Skutečné počty přijatých v
        datech chybí.
      </p>
    </div>
  );
}

const EDUCATION: Record<string, string> = {
  bezVzdel: "Bez vzdělání",
  nizsiStred: "Neúplné základní",
  zaklPraktSkol: "Základní nebo praktická škola",
  nizsiStredOdbor: "Nižší střední odborné",
  stredOdborVyuc: "Střední odborné s výučním listem",
  stredOdbor: "Střední odborné bez maturity",
  usoSMat: "Střední odborné s maturitou",
  usoSMatVyuc: "Střední odborné s maturitou a výučním listem",
  usv: "Úplné střední všeobecné",
  vyssOdbor: "Vyšší odborné",
  bakal: "Bakalářské",
  vysoka: "Vysokoškolské",
  doktorske: "Doktorské",
};
export function EmployerDetail({
  employer,
  fieldName,
}: {
  employer: Employer;
  fieldName: string;
}) {
  const imported = new Intl.DateTimeFormat("cs-CZ", {
    timeZone: "Europe/Prague",
  }).format(new Date(employer.importedat));
  return (
    <div className="place-detail-content employer-detail">
      <div className="place-title">
        <span className="place-symbol employer-symbol">
          <BriefcaseIcon size={21} />
        </span>
        <div>
          <h2 id="detail-heading" tabIndex={-1}>
            {employer.name}
          </h2>
          <p>Zaměstnavatel · {employer.city}</p>
        </div>
      </div>
      <p className="detail-program">
        Obor: <strong>{fieldName}</strong>
      </p>
      <h3>{number(employer.jobs)} míst v příbuzných profesích</h3>
      <p className="detail-note">Vhodnost oboru se liší podle profese.</p>
      <ul className="profession-list" aria-label="Profese a vhodnost oboru">
        {employer.professions.map((p, i) => (
          <li key={`${p.code}-${p.education}-${i}`}>
            <div>
              <strong>{p.name}</strong>
              <small>
                CZ-ISCO {p.code} · {EDUCATION[p.education] ?? p.education}
              </small>
            </div>
            <span
              className={`suitability ${p.suitability === 1 ? "best" : ""}`}
            >
              {p.suitability === 1 ? "Nejvhodnější" : "Vhodný"}
            </span>
            <b className="profession-count">
              {p.jobs}
              <small>
                {p.jobs === 1 ? "místo" : p.jobs < 5 ? "místa" : "míst"}
              </small>
            </b>
          </li>
        ))}
      </ul>
      <p className="detail-note">
        <InfoIcon size={14} />
        Vztah oboru a profese podle NSP. Nejde o hodnocení firmy.
      </p>
      <dl className="employer-facts">
        <div>
          <dt>IČO</dt>
          <dd>{employer.ico}</dd>
        </div>
        <div>
          <dt>Pracoviště</dt>
          <dd>{employer.city}</dd>
        </div>
        <div>
          <dt>Adresní místo</dt>
          <dd>{employer.addresspoint ?? "Neuvedeno"}</dd>
        </div>
        <div>
          <dt>Import dat</dt>
          <dd>{imported}</dd>
        </div>
      </dl>
      <details className="technical-details">
        <summary>Údaje o poloze</summary>
        <p>Kód obce: {employer.municipality}</p>
        <p>
          Souřadnice: {employer.lat.toFixed(5)}, {employer.lon.toFixed(5)}
        </p>
      </details>
    </div>
  );
}
