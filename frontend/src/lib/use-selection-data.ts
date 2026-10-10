"use client";
import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { employersQuery, schoolsQuery } from "./api";
import { selectionData } from "./api-data";
import type { Snapshot } from "./data";

export function useSelectionData(base: Snapshot, field: string, form: string) {
  const schools = useQuery(schoolsQuery(field, form));
  const employers = useQuery(employersQuery(field));
  const data = useMemo(
    () => selectionData(base, field, form, schools.data, employers.data),
    [base, field, form, schools.data, employers.data],
  );
  const employersNotice = !employers.data
    ? employers.error ? "Pracoviště se nepodařilo načíst." : "Načítání pracovišť…"
    : !employers.data.meta.mapovani ? "Pro tento obor nemáme mapování na profese." : undefined;
  return { data, schools, employers, employersNotice };
}
