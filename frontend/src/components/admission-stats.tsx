import type { SchoolResponse } from "@/lib/api";
import { admissionStats } from "@/lib/admissions";

const number = new Intl.NumberFormat("cs-CZ");
const percent = new Intl.NumberFormat("cs-CZ", { style: "percent", maximumFractionDigits: 1 });

export default function AdmissionStats({ offers }: { offers: SchoolResponse["nabidky"] }) {
  const stats = admissionStats(offers);
  return <section className="admission-stats" aria-label="Statistiky přijetí">
    <p className="detail-note">Přijímací řízení 2026 · 1. kolo</p>
    {stats ? <>
      <dl className="admission-numbers">
        <div><dt>Přihlášky</dt><dd>{number.format(stats.applications)}</dd></div>
        <div><dt>Přijatí</dt><dd>{stats.accepted === null ? "Neuvedeno" : number.format(stats.accepted)}</dd></div>
        <div><dt>Podíl přijatých</dt><dd>{stats.rate === null ? "Nelze určit" : percent.format(stats.rate)}</dd></div>
      </dl>
      <p className="detail-note">Za vybraný obor a formu studia. Podíl přijatých z přihlášek.</p>
    </> : <p className="detail-note">Pro tento obor a formu nemáme údaje o přijímání.</p>}
  </section>;
}
