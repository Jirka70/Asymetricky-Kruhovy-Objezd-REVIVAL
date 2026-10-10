"use client";
import type { ReactNode } from "react";

export type QueryState = {
  data: unknown;
  error: Error | null;
  isFetching: boolean;
  refetch: () => Promise<unknown>;
};

export function DataSkeleton({ label = "Načítání dat…", compact = false }) {
  return (
    <div className={`data-skeleton${compact ? " is-compact" : ""}`} role="status" aria-busy="true">
      <span className="sr-only">{label}</span>
      <div className="skeleton-line" />
      <div className="skeleton-line short" />
      <div className="skeleton-block" />
    </div>
  );
}

export function QueryStatus({ queries }: { queries: QueryState[] }) {
  const failed = queries.filter((q) => q.error);
  if (failed.length) {
    return (
      <div className="query-notice" role="alert">
        <span>{failed.every((q) => q.data !== undefined)
          ? "Obnova dat selhala. Zobrazené údaje jsou z posledního úspěšného načtení."
          : "Data se nepodařilo načíst. Zkuste to znovu."}</span>
        <button disabled={failed.some((q) => q.isFetching)} onClick={() => {
          for (const query of failed) void query.refetch();
        }}>Zkusit znovu</button>
      </div>
    );
  }
  if (queries.some((q) => q.isFetching && q.data !== undefined)) {
    return <p className="query-updating" role="status">Aktualizuji data…</p>;
  }
  return null;
}

export function QueryBoundary({ queries, children, compact = false }: {
  queries: QueryState[];
  children: ReactNode;
  compact?: boolean;
}) {
  if (queries.some((q) => q.data === undefined)) {
    return queries.some((q) => q.error)
      ? <QueryStatus queries={queries} />
      : <DataSkeleton compact={compact} />;
  }
  return children;
}
