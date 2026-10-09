#!/usr/bin/env python3
r"""Generate PostgreSQL imports of estimated 2021 age counts for Karlovarský ZSJ.

Usage (Python 3.9+, standard library only):
    python3 scripts/generate_demografie_zsj.py
    python3 scripts/generate_demografie_zsj.py --help

Inputs default to the three source files in ~/Downloads. The generator does
not connect to a database. Apply its standalone SQL to a database that already
contains public."ZSJ" with a primary key on kod:
    psql -h 127.0.0.1 -U obor -d obor_na_dosah -v ON_ERROR_STOP=1 \
        -f scripts/insert_demografie_zsj.sql

Every municipality's age proportions are applied to its ZSJ populations.
Controlled rounding preserves BOTH ZSJ populations and municipal age counts.
Each cell is a floor or ceiling of its proportional estimate. A minimum-cost
flow assigns the rounding increments to minimize total absolute rounding error.
These are estimates, not observed age counts for individual ZSJ.
"""

import argparse
import csv
import hashlib
import heapq
import json
import re
from collections import defaultdict
from dataclasses import dataclass
from fractions import Fraction
from pathlib import Path

YEAR = 2021
CENSUS_DATE = "2021-03-26"
REGION_NUTS = "CZ041"
REGION_CODE = "3051"
DEFAULT_OUTPUT = Path(__file__).resolve().parent / "insert_demografie_zsj.sql"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def count_value(value):
    require(value.isascii() and value.isdigit(), f"Invalid population: {value!r}")
    result = int(value)
    require(result <= 2147483647, f"Population exceeds PostgreSQL integer: {value}")
    return result


def csv_rows(path, required):
    with path.open(encoding="utf-8-sig", newline="") as source:
        reader = csv.DictReader(source)
        require(required <= set(reader.fieldnames or []), f"Missing columns in {path}")
        for row in reader:
            require(None not in row and all(v is not None for v in row.values()),
                    f"Malformed CSV record at line {reader.line_num} in {path}")
            yield row


def age_label(text):
    match = re.fullmatch(r"(\d+)\s*-\s*(\d+)\s+(?:roky|let)", text)
    if match:
        return f"{int(match[1])}-{int(match[2])}"
    if text == "100 a více let":
        return "100+"
    raise ValueError(f"Unknown age label: {text!r}")


@dataclass
class Census:
    zsj_obec: dict
    zsj_population: dict
    groups: dict
    municipality_age: dict
    region_age: dict
    zero_missing: list


def load_sources(zsj_path, population_path, ages_path):
    with zsj_path.open(encoding="utf-8-sig") as source:
        geojson = json.load(source)
    require(geojson.get("type") == "FeatureCollection", "Expected GeoJSON FeatureCollection")
    zsj_obec = {}
    for feature in geojson["features"]:
        p = feature["properties"]
        if p["nuts3_kraj"] != REGION_NUTS:
            continue
        require(p["kod_kraj"] == REGION_CODE, "Conflicting region codes")
        for field in ("kod_zsj", "kod_obec"):
            require(isinstance(p[field], str) and re.fullmatch(r"[0-9]{6}", p[field]),
                    f"Expected six-digit text in {field}: {p[field]!r}")
        require(p["kod_zsj"] not in zsj_obec, f"Duplicate ZSJ: {p['kod_zsj']}")
        zsj_obec[p["kod_zsj"]] = p["kod_obec"]
    require(zsj_obec, "No ZSJ found for Karlovarský kraj")
    zsj_obec = dict(sorted(zsj_obec.items()))

    common = {"uzemi_cis", "uzemi_kod", "ukaz_kod", "sldb_rok", "sldb_datum", "hodnota"}
    zsj_population = {}
    for r in csv_rows(population_path, common):
        if (r["uzemi_cis"] != "47" or r["ukaz_kod"] != "3162"
                or r["sldb_rok"] != str(YEAR) or r["uzemi_kod"] not in zsj_obec):
            continue
        require(r["sldb_datum"] == CENSUS_DATE, "Unexpected census date")
        code = r["uzemi_kod"]
        require(code not in zsj_population, f"Duplicate population for ZSJ {code}")
        zsj_population[code] = count_value(r["hodnota"])
    require(zsj_population.keys() == zsj_obec.keys(),
            f"Missing ZSJ populations: {sorted(zsj_obec.keys() - zsj_population.keys())}")

    municipalities = set(zsj_obec.values())
    groups = {}
    observations = {}
    for r in csv_rows(ages_path, common | {"vek_cis", "vek_kod", "vek_txt", "pohlavi_kod"}):
        territory = (r["uzemi_cis"], r["uzemi_kod"])
        selected = (territory == ("100", REGION_CODE)
                    or (territory[0] == "43" and territory[1] in municipalities))
        if not selected or r["ukaz_kod"] != "3162" or r["sldb_rok"] != str(YEAR):
            continue
        require(r["sldb_datum"] == CENSUS_DATE, "Unexpected census date")
        require(r["pohlavi_kod"] in ("", "1", "2"), "Unexpected sex code")
        age = r["vek_kod"]
        if age:
            require(r["vek_cis"] == "1035", "Unexpected age classification")
            label = age_label(r["vek_txt"])
            require(age not in groups or groups[age] == label, f"Conflicting age label {age}")
            groups[age] = label
        key = (*territory, age, r["pohlavi_kod"])
        require(key not in observations, f"Duplicate age observation: {key}")
        observations[key] = count_value(r["hodnota"])
    expected_labels = {f"{n}-{n + 4}" for n in range(0, 100, 5)} | {"100+"}
    require(len(groups) == 21 and set(groups.values()) == expected_labels,
            "Expected exactly 21 non-overlapping age groups, 0–4 through 100+")
    groups = dict(sorted(groups.items(), key=lambda item: int(re.match(r"\d+", item[1])[0])))

    def age_counts(level, code):
        for age in ("", *groups):
            keys = [(level, code, age, sex) for sex in ("", "1", "2")]
            require(all(k in observations for k in keys), f"Incomplete ages/sexes: {code}, {age}")
            require(observations[keys[0]] == observations[keys[1]] + observations[keys[2]],
                    f"Sex totals disagree: {code}, {age}")
        counts = {age: observations[(level, code, age, "")] for age in groups}
        require(sum(counts.values()) == observations[(level, code, "", "")],
                f"Age totals disagree: {code}")
        return counts

    municipality_population = defaultdict(int)
    for zsj, municipality in zsj_obec.items():
        municipality_population[municipality] += zsj_population[zsj]
    municipality_age = {}
    zero_missing = []
    observed_municipalities = {key[1] for key in observations if key[0] == "43"}
    for municipality, population in sorted(municipality_population.items()):
        if municipality not in observed_municipalities:
            require(population == 0, f"Missing ages for populated municipality {municipality}")
            # A verified zero total and nonnegative age counts imply all ages are zero.
            municipality_age[municipality] = dict.fromkeys(groups, 0)
            zero_missing.append(municipality)
        else:
            counts = age_counts("43", municipality)
            require(sum(counts.values()) == population,
                    f"ZSJ sum differs from municipality population: {municipality}")
            municipality_age[municipality] = counts
    region_age = age_counts("100", REGION_CODE)
    for age, population in region_age.items():
        require(sum(counts[age] for counts in municipality_age.values()) == population,
                f"Municipality age sums differ from Karlovarský kraj: {age}")
    return Census(zsj_obec, zsj_population, groups, municipality_age, region_age, zero_missing)


def round_matrix(row_totals, column_totals):
    """Round proportional cells, preserving both margins and minimizing L1 error.

All calculations use integers. A residual flow edge costs denominator minus
the fractional numerator: minimizing total cost maximizes fractions rounded up.
The number of rounded-up cells is fixed by the margins, hence this also
minimizes absolute error. Sorted inputs and node IDs make ties deterministic.
"""
    require(all(isinstance(n, int) and n >= 0 for n in row_totals + column_totals),
            "Margins must be nonnegative integers")
    total = sum(row_totals)
    require(total == sum(column_totals), "Row and column margins disagree")
    nr, nc = len(row_totals), len(column_totals)
    if total == 0:
        return [[0] * nc for _ in row_totals]
    cells = [[r * c // total for c in column_totals] for r in row_totals]
    row_need = [r - sum(cells[i]) for i, r in enumerate(row_totals)]
    col_need = [c - sum(cells[i][j] for i in range(nr)) for j, c in enumerate(column_totals)]
    source, sink = nr + nc, nr + nc + 1
    graph = [[] for _ in range(sink + 1)]

    def edge(a, b, capacity, cost):
        forward = [b, len(graph[b]), capacity, cost]
        reverse = [a, len(graph[a]), 0, -cost]
        graph[a].append(forward)
        graph[b].append(reverse)
        return forward

    for i, need in enumerate(row_need):
        edge(source, i, need, 0)
    fractional = []
    for i, r in enumerate(row_totals):
        for j, c in enumerate(column_totals):
            remainder = r * c % total
            if remainder:
                fractional.append((i, j, edge(i, nr + j, 1, total - remainder)))
    for j, need in enumerate(col_need):
        edge(nr + j, sink, need, 0)

    potential = [0] * len(graph)
    remaining = sum(row_need)
    while remaining:
        distance = [float("inf")] * len(graph)
        previous = [None] * len(graph)
        distance[source] = 0
        queue = [(0, source)]
        while queue:
            dist, node = heapq.heappop(queue)
            if dist != distance[node]:
                continue
            for index, (neighbor, reverse, capacity, cost) in enumerate(graph[node]):
                if not capacity:
                    continue
                candidate = dist + cost + potential[node] - potential[neighbor]
                if candidate < distance[neighbor]:
                    distance[neighbor] = candidate
                    previous[neighbor] = (node, index)
                    heapq.heappush(queue, (candidate, neighbor))
        require(previous[sink] is not None, "No feasible controlled rounding")
        for node, dist in enumerate(distance):
            if dist != float("inf"):
                potential[node] += dist
        amount = remaining
        node = sink
        while node != source:
            parent, index = previous[node]
            amount = min(amount, graph[parent][index][2])
            node = parent
        node = sink
        while node != source:
            parent, index = previous[node]
            connection = graph[parent][index]
            connection[2] -= amount
            graph[node][connection[1]][2] += amount
            node = parent
        remaining -= amount
    for i, j, connection in fractional:
        cells[i][j] += 1 - connection[2]
    require([sum(row) for row in cells] == row_totals, "Rounded row totals disagree")
    require([sum(cells[i][j] for i in range(nr)) for j in range(nc)] == column_totals,
            "Rounded column totals disagree")
    return cells


def estimate(census):
    by_municipality = defaultdict(list)
    for zsj, municipality in census.zsj_obec.items():
        by_municipality[municipality].append(zsj)
    records = []
    maximum_error = Fraction(0)
    for municipality, zsj_codes in sorted(by_municipality.items()):
        populations = [census.zsj_population[zsj] for zsj in zsj_codes]
        ages = [census.municipality_age[municipality][age] for age in census.groups]
        cells = round_matrix(populations, ages)
        total = sum(populations)
        for i, zsj in enumerate(zsj_codes):
            for j, age in enumerate(census.groups):
                value = cells[i][j]
                if total:
                    error = Fraction(abs(value * total - populations[i] * ages[j]), total)
                    require(error < 1, f"Rounding error exceeds one person: {zsj}, {age}")
                    maximum_error = max(maximum_error, error)
                records.append((zsj, YEAR, age, value))
    records.sort()
    require(len(records) == len(census.zsj_obec) * len(census.groups), "Incomplete output")
    require(len({r[:3] for r in records}) == len(records), "Duplicate output keys")
    require(sum(r[3] for r in records) == sum(census.zsj_population.values()), "Wrong output total")
    return records, maximum_error


def sql_literal(value):
    if isinstance(value, int):
        return str(value)
    require("\x00" not in value, "NUL is not supported in PostgreSQL text")
    return "'" + value.replace("'", "''") + "'"


def inserts(table, columns, rows):
    rows = list(rows)
    parts = []
    for offset in range(0, len(rows), 1000):
        values = ",\n".join("    (" + ", ".join(map(sql_literal, r)) + ")"
                            for r in rows[offset:offset + 1000])
        parts.append(f"INSERT INTO {table} ({columns}) VALUES\n{values};\n")
    return "\n".join(parts)


def source_hash(path):
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def build_sql(census, records, sources):
    metadata = "\n".join(f"-- Zdroj: {path.name}; SHA-256: {source_hash(path)}" for path in sources)
    source_zsj = inserts("pg_temp.demo_zsj_source", "kod_zsj, kod_obce, populace",
                         ((zsj, obec, census.zsj_population[zsj]) for zsj, obec in census.zsj_obec.items()))
    source_groups = inserts("pg_temp.demo_groups_source", "id, vek_od_do", census.groups.items())
    source_ages = inserts("pg_temp.demo_obec_source", "kod_obce, demo_skupina, populace",
                          ((obec, age, count) for obec, ages in census.municipality_age.items()
                           for age, count in ages.items()))
    source_data = inserts("pg_temp.demo_estimate_source", "kod_zsj, rok, demo_skupina, populace", records)
    population = sum(census.zsj_population.values())
    return f'''-- ODHAD věkové struktury ZSJ v Karlovarském kraji (CZ041), SLDB {YEAR}.
{metadata}
-- Vygenerováno pomocí scripts/generate_demografie_zsj.py (Python, standardní knihovna).
-- {len(census.zsj_obec)} ZSJ × {len(census.groups)} skupin = {len(records)} záznamů; {population} obyvatel.
-- Obce a vojenské újezdy ve vazebníku: {len(census.municipality_age)}.
-- Území bez věkových dat s ověřenou nulovou populací (všechny skupiny = 0): {', '.join(census.zero_missing) or 'žádná'}.
-- ZSJ → obec pochází z atributů kod_zsj/kod_obec v zsj.txt; filtr nuts3_kraj=CZ041.
-- Populace ZSJ: uzemi_cis=47, ukaz_kod=3162. Věk obce: uzemi_cis=43,
-- ukaz_kod=3162, vek_cis=1035; prázdné pohlavi_kod znamená obě pohlaví celkem.
-- Odhad = populace ZSJ × věkový podíl obce, rozhodné datum {CENSUS_DATE}.
-- Řízené zaokrouhlení zachovává populaci každé ZSJ i každou věkovou skupinu obce.
-- Každá buňka je dolní nebo horní celé zaokrouhlení odhadu; chyba je menší než 1 osoba.
-- Z přípustných zaokrouhlení se vybírá minimum součtu absolutních odchylek.
-- Jde o modelový odhad, nikoli zjištěné věkové složení ZSJ ani aktuální populaci.
-- Vyžaduje existující public."ZSJ" s PK na kod; doplní textový sloupec kod_obce.
-- Existující hodnoty se nepřepisují; při neshodě se celý import vrátí zpět.
-- Kontroly se týkají importovaných ZSJ a roku {YEAR}. Jiné roky/území se nemění.
-- SQL je samostatné, zdrojové soubory při spuštění nepotřebuje.
-- psql -h 127.0.0.1 -U obor -d obor_na_dosah -v ON_ERROR_STOP=1 -f scripts/insert_demografie_zsj.sql

BEGIN;
SET LOCAL client_encoding = 'UTF8';
SET LOCAL standard_conforming_strings = on;

ALTER TABLE public."ZSJ" ADD COLUMN IF NOT EXISTS kod_obce text;
CREATE TABLE IF NOT EXISTS public."DEMO_SKUPINA" (
    id text PRIMARY KEY,
    vek_od_do text NOT NULL
);
CREATE TABLE IF NOT EXISTS public."DATA_DEMOGRAFIE_ZSJ" (
    kod_zsj text NOT NULL REFERENCES public."ZSJ" (kod),
    rok integer NOT NULL,
    demo_skupina text NOT NULL REFERENCES public."DEMO_SKUPINA" (id),
    populace integer NOT NULL CHECK (populace >= 0),
    PRIMARY KEY (kod_zsj, rok, demo_skupina)
);
COMMENT ON TABLE public."DATA_DEMOGRAFIE_ZSJ" IS
    'Modelový odhad věkových počtů ZSJ z podílů obce a populace ZSJ (SLDB 2021). Řízené zaokrouhlení zachovává součty za ZSJ a obec/věkovou skupinu. Nejde o zjištěnou věkovou strukturu ZSJ.';
COMMENT ON COLUMN public."DATA_DEMOGRAFIE_ZSJ".populace IS
    'Odhad počtu osob, nikoli přímé pozorování; všechny věkové skupiny a obě pohlaví celkem.';
COMMENT ON COLUMN public."DEMO_SKUPINA".id IS 'Textový kód věkové skupiny ČSÚ, číselník 1035.';

CREATE TEMP TABLE demo_zsj_source (
    kod_zsj text PRIMARY KEY, kod_obce text NOT NULL, populace integer NOT NULL
) ON COMMIT DROP;
CREATE TEMP TABLE demo_groups_source (
    id text PRIMARY KEY, vek_od_do text NOT NULL
) ON COMMIT DROP;
CREATE TEMP TABLE demo_obec_source (
    kod_obce text NOT NULL, demo_skupina text NOT NULL, populace integer NOT NULL,
    PRIMARY KEY (kod_obce, demo_skupina)
) ON COMMIT DROP;
CREATE TEMP TABLE demo_estimate_source (
    kod_zsj text NOT NULL, rok integer NOT NULL, demo_skupina text NOT NULL,
    populace integer NOT NULL CHECK (populace >= 0), PRIMARY KEY (kod_zsj, rok, demo_skupina)
) ON COMMIT DROP;

{source_zsj}
{source_groups}
{source_ages}
{source_data}
DO $check_mapping$
BEGIN
    IF EXISTS (
        SELECT 1 FROM pg_temp.demo_zsj_source s
        LEFT JOIN public."ZSJ" z ON z.kod = s.kod_zsj
        WHERE z.kod IS NULL OR (z.kod_obce IS NOT NULL AND z.kod_obce <> s.kod_obce)
    ) THEN
        RAISE EXCEPTION 'Chybí ZSJ nebo se její kod_obce liší od zdrojového vazebníku.';
    END IF;
END;
$check_mapping$;

UPDATE public."ZSJ" z SET kod_obce = s.kod_obce
FROM pg_temp.demo_zsj_source s WHERE z.kod = s.kod_zsj AND z.kod_obce IS NULL;
INSERT INTO public."DEMO_SKUPINA" (id, vek_od_do)
SELECT id, vek_od_do FROM pg_temp.demo_groups_source ON CONFLICT (id) DO NOTHING;
INSERT INTO public."DATA_DEMOGRAFIE_ZSJ" (kod_zsj, rok, demo_skupina, populace)
SELECT kod_zsj, rok, demo_skupina, populace FROM pg_temp.demo_estimate_source
ON CONFLICT (kod_zsj, rok, demo_skupina) DO NOTHING;

DO $check_import$
DECLARE
    imported_count bigint;
BEGIN
    IF (SELECT count(*) FROM pg_temp.demo_zsj_source) <> {len(census.zsj_obec)}
       OR (SELECT count(*) FROM pg_temp.demo_groups_source) <> {len(census.groups)}
       OR (SELECT count(*) FROM pg_temp.demo_obec_source) <> {len(census.municipality_age) * len(census.groups)}
       OR (SELECT count(*) FROM pg_temp.demo_estimate_source) <> {len(records)} THEN
        RAISE EXCEPTION 'Nesouhlasí počty vložených zdrojových záznamů.';
    END IF;
    IF EXISTS (
        SELECT id, vek_od_do FROM pg_temp.demo_groups_source
        EXCEPT SELECT id, vek_od_do FROM public."DEMO_SKUPINA"
    ) OR EXISTS (
        SELECT kod_zsj, rok, demo_skupina, populace FROM pg_temp.demo_estimate_source
        EXCEPT SELECT kod_zsj, rok, demo_skupina, populace FROM public."DATA_DEMOGRAFIE_ZSJ"
    ) THEN
        RAISE EXCEPTION 'Importované hodnoty se liší od připraveného odhadu.';
    END IF;
    SELECT count(*) INTO imported_count FROM public."DATA_DEMOGRAFIE_ZSJ" d
    JOIN pg_temp.demo_zsj_source s ON s.kod_zsj = d.kod_zsj WHERE d.rok = {YEAR};
    IF imported_count <> {len(records)} THEN
        RAISE EXCEPTION 'Počet demografických záznamů je %, očekáváno {len(records)}.', imported_count;
    END IF;
    IF EXISTS (
        SELECT s.kod_zsj FROM pg_temp.demo_zsj_source s
        LEFT JOIN public."DATA_DEMOGRAFIE_ZSJ" d ON d.kod_zsj = s.kod_zsj AND d.rok = {YEAR}
        GROUP BY s.kod_zsj, s.populace
        HAVING count(d.demo_skupina) <> {len(census.groups)} OR sum(d.populace) <> s.populace
    ) THEN
        RAISE EXCEPTION 'Součet nebo počet věkových skupin nesouhlasí s populací ZSJ.';
    END IF;
    IF EXISTS (
        SELECT kod_obce, demo_skupina, populace::bigint FROM pg_temp.demo_obec_source
        EXCEPT
        SELECT s.kod_obce, d.demo_skupina, sum(d.populace)
        FROM public."DATA_DEMOGRAFIE_ZSJ" d
        JOIN pg_temp.demo_zsj_source s ON s.kod_zsj = d.kod_zsj
        WHERE d.rok = {YEAR} GROUP BY s.kod_obce, d.demo_skupina
    ) THEN
        RAISE EXCEPTION 'Věkové součty obcí neodpovídají zdrojovým datům.';
    END IF;
    RAISE NOTICE 'Kontrola OK: {len(census.zsj_obec)} ZSJ, {len(census.groups)} skupin, % záznamů, {population} obyvatel; součty ZSJ i obcí souhlasí.', imported_count;
END;
$check_import$;

COMMIT;
'''


def main():
    downloads = Path.home() / "Downloads"
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--zsj", type=Path, default=downloads / "zsj.txt")
    parser.add_argument("--population", type=Path, default=downloads / "sldb2021_obyv_byt_cob_zsj.csv")
    parser.add_argument("--ages", type=Path, default=downloads / "sldb2021_vek5_pohlavi.csv")
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    sources = [args.zsj, args.population, args.ages]
    try:
        require(all(args.output.resolve() != path.resolve() for path in sources),
                "Output must not overwrite a source file")
        census = load_sources(*sources)
        records, maximum_error = estimate(census)
        sql = build_sql(census, records, sources)
        args.output.write_text(sql, encoding="utf-8")
    except (OSError, ValueError, KeyError, TypeError, csv.Error) as error:
        parser.exit(1, f"Error: {error}\n")
    print(json.dumps({
        "output": str(args.output), "year": YEAR, "region": REGION_NUTS,
        "zsj": len(census.zsj_obec), "age_groups": len(census.groups), "records": len(records),
        "municipalities_with_observed_ages": len(census.municipality_age) - len(census.zero_missing),
        "missing_zero_population_municipalities": census.zero_missing,
        "population": sum(census.zsj_population.values()),
        "zero_population_zsj": sum(n == 0 for n in census.zsj_population.values()),
        "age_15_19": sum(n for age, n in census.region_age.items() if census.groups[age] == "15-19"),
        "maximum_absolute_rounding_error": float(maximum_error),
        "zsj_municipality_and_regional_totals_verified": True,
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
