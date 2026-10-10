"use client";
import { useMemo, type ReactNode } from "react";
import { useQuery } from "@tanstack/react-query";
import { snapshotQuery, schoolsQuery, programsQuery, zsjListQuery } from "@/lib/api";
import { catalogData } from "@/lib/api-data";
import type { Snapshot } from "@/lib/data";
import { QueryBoundary, QueryStatus } from "./query-state";
export function DataProvider({
  children,
}: {
  children: (data: Snapshot) => ReactNode;
}) {
  // Independent reads start together. Both forms preserve distance-only programs.
  const snapshot = useQuery(snapshotQuery);
  const schools = useQuery(schoolsQuery());
  const daily = useQuery(programsQuery("den"));
  const distance = useQuery(programsQuery("dal"));
  const zones = useQuery(zsjListQuery);
  const data = useMemo(() =>
    snapshot.data && schools.data && daily.data && distance.data && zones.data
      ? catalogData(snapshot.data, schools.data, daily.data, distance.data, zones.data)
      : undefined,
  [snapshot.data, schools.data, daily.data, distance.data, zones.data]);
  const queries = [snapshot, schools, daily, distance, zones];
  return (
    <QueryBoundary queries={queries}>
      <QueryStatus queries={queries} />
      {data && (data.zsj.length ? children(data) : <p className="empty-state" role="status">API nevrátilo žádné základní sídelní jednotky.</p>)}
    </QueryBoundary>
  );
}
