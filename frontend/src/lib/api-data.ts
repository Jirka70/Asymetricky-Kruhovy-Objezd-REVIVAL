import type { Snapshot } from "./data";
import type { EmployersResponse, ProgramsResponse, SchoolsResponse, ZsjRecord } from "./api";

/** API owns catalogs, ZSJ and statistics; the snapshot supplies municipality names and demography. */
export function catalogData(
  snapshot: Snapshot,
  schools: SchoolsResponse,
  daily: ProgramsResponse,
  distance: ProgramsResponse,
  zones: ZsjRecord[],
): Snapshot {
  const localSchools = new Map(snapshot.schools.map((s) => [s.id, s]));
  const demography = new Map(snapshot.zsj.map((zone) => [zone.id, zone.children]));
  const fields = new Map<string, Snapshot["fields"][number]>();
  const demand = new Map<string, Snapshot["demand"][number]>();
  for (const [form, catalog] of [["den", daily], ["dal", distance]] as const) {
    for (const program of catalog.data) {
      const field = fields.get(program.kod) ?? {
        id: program.kod, name: program.nazev, forms: [],
      };
      field.forms!.push(form);
      fields.set(program.kod, field);
      // No mapping means unknown demand, not zero vacancies.
      if (program.volna_mista !== null && program.zamestnavatelu !== null) {
        demand.set(program.kod, {
          field: program.kod,
          jobs: program.volna_mista,
          employers: program.zamestnavatelu,
        });
      }
    }
  }
  return {
    ...snapshot,
    zsj: zones.map((zone) => ({
      id: zone.kod, name: zone.nazev, municipality: zone.kod_obce ?? "",
      lat: zone.lat, lon: zone.lon, boundary: zone.boundary,
      children: demography.get(zone.kod) ?? null,
    })),
    schools: schools.features.map(({ properties: p, geometry }) => {
      const local = localSchools.get(p.redizo);
      return {
        id: p.redizo,
        name: p.nazev,
        shortName: local?.shortName ?? p.nazev,
        city: p.obec ?? local?.city ?? "",
        address: "",
        web: null,
        lon: geometry.coordinates[0],
        lat: geometry.coordinates[1],
      };
    }),
    fields: [...fields.values()].sort((a, b) => a.name.localeCompare(b.name, "cs")),
    demand: [...demand.values()].sort((a, b) => b.jobs - a.jobs || a.field.localeCompare(b.field)),
    offerings: [],
    employers: [],
  };
}

export function selectionData(
  base: Snapshot,
  field: string,
  form: string,
  schools?: SchoolsResponse,
  employers?: EmployersResponse,
): Snapshot {
  const municipalities = new Map(base.municipalities.map((m) => [m.id, m.name]));
  return {
    ...base,
    // The filtered endpoint already sums all specializations of a school/program/form.
    offerings: (schools?.features ?? []).map(({ properties: p }) => ({
      school: p.redizo,
      field,
      form,
      capacity: p.kapacita ?? null,
      applications: p.prihlasky ?? null,
      accepted: null,
    })),
    employers: (employers?.features ?? []).map(({ properties: p, geometry }) => ({
      id: p.id,
      name: p.nazev,
      ico: p.ico,
      municipality: p.kod_obce,
      city: municipalities.get(p.kod_obce) ?? p.kod_obce,
      addresspoint: null,
      importedat: employers?.meta.importovano ?? null,
      lon: geometry.coordinates[0],
      lat: geometry.coordinates[1],
      field,
      jobs: p.pocet_mist,
      professions: p.profese.map((profession) => ({
        code: profession.cz_isco3,
        name: profession.nazev,
        education: null,
        jobs: profession.pocet_mist,
        suitability: profession.vhodnost,
      })),
    })),
  };
}
