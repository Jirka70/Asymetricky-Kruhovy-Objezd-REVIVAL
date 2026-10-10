"use client";
import { useEffect, useState, type ReactNode } from "react";
import type { Snapshot } from "@/lib/data";
let cached: Promise<Snapshot> | undefined;
function load() {
  cached ??= fetch("/data/snapshot.json")
    .then((r) => {
      if (!r.ok) throw new Error("Data nejsou dostupná");
      return r.json() as Promise<Snapshot>;
    })
    .catch((e) => {
      cached = undefined;
      throw e;
    });
  return cached;
}
export function DataProvider({
  children,
}: {
  children: (data: Snapshot) => ReactNode;
}) {
  const [data, setData] = useState<Snapshot | null>(null);
  const [error, setError] = useState(false);
  const [retry, setRetry] = useState(0);
  useEffect(() => {
    let active = true;
    load()
      .then((d) => {
        if (active) {
          setData(d);
          setError(false);
        }
      })
      .catch(() => {
        if (active) setError(true);
      });
    return () => {
      active = false;
    };
  }, [retry]);
  if (error)
    return (
      <div className="load-state" role="alert">
        <h2>Datový snímek se nepodařilo načíst.</h2>
        <button onClick={() => setRetry(retry + 1)}>Zkusit znovu</button>
      </div>
    );
  if (!data)
    return (
      <div className="load-state" role="status">
        Načítání dat Karlovarského kraje…
      </div>
    );
  return children(data);
}
