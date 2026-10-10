import type { ConnectionSummary as Summary } from "@/lib/api";
import { number } from "@/lib/data";

export function transfers(count: number) {
  return count === 0 ? "Bez přestupu" : `${count} ${count === 1 ? "přestup" : count < 5 ? "přestupy" : "přestupů"}`;
}

export default function ConnectionSummary({ connection }: { connection: Summary }) {
  if (connection.stav !== "ok") return null;
  const parts = [
    connection.prestupy == null ? null : transfers(connection.prestupy),
    connection.chuze_m == null ? null : `Chůze ${number(connection.chuze_m)} m`,
    connection.vzdalenost_m == null ? null : `Celkem ${number(connection.vzdalenost_m)} m`,
  ].filter(Boolean);
  return <div className="connection-summary">
    {(connection.odjezd || connection.prijezd) && <p>
      {connection.odjezd && <>Odjezd <strong>{connection.odjezd.slice(0, 5)}</strong></>}
      {connection.odjezd && connection.prijezd && " · "}
      {connection.prijezd && <>Příjezd <strong>{connection.prijezd.slice(0, 5)}</strong></>}
    </p>}
    {parts.length > 0 && <p>{parts.join(" · ")}</p>}
    {Boolean(connection.linky?.length) && <p>Linky: {connection.linky!.join(" → ")}</p>}
  </div>;
}
