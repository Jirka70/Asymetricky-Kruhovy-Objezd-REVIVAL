"use client";
import { useQuery } from "@tanstack/react-query";
import { ArrowUpRightIcon } from "@phosphor-icons/react";
import { schoolQuery } from "@/lib/api";
import { QueryBoundary, QueryStatus } from "./query-state";
import AdmissionStats from "./admission-stats";

type SchoolSelection = {
  redizo: string;
  field: string;
  form: string;
};

export function SchoolAdmissions({ redizo, field, form }: SchoolSelection) {
  const query = useQuery(schoolQuery(redizo));
  const offers = query.data?.nabidky.filter((offer) => offer.kod_oboru === field && offer.forma === form) ?? [];
  return <div className="school-admissions">
    <QueryBoundary queries={[query]} compact>
      <QueryStatus queries={[query]} />
      <AdmissionStats offers={offers} />
    </QueryBoundary>
  </div>;
}

export default function SchoolInformation({ redizo, field, form, showAdmissions = true }: SchoolSelection & { showAdmissions?: boolean }) {
  const query = useQuery(schoolQuery(redizo));
  const school = query.data;
  const offers = school?.nabidky.filter((o) => o.kod_oboru === field && o.forma === form) ?? [];
  const web = school?.web;
  const href = web?.startsWith("https://") || web?.startsWith("http://")
    ? web : web ? `https://${web}` : null;
  return (
    <QueryBoundary queries={[query]} compact>
      <QueryStatus queries={[query]} />
      {school && <>
        <p>{school.nazev}</p>
        <p>{school.adresa ?? "Adresa není uvedena."}</p>
        {offers.length > 0 && <p>
          Kapacita: {offers.reduce((sum, o) => sum + o.kapacita, 0)}
        </p>}
        {showAdmissions && <AdmissionStats offers={offers} />}
        {href && <a className="text-link" href={href} target="_blank" rel="noreferrer">
          Web školy <ArrowUpRightIcon size={15} />
        </a>}
      </>}
    </QueryBoundary>
  );
}
