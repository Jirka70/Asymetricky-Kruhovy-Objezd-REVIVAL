"use client";
import { useQuery } from "@tanstack/react-query";
import { programQuery } from "@/lib/api";
import { QueryBoundary, QueryStatus } from "./query-state";

export default function ProgramDetail({ field, limit, onSelectSchool }: {
  field: string; limit: number; onSelectSchool?: (id: string) => void;
}) {
  const query = useQuery(programQuery(field, limit));
  const result = query.data;
  const value = (number: number | null | undefined) => number == null ? "Neuvedeno" : new Intl.NumberFormat("cs-CZ", {maximumFractionDigits: 2}).format(number);
  return (
    <section className="section program-detail">
      <h2>Detail oboru</h2>
      <p className="data-note">Současná nabídka · všechny formy studia · ranní dojezd {limit === 0 ? "bez časového limitu" : `do ${limit} minut`}.</p>
      {result && <QueryStatus queries={[query]} />}
      <QueryBoundary queries={[query]} compact>
        {result && <>
          <h3>{result.obor.nazev}</h3>
          <p>{value(result.obor.pocet_skol)} škol · Kapacita: {value(result.obor.kapacita)} · Přihlášky: {value(result.obor.prihlasky)}</p>
          <p>Přihlášek na místo: {value(result.obor.prihlasky_na_misto)}</p>
          <p>Odhad dětí v dosahu: {value(result.obor.deti_v_dosahu)} · mimo dosah: {value(result.obor.deti_bez_oboru)}</p>
          <p className="data-note">Odhad jednoho ročníku ze SLDB 2021 (10–14 let / 5). Přihlášky na místo nejsou míra přijetí.</p>
          <p>{result.trh_prace ? `Volná pracovní místa: ${value(result.trh_prace.volna_mista)} · Zaměstnavatelé: ${value(result.trh_prace.zamestnavatelu)}` : "Pro obor není dostupné mapování trhu práce."}</p>
          <details>
            <summary>Nabídky škol ({result.nabidky.length})</summary>
            <ul>{result.nabidky.map((offer, index) => <li key={`${offer.redizo}-${offer.forma}-${index}`}>
              {offer.nazev_skoly ?? offer.redizo ?? "Škola neuvedena"} · {offer.forma === "den" ? "denní" : "dálkové"} · {offer.kapacita} míst / {offer.prihlasky} přihlášek
            </li>)}</ul>
          </details>
          <h3>Kde by nový obor rozšířil dosah?</h3>
          {result.kandidati.length ? <ul>{result.kandidati.map((school) => <li key={school.redizo}>
            {onSelectSchool ? <button className="button-link" onClick={() => onSelectSchool(school.redizo)}>{school.nazev}</button> : school.nazev}
            {" · +"}{value(school.nove_dosazene_deti)} dětí{school.ma_pribuzny_obor ? " · má příbuzný obor" : ""}
          </li>)}</ul> : <p>API nevrátilo žádné kandidátní školy.</p>}
        </>}
      </QueryBoundary>
    </section>
  );
}
