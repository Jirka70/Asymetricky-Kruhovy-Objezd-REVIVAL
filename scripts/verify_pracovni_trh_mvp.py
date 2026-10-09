#!/usr/bin/env python3
"""Verify generated SQL against independent source queries in a disposable DB.

NEVER connects to the project DB. Starts a new network-disabled Docker container
without ports/host volumes, stores PostgreSQL in tmpfs, then removes it in finally.
Requires the already installed postgis/postgis:18-3.6 image; does not pull images.
"""

import copy
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import subprocess
import time
import uuid

import generate_pracovni_trh_mvp as generator

ROOT = Path(__file__).resolve().parents[1]
REPORT = ROOT / "scripts/sql/pracovni_trh_mvp_verification.json"


def run(command, source=None, expect_success=True):
    result = subprocess.run(command, input=source, capture_output=True, text=True, encoding="utf-8")
    if expect_success and result.returncode:
        raise RuntimeError(result.stderr.strip())
    return result


def quote(value):
    # Independent SQL quoting for reference data, never a shell command.
    return "'" + value.replace("'", "''") + "'"


def verify():
    report = {"status": "running", "started_at": datetime.now(timezone.utc).isoformat(),
              "target": "new disposable container, network=none, PostgreSQL storage=tmpfs",
              "project_database_accessed": False, "checks": []}
    name = "codex-pracovni-trh-check-" + uuid.uuid4().hex[:10]
    created = False
    checks = report["checks"]

    def ok(label):
        checks.append(label)
        print("PASS: " + label, flush=True)

    def sql(source, expect_success=True):
        return run(["docker", "exec", "-i", name, "psql", "-X", "-qAt",
                    "-v", "ON_ERROR_STOP=1", "-h", "127.0.0.1", "-U", "verify", "-d", "verify"],
                   source, expect_success)

    def scalar(source):
        return sql(source).stdout.strip()

    def expect_error(source, expected):
        result = sql(source, expect_success=False)
        assert result.returncode != 0 and expected in result.stderr, result.stderr

    def fingerprint():
        result = {}
        for table, fields in [
            ('"ZAMESTNAVATELE"', "*"), ('"PROFESNI_SKUPINY"', "*"),
            ('"POPTAVKA_PROFESI"', "zamestnavatel_id, cz_isco3, min_vzdelani, pocet_mist"),
            ('"OBOR_PROFESE"', "*"),
        ]:
            rows = scalar(f"SELECT row_to_json(r)::text FROM (SELECT {fields} FROM {table}) r ORDER BY row_to_json(r)::text;")
            result[table] = hashlib.sha256(rows.encode()).hexdigest()
        return result

    try:
        model = generator.build_model()
        files = [ROOT / "scripts/sql" / n for n in generator.SQL_NAMES]
        before = {p.name: generator.sha(p) for p in files}
        generator.generate(model)
        assert before == {p.name: generator.sha(p) for p in files}
        report["sql_sha256"] = before
        ok("SQL regeneration is byte-for-byte deterministic")

        jobs = generator.load(generator.JOBS)["polozky"]
        bad_jobs = jobs + [copy.deepcopy(jobs[0])]
        try:
            generator.build_model(jobs=bad_jobs)
            raise AssertionError("Duplicate source job accepted")
        except ValueError as error:
            assert "Duplicate job" in str(error)
        bad_jobs = copy.deepcopy(jobs)
        bad_jobs[0]["mistoVykonuPrace"]["pracoviste"] *= 2
        try:
            generator.build_model(jobs=bad_jobs)
            raise AssertionError("Ambiguous multiple-workplace count accepted")
        except ValueError as error:
            assert "multiple workplaces" in str(error)
        ok("Duplicate source jobs and ambiguous multiple workplaces are rejected")
        assert generator.build_model(jobs=list(reversed(jobs)))["employers"] == model["employers"]
        assert generator.build_model(jobs=list(reversed(jobs)))["demand"] == model["demand"]
        ok("Workplace IDs and aggregation are independent of source row order")

        run(["docker", "run", "--detach", "--rm", "--pull=never", "--platform=linux/amd64",
             "--name", name, "--network=none", "--memory=768m",
             "--mount", "type=tmpfs,destination=/var/lib/postgresql",
             "-e", "POSTGRES_HOST_AUTH_METHOD=trust", "-e", "POSTGRES_USER=verify",
             "-e", "POSTGRES_DB=verify", "postgis/postgis:18-3.6"])
        created = True
        for attempt in range(60):
            # The bootstrap server accepts Unix sockets before the application
            # DB/extensions exist; TCP is enabled only on the final server.
            ready = run(["docker", "exec", name, "psql", "-X", "-qAt", "-h", "127.0.0.1",
                         "-U", "verify", "-d", "verify", "-c", "SELECT 1"], expect_success=False)
            if ready.returncode == 0:
                break
            time.sleep(0.5)
        else:
            raise RuntimeError("Temporary PostgreSQL did not start")
        container = json.loads(run(["docker", "inspect", name]).stdout)[0]
        assert container["HostConfig"]["NetworkMode"] == "none"
        assert not container["HostConfig"]["PortBindings"]
        assert all(m["Type"] == "tmpfs" for m in container["Mounts"]), container["Mounts"]
        report["postgres_version"] = scalar("SHOW server_version;")
        ok("Isolated PostgreSQL: no network, published ports or persistent/host volumes")

        for relative in ["scripts/sql/insert_skoly.sql", "scripts/insert_obory_nabidka_oboru.sql"]:
            sql((ROOT / relative).read_text(encoding="utf-8"))
        prior = scalar('''SELECT md5(string_agg(row_to_json(x)::text, '' ORDER BY row_to_json(x)::text))
FROM (SELECT * FROM "OBORY") x;''')
        for path in files:
            sql(path.read_text(encoding="utf-8"))
        ok("All four standalone SQL files execute on PostgreSQL with existing school schema")

        # Independently recompute expected demand using raw JSON inside PostgreSQL.
        # This uses neither generator aggregation nor its hashed identifiers.
        raw = json.dumps({"polozky": jobs}, ensure_ascii=False)
        nsp = generator.load(generator.NSP)
        raw_nsp = json.dumps(nsp, ensure_ascii=False)
        reference = '''
CREATE TEMP TABLE source_jobs AS
SELECT
    j->'zamestnavatel'->>'ico' AS ico,
    j->'zamestnavatel'->>'nazev' AS nazev,
    j->'mistoVykonuPrace'->'pracoviste'->0->'adresa'->>'kodAdresnihoMista' AS adresa,
    split_part(j->'mistoVykonuPrace'->'pracoviste'->0->'adresa'->'obec'->>'id', '/', 2) AS obec,
    left(split_part(j->'profeseCzIsco'->>'id', '/', 2), 3) AS skupina,
    split_part(j->'minPozadovaneVzdelani'->>'id', '/', 2) AS vzdelani,
    (j->>'pocetMist')::integer AS mista
FROM jsonb_array_elements(''' + quote(raw) + '''::jsonb->'polozky') j;
CREATE TEMP TABLE expected_demand AS
SELECT z.id AS zamestnavatel_id, s.skupina AS cz_isco3, s.vzdelani AS min_vzdelani,
       sum(s.mista)::integer AS pocet_mist
FROM source_jobs s
JOIN "ZAMESTNAVATELE" z ON z.ico = s.ico AND z.kod_adresniho_mista IS NOT DISTINCT FROM s.adresa
                          AND z.kod_obce = s.obec
GROUP BY z.id, s.skupina, s.vzdelani;
CREATE TEMP TABLE expected_mapping AS
SELECT left(i->'isco'->>'code', 3) AS cz_isco3, r->'rvp'->>'code' AS kod_oboru,
       min((r->>'kkovSuitabilityLevel')::smallint) AS vhodnost
FROM jsonb_array_elements(''' + quote(raw_nsp) + '''::jsonb->'work_units') u
CROSS JOIN LATERAL jsonb_array_elements(u->'isco'->'response'->'data') i
CROSS JOIN LATERAL jsonb_array_elements(u->'education'->'response'->'data'->'rvps') r
JOIN "OBORY" o ON o.kod = r->'rvp'->>'code'
JOIN "PROFESNI_SKUPINY" g ON g.cz_isco3 = left(i->'isco'->>'code', 3)
GROUP BY 1, 2;
DO $$ BEGIN
    IF EXISTS (
        (SELECT * FROM expected_demand EXCEPT ALL
         SELECT zamestnavatel_id, cz_isco3, min_vzdelani, pocet_mist FROM "POPTAVKA_PROFESI")
        UNION ALL
        (SELECT zamestnavatel_id, cz_isco3, min_vzdelani, pocet_mist FROM "POPTAVKA_PROFESI"
         EXCEPT ALL SELECT * FROM expected_demand)
    ) THEN RAISE EXCEPTION 'Demand differs from raw JSON'; END IF;
    IF EXISTS (
        (SELECT * FROM expected_mapping EXCEPT ALL SELECT * FROM "OBOR_PROFESE")
        UNION ALL (SELECT * FROM "OBOR_PROFESE" EXCEPT ALL SELECT * FROM expected_mapping)
    ) THEN RAISE EXCEPTION 'Mapping differs from NSP evidence'; END IF;
    IF (SELECT sum(pocet_mist) FROM "POPTAVKA_PROFESI") <> (SELECT sum(mista) FROM source_jobs)
    THEN RAISE EXCEPTION 'Lost or duplicated source positions'; END IF;
    IF EXISTS (SELECT ico, nazev, adresa, obec FROM source_jobs EXCEPT
               SELECT ico, nazev, kod_adresniho_mista, kod_obce FROM "ZAMESTNAVATELE")
    THEN RAISE EXCEPTION 'Employer/location differs from source'; END IF;
END $$;
'''
        sql(reference)
        ok("Every demand row and every mapping pair match independent SQL recomputation from raw JSON")

        dimensions = json.loads(scalar('''SELECT json_build_object(
  'workplaces', (SELECT count(*) FROM "ZAMESTNAVATELE"),
  'employers', (SELECT count(DISTINCT ico) FROM "ZAMESTNAVATELE"),
  'groups', (SELECT count(*) FROM "PROFESNI_SKUPINY"),
  'demand_rows', (SELECT count(*) FROM "POPTAVKA_PROFESI"),
  'positions', (SELECT sum(pocet_mist) FROM "POPTAVKA_PROFESI"),
  'mapping_pairs', (SELECT count(*) FROM "OBOR_PROFESE"),
  'mapped_groups', (SELECT count(DISTINCT cz_isco3) FROM "OBOR_PROFESE"),
  'mapped_programs', (SELECT count(DISTINCT kod_oboru) FROM "OBOR_PROFESE"),
  'import_timestamps', (SELECT count(DISTINCT importovano_at) FROM "POPTAVKA_PROFESI")
);'''))
        assert dimensions["positions"] == sum(j["pocetMist"] for j in jobs) == 1889
        assert dimensions["employers"] == len({j["zamestnavatel"]["ico"] for j in jobs}) == 390
        assert dimensions["import_timestamps"] == 1
        report["database_counts"] = dimensions
        ok("All 744 jobs / 1889 positions preserved; 390 distinct employers; consistent import timestamp")

        schema = json.loads(scalar('''SELECT json_object_agg(table_name, names) FROM (
 SELECT table_name, json_agg(column_name ORDER BY ordinal_position) names
 FROM information_schema.columns WHERE table_schema = 'public' AND table_name IN
 ('ZAMESTNAVATELE','PROFESNI_SKUPINY','POPTAVKA_PROFESI','OBOR_PROFESE') GROUP BY table_name) s;'''))
        assert schema == {
            "ZAMESTNAVATELE": ["id", "ico", "nazev", "kod_adresniho_mista", "kod_obce", "lat", "lon"],
            "PROFESNI_SKUPINY": ["cz_isco3", "nazev"],
            "POPTAVKA_PROFESI": ["zamestnavatel_id", "cz_isco3", "min_vzdelani", "pocet_mist", "importovano_at"],
            "OBOR_PROFESE": ["cz_isco3", "kod_oboru", "vhodnost"],
        }
        ok("Exactly the agreed four-table column layout")

        # Validate coordinates against independent RUIAN response and local boundary.
        coordinates = {str(f["attributes"]["kod"]): f["geometry"]
                       for page in generator.load(generator.RUIAN)["pages"]
                       for f in page["response"]["features"] if f.get("geometry")}
        db_points = json.loads(scalar('''SELECT json_agg(json_build_object('kod', kod_adresniho_mista,
            'lat', lat, 'lon', lon)) FROM "ZAMESTNAVATELE";'''))
        for row in db_points:
            point = coordinates.get(row["kod"])
            if point:
                assert abs(point["y"] - row["lat"]) < 0.0000001
                assert abs(point["x"] - row["lon"]) < 0.0000001
            else:
                assert row["lat"] is None and row["lon"] is None
        region = generator.load(ROOT / "location_data/karlovarsky_kraj.geojson")
        if region["type"] == "FeatureCollection":
            geometries = [f["geometry"] for f in region["features"]]
        elif region["type"] == "Feature":
            geometries = [region["geometry"]]
        else:
            geometries = [region]
        union = "ST_UnaryUnion(ST_Collect(ARRAY[" + ",".join(
            "ST_SetSRID(ST_GeomFromGeoJSON(" + quote(json.dumps(g)) + "),4326)" for g in geometries) + "]))"
        outside = scalar(f'''SELECT count(*) FROM "ZAMESTNAVATELE" WHERE lat IS NOT NULL
AND NOT ST_Covers({union}, ST_SetSRID(ST_MakePoint(lon,lat),4326));''')
        assert outside == "0", f"Coordinates outside region: {outside}"
        report["geocoded_workplaces"] = sum(r["lat"] is not None for r in db_points)
        report["workplaces_without_coordinates"] = sum(r["lat"] is None for r in db_points)
        report["positions_without_coordinates"] = int(scalar('''SELECT sum(p.pocet_mist)
FROM "POPTAVKA_PROFESI" p JOIN "ZAMESTNAVATELE" z ON z.id=p.zamestnavatel_id WHERE z.lat IS NULL;'''))
        assert report["positions_without_coordinates"] == model["report"]["positions_without_coordinates"]
        ok("Coordinates match RUIAN, are inside Karlovarsky region, and missing locations remain NULL")

        sample = scalar('''SELECT sum(p.pocet_mist), count(DISTINCT z.ico)
FROM "POPTAVKA_PROFESI" p JOIN "ZAMESTNAVATELE" z ON z.id=p.zamestnavatel_id
WHERE p.cz_isco3='512';''')
        assert sample == "170|67", sample
        wit = scalar('''SELECT sum(p.pocet_mist), count(DISTINCT z.id), count(DISTINCT z.ico)
FROM "POPTAVKA_PROFESI" p JOIN "ZAMESTNAVATELE" z ON z.id=p.zamestnavatel_id
WHERE z.ico='40525881';''')
        assert wit == "34|2|1", wit
        assert scalar('''SELECT count(*) FROM "ZAMESTNAVATELE" WHERE ico='02183765';''') == "1"
        assert int(scalar('''SELECT count(DISTINCT s.redizo) FROM "OBOR_PROFESE" op
JOIN "NABIDKA_OBORU" n ON n.kod_oboru=op.kod_oboru
JOIN stredni_skoly s ON s.redizo=n.redizo WHERE op.cz_isco3='512';''')) == 3
        assert scalar('''SELECT sum(p.pocet_mist) FROM "POPTAVKA_PROFESI" p
WHERE p.cz_isco3='512' AND EXISTS (
 SELECT 1 FROM "OBOR_PROFESE" op JOIN "NABIDKA_OBORU" n ON n.kod_oboru=op.kod_oboru
 WHERE op.cz_isco3=p.cz_isco3
);''') == "170"
        ok("EXISTS search across multiple matching programs/schools counts each position only once")
        ok("Search examples: group 512 = 170 jobs / 67 firms / 3 schools; WITTE = 2 workplaces / 1 firm; leading-zero ICO retained")

        initial = fingerprint()
        for path in files:
            sql(path.read_text(encoding="utf-8"))
        assert initial == fingerprint()
        ok("Repeated imports do not duplicate or alter business data")

        sql('''INSERT INTO "ZAMESTNAVATELE" VALUES (1,'00000001','TEST stale',NULL,'554961',NULL,NULL);
INSERT INTO "POPTAVKA_PROFESI" VALUES (1,'512','test',99,now());
INSERT INTO "OBOR_PROFESE" SELECT g.cz_isco3,o.kod,2
FROM "PROFESNI_SKUPINY" g CROSS JOIN "OBORY" o
WHERE NOT EXISTS(SELECT 1 FROM "OBOR_PROFESE" m WHERE m.cz_isco3=g.cz_isco3 AND m.kod_oboru=o.kod)
ORDER BY 1,2 LIMIT 1;''')
        for path in files:
            sql(path.read_text(encoding="utf-8"))
        assert initial == fingerprint()
        ok("Full snapshot refresh removes stale demand, workplaces and mapping pairs")

        demand_sql = files[2].read_text(encoding="utf-8")
        first_id = model["demand"][0][0]
        broken = demand_sql.replace(f"    ({first_id},", "    (1,", 1)
        assert broken != demand_sql
        expect_error(broken, "foreign key constraint")
        assert initial == fingerprint()
        map_sql = files[3].read_text(encoding="utf-8")
        broken_map = map_sql.replace(quote(model["mapping"][0][1]), "'INVALID-RVP'", 1)
        expect_error(broken_map, "foreign key constraint")
        assert initial == fingerprint()
        ok("Foreign-key failure rolls back entire demand/mapping replacement without data loss")

        expect_error('''BEGIN; UPDATE "POPTAVKA_PROFESI" SET pocet_mist=-1; COMMIT;''', "check constraint")
        expect_error('''BEGIN; UPDATE "OBOR_PROFESE" SET vhodnost=3; COMMIT;''', "check constraint")
        expect_error('''BEGIN; INSERT INTO "POPTAVKA_PROFESI" SELECT * FROM "POPTAVKA_PROFESI" LIMIT 1; COMMIT;''', "unique constraint")
        expect_error('''BEGIN; UPDATE "ZAMESTNAVATELE" SET lon=NULL WHERE lat IS NOT NULL; COMMIT;''', "check constraint")
        assert initial == fingerprint()
        assert prior == scalar('''SELECT md5(string_agg(row_to_json(x)::text, '' ORDER BY row_to_json(x)::text))
FROM (SELECT * FROM "OBORY") x;''')
        ok("PK/check constraints reject invalid data; existing OBORY remains unchanged")
        report["business_data_fingerprints"] = initial
        report["status"] = "passed"
    except Exception as error:
        report["status"] = "failed"
        report["error"] = str(error)
        raise
    finally:
        if created:
            run(["docker", "rm", "--force", name])
            report["temporary_container_removed"] = True
        report["finished_at"] = datetime.now(timezone.utc).isoformat()
        REPORT.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"Verified {len(checks)} checks. Report: {REPORT}")


if __name__ == "__main__":
    verify()
