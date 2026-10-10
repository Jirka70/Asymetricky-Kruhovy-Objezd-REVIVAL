"use client";
import { useMemo, type ReactNode } from "react";
import { useQuery } from "@tanstack/react-query";
import { snapshotQuery, schoolsQuery, programsQuery } from "@/lib/api";
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
  const data = useMemo(() =>
    snapshot.data && schools.data && daily.data && distance.data
      ? catalogData(snapshot.data, schools.data, daily.data, distance.data)
      : undefined,
  [snapshot.data, schools.data, daily.data, distance.data]);
  const queries = [snapshot, schools, daily, distance];
  return (
    <QueryBoundary queries={queries}>
      <QueryStatus queries={queries} />
      {data && children(data)}
    </QueryBoundary>
  );
}
