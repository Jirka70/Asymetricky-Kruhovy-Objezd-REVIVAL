"use client";
import { GraduationCapIcon, CaretRightIcon } from "@phosphor-icons/react";
import type { StudentSchool } from "@/lib/api";
import type { School } from "@/lib/data";
import AdmissionStats from "./admission-stats";

export default function StudentSchoolCard({ result, school, active, onSelect, showAdmissions = true }: {
  result: StudentSchool; school?: School; active?: boolean; onSelect?: () => void;
  showAdmissions?: boolean;
}) {
  const duration = result.spoj.cas_min;
  const capacity = result.nabidky.reduce((sum, offer) => sum + offer.kapacita, 0);
  const title = <>
    <span className="place-symbol school-symbol"><GraduationCapIcon size={20} weight="fill" /></span>
    <span className="journey-school-name">{school?.shortName ?? result.nazev}<small>{school?.city}</small></span>
    <span className="journey-times"><strong>{duration == null ? "Dojezd neznámý" : `${duration} min`}</strong></span>
    {onSelect && <CaretRightIcon size={14} />}
  </>;
  return <article className={`journey-result${active ? " is-selected" : ""}`}>
    {onSelect ? <button className="journey-select" onClick={onSelect} aria-label={`Detail školy ${result.nazev}`}>{title}</button> : <div className="journey-select">{title}</div>}
    <p className="detail-note">{duration == null ? "Doba dojezdu není v datech dostupná; spojení může existovat." : result.v_dosahu ? "V zadaném limitu dojezdu" : "Mimo zadaný limit dojezdu"}</p>
    {showAdmissions && <>
      <p className="detail-note">Kapacita: {capacity}</p>
      <AdmissionStats offers={result.nabidky} />
    </>}
  </article>;
}
