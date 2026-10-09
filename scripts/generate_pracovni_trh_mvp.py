#!/usr/bin/env python3
"""Generate four standalone SQL imports from local jobs and NSP/RUIAN snapshots.

No database or network access. Run fetch_pracovni_trh_mvp.py first to refresh
external evidence. SQL is deterministic for the same source snapshots.
"""

import csv
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "datasety"
OUT = ROOT / "scripts/sql"
JOBS = DATA / "volna-mista_karlovarsky_kraj.json"
SCHOOLS = DATA / "PZ2026_kolo1_skolobory_prihlasky_karlovarsky_kraj.csv"
NSP = DATA / "nsp_mapovani_mvp.json"
RUIAN = DATA / "ruian_pracoviste_mvp.json"
SQL_NAMES = ["insert_zamestnavatele.sql", "insert_profesni_skupiny.sql",
             "insert_poptavka_profesi.sql", "insert_obor_profese.sql"]


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load(path):
    return json.loads(path.read_text(encoding="utf-8"))


def code(value, prefix):
    raw = (value or {}).get("id", "")
    require(raw.startswith(prefix + "/"), f"Missing/invalid {prefix} code: {raw!r}")
    return raw.split("/", 1)[1]


def workplace_key(ico, workplace):
    address = workplace["adresa"]
    if address.get("kodAdresnihoMista"):
        return (ico, "ruian", str(address["kodAdresnihoMista"]))
    # No fake precise address or municipality centroid. Distinguish workplaces
    # without an address ID by all original address fields and workplace name.
    return (ico, "fallback", workplace.get("nazev"),
            json.dumps(address, sort_keys=True, ensure_ascii=False, separators=(",", ":")))


def workplace_id(key):
    # Stable signed-bigint ID, independent of row order or future additions.
    data = json.dumps(key, ensure_ascii=False, separators=(",", ":")).encode()
    return int.from_bytes(hashlib.sha256(data).digest()[:8], "big") & ((1 << 63) - 1)


def build_model(jobs=None, nsp=None, ruian=None):
    jobs = load(JOBS)["polozky"] if jobs is None else jobs
    nsp = load(NSP) if nsp is None else nsp
    ruian = load(RUIAN) if ruian is None else ruian
    require(nsp["jobs_sha256"] == sha(JOBS), "NSP snapshot belongs to different jobs")
    with SCHOOLS.open(encoding="utf-8-sig", newline="") as source:
        programs = {r["KKOV"]: r["OBOR - NÁZEV"] for r in csv.DictReader(source)}

    coords = {}
    rejected_coords = []
    for page in ruian["pages"]:
        response = page["response"]
        require(not response.get("exceededTransferLimit"), "Truncated RUIAN response")
        require(response["spatialReference"].get("wkid") == 4326, "RUIAN must use WGS84")
        for feature in response["features"]:
            a, p = feature["attributes"], feature.get("geometry")
            key = str(a["kod"])
            require(key not in coords, f"Duplicate RUIAN point {key}")
            if not p or a.get("platido") is not None or a.get("nespravny") not in (None, "N", "0"):
                rejected_coords.append(key)
                continue
            require(48 <= p["y"] <= 52 and 12 <= p["x"] <= 19,
                    f"Invalid Czech coordinates: {key}")
            coords[key] = (round(p["y"], 7), round(p["x"], 7))

    employers, keys, demand, ids = {}, {}, Counter(), set()
    addresses, group_set = set(), set()
    job_groups = Counter()
    for job in jobs:
        portal = job["portalId"]
        require(portal not in ids, f"Duplicate job portalId: {portal}")
        require(job["id"] == f"VolneMisto/{portal}", "Inconsistent job ID")
        ids.add(portal)
        places = job["mistoVykonuPrace"]["pracoviste"]
        require(len(places) == 1, f"Job {portal}: multiple workplaces need explicit allocation")
        workplace = places[0]
        a = workplace["adresa"]
        require(code(a["kraj"], "Kraj") == "51", f"Job {portal}: outside region")
        ico, name = job["zamestnavatel"]["ico"], job["zamestnavatel"]["nazev"]
        require(re.fullmatch(r"\d{8}", ico, re.ASCII) and name, "Invalid employer")
        key = workplace_key(ico, workplace)
        eid = workplace_id(key)
        require(eid > 0 and (eid not in keys or keys[eid] == key), "Workplace hash collision")
        keys[eid] = key
        address = str(a["kodAdresnihoMista"]) if a.get("kodAdresnihoMista") else None
        municipality = code(a["obec"], "Obec")
        require(re.fullmatch(r"\d{6}", municipality, re.ASCII), "Invalid municipality")
        if address:
            addresses.add(address)
        lat, lon = coords.get(address, (None, None))
        row = (eid, ico, name, address, municipality, lat, lon)
        require(eid not in employers or employers[eid] == row, f"Conflicting workplace {eid}")
        employers[eid] = row
        full_isco = code(job["profeseCzIsco"], "CzIsco")
        require(re.fullmatch(r"\d{3,5}", full_isco, re.ASCII), "Invalid ISCO")
        group = full_isco[:3]
        education = code(job["minPozadovaneVzdelani"], "VzdelaniDetailniKategorie")
        count = job["pocetMist"]
        require(type(count) is int and count >= 0, "Invalid number of positions")
        demand[(eid, group, education)] += count
        job_groups[group] += 1
        group_set.add(group)
    require(addresses == set(ruian["requested_address_codes"]), "RUIAN snapshot does not cover job addresses")
    require(set(coords) <= addresses, "Unexpected RUIAN addresses")

    groups = {g["code"]: g for g in nsp["groups"]}
    require(len(groups) == len(nsp["groups"]) and set(groups) == group_set,
            "NSP snapshot does not cover exactly the job groups")
    dictionary = nsp["dictionary"]["response"]["data"]
    units = {u["slug"]: u for u in nsp["work_units"]}
    require(len(units) == len(nsp["work_units"]), "Duplicate NSP work units")
    require(set(units) == {u["slug"] for g in groups.values() for u in g["work_units"]},
            "Missing NSP work-unit details")
    suitability = nsp["suitability"]["response"]["data"]
    require({r["id"]: r["constant"] for r in suitability} == {1: "BEST", 2: "APPROPRIATE"},
            "NSP suitability semantics changed")
    mapping, evidence = {}, defaultdict(list)
    without_units, without_rvp, outside_programs = [], [], []
    for g, group in sorted(groups.items()):
        expected_ids = {r["id"] for r in dictionary if r["code"].startswith(g)}
        require(set(group["queried_isco_ids"]) == expected_ids, f"Incomplete ISCO expansion: {g}")
        if not group["work_units"]:
            without_units.append(g)
        any_rvp = False
        for work in group["work_units"]:
            unit = units[work["slug"]]
            iscos = [x["isco"]["code"] for x in unit["isco"]["response"]["data"]]
            require(any(i.startswith(g) for i in iscos), f"NSP returned unrelated unit: {g}/{unit['slug']}")
            for relation in unit["education"]["response"]["data"]["rvps"]:
                any_rvp = True
                program, level = relation["rvp"]["code"], relation["kkovSuitabilityLevel"]
                require(type(level) is int and level in (1, 2), "Invalid suitability")
                if program not in programs:
                    continue
                pair = (g, program)
                mapping[pair] = min(mapping.get(pair, 2), level)
                evidence[pair].append({"slug": unit["slug"], "cz_isco": iscos, "vhodnost": level})
        if group["work_units"] and not any_rvp:
            without_rvp.append(g)
        if any_rvp and not any(k[0] == g for k in mapping):
            outside_programs.append(g)

    report = {
        "sources_sha256": {str(p.relative_to(ROOT)): sha(p) for p in (JOBS, SCHOOLS, NSP, RUIAN)},
        "jobs": len(jobs), "positions": sum(demand.values()),
        "employers_distinct_ico": len({r[1] for r in employers.values()}),
        "workplaces": len(employers), "municipalities": len({r[4] for r in employers.values()}),
        "workplaces_with_coordinates": sum(r[5] is not None for r in employers.values()),
        "positions_without_coordinates": sum(n for (e, _, _), n in demand.items() if employers[e][5] is None),
        "workplaces_without_address_code": sum(r[3] is None for r in employers.values()),
        "address_codes_not_geocoded": sorted(addresses - set(coords)),
        "rejected_ruian_coordinates": sorted(rejected_coords),
        "profession_groups": len(groups), "demand_rows": len(demand),
        "mapping_pairs": len(mapping), "mapped_groups": len({k[0] for k in mapping}),
        "positions_in_unmapped_groups": sum(n for (_, g, _), n in demand.items()
                                           if g not in {k[0] for k in mapping}),
        "mapped_school_programs": len({k[1] for k in mapping}), "school_programs": len(programs),
        "groups_without_nsp_work_units": without_units,
        "groups_without_rvp_in_nsp": without_rvp,
        "groups_with_rvp_only_outside_local_programs": outside_programs,
        "programs_without_mapping": sorted(set(programs) - {k[1] for k in mapping}),
        "mapping_evidence": [{"cz_isco3": g, "kod_oboru": p, "vhodnost": mapping[(g, p)],
                              "work_units": sorted(evidence[(g, p)], key=lambda x: x["slug"])}
                             for g, p in sorted(mapping)],
        "group_totals": [{"cz_isco3": g, "nazev": groups[g]["title"],
                          "pocet_mist": sum(n for (_, c, _), n in demand.items() if c == g),
                          "pocet_inzeratu": job_groups[g],
                          "pocet_zamestnavatelu": len({employers[e][1] for e, c, _ in demand if c == g}),
                          "pocet_oboru": sum(c == g for c, _ in mapping)} for g in sorted(groups)],
        "notes": [
            "Only current source snapshot; no history, trend or workforce size.",
            "All 3/4/5-digit ISCO codes are grouped by their first three digits.",
            "NSP matching uses every descendant ID, including duplicate codes under distinct IDs.",
            "Only exact RVP codes present in the school CSV are mapped; KKOV/NSK codes are not converted.",
            "Suitability is best evidence for at least one occupation in the group, not every position.",
            "Missing coordinates remain NULL; municipality/district-wide work is not a precise location.",
            "importovano_at is set on SQL execution, not the vacancy publication/source reference date.",
            "SQL generated only. Verification must use an isolated disposable container.",
        ],
    }
    return {
        "employers": sorted(employers.values()),
        "groups": [(g, groups[g]["title"]) for g in sorted(groups)],
        "demand": [(*key, demand[key]) for key in sorted(demand)],
        "mapping": [(g, p, mapping[(g, p)]) for g, p in sorted(mapping)],
        "report": report,
    }


def literal(value):
    if value is None:
        return "NULL"
    if isinstance(value, (int, float)):
        return str(value)
    require("\x00" not in value, "NUL is invalid in PostgreSQL text")
    return "'" + value.replace("'", "''") + "'"


def values(rows):
    return ",\n".join("    (" + ", ".join(literal(x) for x in r) + ")" for r in rows)


def check(condition, message):
    return f"DO $$ BEGIN IF NOT ({condition}) THEN RAISE EXCEPTION {literal(message)}; END IF; END $$;\n"


def write_sql(name, model, body):
    header = "-- MVP pracovního trhu. Vygenerováno; NEAPLIKOVÁNO na projektovou DB.\n"
    header += "-- Generátor: scripts/generate_pracovni_trh_mvp.py (bez připojení k DB).\n"
    header += "-- Pořadí: " + " -> ".join(SQL_NAMES) + ".\n"
    header += "-- OBORY musí být předem naplněné. Použijte psql -X -v ON_ERROR_STOP=1 -f SOUBOR.\n"
    for path, digest in model["report"]["sources_sha256"].items():
        header += f"-- SHA-256 {path}: {digest}\n"
    header += "BEGIN;\nSET LOCAL client_encoding = 'UTF8';\nSET LOCAL standard_conforming_strings = on;\n\n"
    (OUT / name).write_text(header + body + "\nCOMMIT;\n", encoding="utf-8")


def generate(model):
    OUT.mkdir(exist_ok=True)
    write_sql(SQL_NAMES[0], model, '''-- Jeden řádek = IČO + pracoviště, nikoli celá firma.
-- ID je stabilní 63bitový SHA-256 otisk IČO a adresního kódu; bez kódu se
-- používá celý zdrojový popis adresy a název pracoviště. Kolize se kontrolují.
-- Souřadnice: ČÚZK RÚIAN, WGS84; NULL nikdy nenahrazujeme středem obce.
CREATE TABLE IF NOT EXISTS public."ZAMESTNAVATELE" (
    id bigint PRIMARY KEY CHECK (id > 0),
    ico text NOT NULL CHECK (ico ~ '^[0-9]{8}$'),
    nazev text NOT NULL,
    kod_adresniho_mista text NULL,
    kod_obce text NOT NULL CHECK (kod_obce ~ '^[0-9]{6}$'),
    lat double precision NULL CHECK (lat BETWEEN -90 AND 90),
    lon double precision NULL CHECK (lon BETWEEN -180 AND 180),
    CHECK ((lat IS NULL) = (lon IS NULL)),
    UNIQUE (ico, kod_adresniho_mista)
);
CREATE INDEX IF NOT EXISTS zamestnavatele_kod_obce_idx ON public."ZAMESTNAVATELE" (kod_obce);
CREATE TEMP TABLE import_zamestnavatele (LIKE public."ZAMESTNAVATELE" INCLUDING CONSTRAINTS) ON COMMIT DROP;
INSERT INTO import_zamestnavatele (id, ico, nazev, kod_adresniho_mista, kod_obce, lat, lon) VALUES
''' + values(model["employers"]) + ";\n" + check('''NOT EXISTS (
    SELECT 1 FROM import_zamestnavatele s JOIN public."ZAMESTNAVATELE" t USING (id)
    WHERE s.ico <> t.ico OR s.kod_adresniho_mista IS DISTINCT FROM t.kod_adresniho_mista
)''', "Kolize ID pracoviště; import byl vrácen zpět") + '''INSERT INTO public."ZAMESTNAVATELE"
SELECT * FROM import_zamestnavatele
ON CONFLICT (id) DO UPDATE SET
    nazev = EXCLUDED.nazev, kod_obce = EXCLUDED.kod_obce,
    lat = EXCLUDED.lat, lon = EXCLUDED.lon;
''' + check('''NOT EXISTS (SELECT * FROM import_zamestnavatele EXCEPT
                         SELECT * FROM public."ZAMESTNAVATELE")''', "Nesoulad importu zaměstnavatelů"))

    write_sql(SQL_NAMES[1], model, '''CREATE TABLE IF NOT EXISTS public."PROFESNI_SKUPINY" (
    cz_isco3 text PRIMARY KEY CHECK (cz_isco3 ~ '^[0-9]{3}$'),
    nazev text NOT NULL
);
INSERT INTO public."PROFESNI_SKUPINY" (cz_isco3, nazev) VALUES
''' + values(model["groups"]) + '''
ON CONFLICT (cz_isco3) DO UPDATE SET nazev = EXCLUDED.nazev;
''')

    write_sql(SQL_NAMES[2], model, '''-- Nahrazuje celý stav poptávky v jedné transakci. Staré exporty se nesčítají.
-- Čas je okamžik spuštění importu, nikoli datum vzniku nebo ověření inzerátů.
-- min_vzdelani zachovává kód VzdelaniDetailniKategorie bez prefixu.
CREATE TABLE IF NOT EXISTS public."POPTAVKA_PROFESI" (
    zamestnavatel_id bigint NOT NULL REFERENCES public."ZAMESTNAVATELE" (id),
    cz_isco3 text NOT NULL REFERENCES public."PROFESNI_SKUPINY" (cz_isco3),
    min_vzdelani text NOT NULL,
    pocet_mist integer NOT NULL CHECK (pocet_mist >= 0),
    importovano_at timestamptz NOT NULL,
    PRIMARY KEY (zamestnavatel_id, cz_isco3, min_vzdelani)
);
CREATE INDEX IF NOT EXISTS poptavka_profesi_cz_isco3_idx ON public."POPTAVKA_PROFESI" (cz_isco3);
CREATE TEMP TABLE import_poptavka (
    zamestnavatel_id bigint, cz_isco3 text, min_vzdelani text, pocet_mist integer,
    PRIMARY KEY (zamestnavatel_id, cz_isco3, min_vzdelani)
) ON COMMIT DROP;
INSERT INTO import_poptavka VALUES
''' + values(model["demand"]) + ''';
DELETE FROM public."POPTAVKA_PROFESI";
INSERT INTO public."POPTAVKA_PROFESI"
SELECT *, transaction_timestamp() FROM import_poptavka;
-- Odstraní pracoviště, která už nejsou v tomto úplném snapshotu.
DELETE FROM public."ZAMESTNAVATELE" z
WHERE NOT EXISTS (SELECT 1 FROM public."POPTAVKA_PROFESI" p WHERE p.zamestnavatel_id = z.id);
''' + check(f'''(SELECT COUNT(*) FROM public."POPTAVKA_PROFESI") = {len(model['demand'])}
AND (SELECT SUM(pocet_mist) FROM public."POPTAVKA_PROFESI") = {model['report']['positions']}
AND (SELECT COUNT(*) FROM public."ZAMESTNAVATELE") = {len(model['employers'])}''', "Nesouhlasí kontrolní součty poptávky"))

    write_sql(SQL_NAMES[3], model, '''-- Pouze přesné RVP kódy z OBORY; žádné odhady převodu KKOV/NSK.
-- Vhodnost: 1 = nejvhodnější, 2 = vhodná (NSP).
-- Hodnota je nejlepší doložená vazba ALESPOŇ JEDNOHO povolání ve skupině.
-- Nepopisuje kvalifikaci pro každé místo ve skupině ani počet absolventů.
CREATE TABLE IF NOT EXISTS public."OBOR_PROFESE" (
    cz_isco3 text NOT NULL REFERENCES public."PROFESNI_SKUPINY" (cz_isco3),
    kod_oboru text NOT NULL REFERENCES public."OBORY" (kod),
    vhodnost smallint NOT NULL CHECK (vhodnost IN (1, 2)),
    PRIMARY KEY (cz_isco3, kod_oboru)
);
CREATE INDEX IF NOT EXISTS obor_profese_kod_oboru_idx ON public."OBOR_PROFESE" (kod_oboru);
CREATE TEMP TABLE import_obor_profese (cz_isco3 text, kod_oboru text, vhodnost smallint,
    PRIMARY KEY (cz_isco3, kod_oboru)) ON COMMIT DROP;
INSERT INTO import_obor_profese VALUES
''' + values(model["mapping"]) + ''';
DELETE FROM public."OBOR_PROFESE";
INSERT INTO public."OBOR_PROFESE" SELECT * FROM import_obor_profese;
''' + check(f'''(SELECT COUNT(*) FROM public."OBOR_PROFESE") = {len(model['mapping'])}''', "Nesouhlasí počet vazeb obor–profese"))
    (OUT / "pracovni_trh_mvp_report.json").write_text(
        json.dumps(model["report"], ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def main():
    model = build_model()
    generate(model)
    print(json.dumps({k: v for k, v in model["report"].items()
                      if not isinstance(v, (dict, list))}, ensure_ascii=False, indent=2))
    print("SQL files generated. No database connection was made.")


if __name__ == "__main__":
    main()
