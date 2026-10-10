"use client";
import { useQuery } from "@tanstack/react-query";
import { ArrowUpRightIcon } from "@phosphor-icons/react";
import { schoolQuery } from "@/lib/api";
import { QueryBoundary, QueryStatus } from "./query-state";

export default function SchoolInformation({ redizo, field, form }: {
  redizo: string;
  field: string;
  form: string;
}) {
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
          Kapacita: {offers.reduce((sum, o) => sum + o.kapacita, 0)} · Přihlášky: {offers.reduce((sum, o) => sum + o.prihlasky, 0)}
        </p>}
        {href && <a className="text-link" href={href} target="_blank" rel="noreferrer">
          Web školy <ArrowUpRightIcon size={15} />
        </a>}
      </>}
    </QueryBoundary>
  );
}
