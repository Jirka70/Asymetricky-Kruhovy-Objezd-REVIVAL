#!/usr/bin/env python3
"""Calculate ZSJ-to-school travel times using OTP (Python 3.9+, stdlib only).

python3 scripts/calculate_zsj_school_times.py --workers 6
python3 scripts/calculate_zsj_school_times.py --slot 07:00-08:00 --slot 09:00-10:00

Origin: representative interior point of each ZSJ polygon in insert_zsj.sql,
or explicit kod_zsj,lat,lon coordinates supplied with --zsj-csv.
Minimum door-to-door duration among returned OTP candidates arriving in the
half-open slot [start,end), with departure on the same date. Transit + walking
and walking-only use OTP defaults. No qualifying route => empty doba_jizdy.
"""
import argparse
import csv
import hashlib
import json
import math
import re
import sys
import time
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timedelta
from itertools import product
from pathlib import Path
from test_otp_pairs import PRAGUE, ROOT, fetch_json, in_region, location, parse_time

FIELDS = ["kod_zsj", "redizo", "slot_prijezdu", "doba_jizdy"]
DIAGNOSTIC_FIELDS = FIELDS + [
    "status", "odjezd", "prijezd", "candidates_returned", "eligible_candidates",
    "api_seconds", "errors",
]
QUERY = """
query Matrix($origin: PlanLabeledLocationInput!, $destination: PlanLabeledLocationInput!,
             $dateTime: PlanDateTimeInput!, $first: Int!, $window: Duration!) {
  planConnection(origin: $origin, destination: $destination,
                 dateTime: $dateTime, first: $first, searchWindow: $window) {
    routingErrors { code description inputField }
    edges { node { start end duration } }
  }
}
"""
SQL_ROW = re.compile(r"\s*\('((?:''|[^'])*)', '((?:''|[^'])*)', '((?:''|[^'])*)'\),?\s*")


def positive_int(value):
    number = int(value)
    if number < 1:
        raise argparse.ArgumentTypeError("Must be a positive integer.")
    return number


def slot_bounds(value, day):
    try:
        start, end = value.split("-")
        start = datetime.strptime(f"{day} {start}", "%Y-%m-%d %H:%M").replace(tzinfo=PRAGUE)
        end = datetime.strptime(f"{day} {end}", "%Y-%m-%d %H:%M").replace(tzinfo=PRAGUE)
    except ValueError as error:
        raise ValueError(f"Invalid slot {value!r}; use HH:MM-HH:MM.") from error
    if start >= end:
        raise ValueError("Slots must end after they start, on the same day.")
    return start, end


def interior_point(geometry):
    """Midpoint of the widest interior scanline interval; accounts for holes."""
    if geometry["type"] == "Polygon":
        polygons = [geometry["coordinates"]]
    elif geometry["type"] == "MultiPolygon":
        polygons = geometry["coordinates"]
    else:
        raise ValueError("ZSJ geometry must be Polygon or MultiPolygon.")
    choices = []
    for polygon in polygons:
        ys = sorted({float(point[1]) for ring in polygon for point in ring})
        middle = (ys[0] + ys[-1]) / 2
        below = max(y for y in ys if y <= middle)
        above = min(y for y in ys if y > middle)
        y = (below + above) / 2
        intersections = []
        for ring in polygon:
            for p1, p2 in zip(ring, ring[1:] + ring[:1]):
                x1, y1 = p1[:2]
                x2, y2 = p2[:2]
                if (y1 > y) != (y2 > y):
                    intersections.append(x1 + (y - y1) * (x2 - x1) / (y2 - y1))
        intersections.sort()
        for left, right in zip(intersections[::2], intersections[1::2]):
            lon = (left + right) / 2
            if right > left and in_region(lon, y, [polygon]):
                choices.append((right - left, y, lon))
    if not choices:
        raise ValueError("Cannot find a point inside a ZSJ polygon.")
    _, lat, lon = max(choices)
    return lat, lon


def validate_points(points, id_field):
    seen = set()
    for point in points:
        code = point[id_field]
        if not code or code in seen:
            raise ValueError(f"Missing or duplicate {id_field}: {code!r}")
        seen.add(code)
        if not (math.isfinite(point["lat"]) and math.isfinite(point["lon"])
                and -90 <= point["lat"] <= 90 and -180 <= point["lon"] <= 180):
            raise ValueError(f"Invalid coordinates for {id_field}={code}.")
    if not points:
        raise ValueError(f"No {id_field} records found.")


def read_zsj(args):
    if args.zsj_csv:
        with args.zsj_csv.open(encoding="utf-8-sig", newline="") as source:
            points = [
                {"kod_zsj": row["kod_zsj"], "lat": float(row["lat"]), "lon": float(row["lon"])}
                for row in csv.DictReader(source)
            ]
    else:
        points = []
        with args.zsj_sql.open(encoding="utf-8") as source:
            for line in source:
                match = SQL_ROW.fullmatch(line)
                if match:
                    code, name, geometry = (value.replace("''", "'") for value in match.groups())
                    lat, lon = interior_point(json.loads(geometry))
                    points.append({"kod_zsj": code, "nazev": name, "lat": lat, "lon": lon})
        # Fail on incomplete extraction rather than silently skipping ZSJ.
        expected = re.search(r"Počet záznamů:\s*(\d+)", args.zsj_sql.read_text(encoding="utf-8"))
        if expected and int(expected.group(1)) != len(points):
            raise ValueError("ZSJ extraction count does not match the SQL source header.")
    validate_points(points, "kod_zsj")
    return points


def read_schools(path):
    with path.open(encoding="utf-8-sig", newline="") as source:
        schools = [{"redizo": row["redizo"], "lat": float(row["lat"]), "lon": float(row["lon"])}
                   for row in csv.DictReader(source)]
    validate_points(schools, "redizo")
    return schools


def key(row):
    return tuple(row[field] for field in FIELDS[:3])


def fingerprint(path):
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def calculate(task, args):
    origin, school, slot, start, end = task
    row = {"kod_zsj": origin["kod_zsj"], "redizo": school["redizo"],
           "slot_prijezdu": slot, "doba_jizdy": ""}
    begin = time.monotonic()
    payload = {
        "query": QUERY,
        "variables": {
            "origin": location({"id": origin["kod_zsj"], "lat": origin["lat"], "lon": origin["lon"]}),
            "destination": location({"id": school["redizo"], "lat": school["lat"], "lon": school["lon"]}),
            "dateTime": {"latestArrival": (end - timedelta(seconds=1)).isoformat()},
            "first": args.candidates,
            "window": f"PT{int((end - start).total_seconds())}S",
        },
    }
    try:
        response = fetch_json(args.url, payload, args.timeout)
        if response.get("errors"):
            raise ValueError(json.dumps(response["errors"], ensure_ascii=False))
        plan = (response.get("data") or {}).get("planConnection")
        if plan is None:
            raise ValueError("OTP returned no planConnection.")
        candidates = [edge["node"] for edge in plan["edges"] or [] if edge and edge.get("node")]
        eligible = [item for item in candidates if start <= parse_time(item["end"]) < end
                    and parse_time(item["start"]).date() == start.date()]
        row.update(candidates_returned=len(candidates), eligible_candidates=len(eligible),
                   errors=json.dumps(plan.get("routingErrors", []), ensure_ascii=False))
        if not eligible:
            row["status"] = "no_route" if not candidates else "no_route_in_slot"
        else:
            best = min(eligible, key=lambda item: (
                float(item["duration"]), -parse_time(item["end"]).timestamp()))
            row.update(status="ok", doba_jizdy=f"{float(best['duration']) / 60:.2f}",
                       odjezd=parse_time(best["start"]).isoformat(),
                       prijezd=parse_time(best["end"]).isoformat())
    except (OSError, ValueError, KeyError, TypeError) as error:
        row.update(status="api_error", errors=str(error))
    row["api_seconds"] = round(time.monotonic() - begin, 3)
    return row


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--date", default="2026-10-12")
    parser.add_argument("--slot", action="append", help="Repeatable HH:MM-HH:MM; default 07:00-08:00")
    parser.add_argument("--workers", type=positive_int, default=6)
    parser.add_argument("--candidates", type=positive_int, default=50)
    parser.add_argument("--timeout", type=positive_int, default=60)
    parser.add_argument("--url", default="http://localhost:8080/otp/gtfs/v1")
    parser.add_argument("--zsj-sql", type=Path, default=ROOT / "scripts/sql/insert_zsj.sql")
    parser.add_argument("--zsj-csv", type=Path, help="Alternative origins: kod_zsj,lat,lon")
    parser.add_argument("--schools", type=Path, default=ROOT / "datasety/skoly.csv")
    parser.add_argument("--output", type=Path,
                        help="Default: datasety/zsj_skoly_<date>.csv")
    parser.add_argument("--resume", action="store_true", help="Continue a matching interrupted run")
    parser.add_argument("--limit-zsj", type=positive_int, help="Optional smoke-test subset")
    args = parser.parse_args()
    try:
        day = datetime.strptime(args.date, "%Y-%m-%d").date()
        slots = []
        for value in args.slot or ["07:00-08:00"]:
            start, end = slot_bounds(value, args.date)
            label = f"{start:%H:%M}-{end:%H:%M}"
            if any(s[0] == label for s in slots):
                raise ValueError(f"Duplicate slot: {label}")
            slots.append((label, start, end))
        zsj = read_zsj(args)
        if args.limit_zsj:
            zsj = zsj[:args.limit_zsj]
        schools = read_schools(args.schools)
        preflight = fetch_json(args.url, {"query": "{ __typename }"}, args.timeout)
        if preflight.get("errors") or not preflight.get("data"):
            raise ValueError(f"OTP preflight failed: {preflight}")
    except (OSError, ValueError, KeyError, TypeError) as error:
        parser.exit(1, f"Error: {error}\n")
    output = args.output or ROOT / f"datasety/zsj_skoly_{args.date}.csv"
    diagnostics = output.with_name(output.stem + "_diagnostics.csv")
    metadata = output.with_name(output.stem + "_summary.json")
    points_path = output.with_name(output.stem + "_origins.csv")
    config = {
        "date": args.date, "timezone": "Europe/Prague",
        "slots": [slot[0] for slot in slots], "zsj_count": len(zsj),
        "school_count": len(schools), "candidates": args.candidates, "api_url": args.url,
        "zsj_source": str(args.zsj_csv or args.zsj_sql),
        "zsj_sha256": fingerprint(args.zsj_csv or args.zsj_sql),
        "schools_source": str(args.schools), "schools_sha256": fingerprint(args.schools),
        "origin_method": "provided coordinates" if args.zsj_csv else "interior scanline midpoint",
        "departure_rule": "same calendar date",
        "arrival_rule": "slot start inclusive, slot end exclusive",
        "duration_rule": "minimum returned candidate duration; includes walking and transfer waits",
        "modes": "OTP defaults: transit + walking, walking-only allowed",
    }
    total = len(zsj) * len(schools) * len(slots)
    counts = Counter()
    completed = set()
    previous_elapsed = 0
    try:
        if args.resume:
            old = json.loads(metadata.read_text(encoding="utf-8"))
            if old["config"] != config:
                raise ValueError("Resume settings/inputs differ from the saved run.")
            with output.open(encoding="utf-8-sig", newline="") as source:
                reader = csv.DictReader(source)
                if reader.fieldnames != FIELDS:
                    raise ValueError("Existing CSV has an unexpected header.")
                matrix_rows = list(reader)
            with diagnostics.open(encoding="utf-8-sig", newline="") as source:
                diagnostic_rows = list(csv.DictReader(source))
            if len(matrix_rows) != len(diagnostic_rows):
                raise ValueError("Output/diagnostic checkpoint lengths differ.")
            for matrix_row, diagnostic_row in zip(matrix_rows, diagnostic_rows):
                if any(matrix_row[f] != diagnostic_row[f] for f in FIELDS):
                    raise ValueError("Output and diagnostic checkpoints differ.")
                if key(matrix_row) in completed:
                    raise ValueError("Duplicate checkpoint record.")
                completed.add(key(matrix_row))
                counts[diagnostic_row["status"]] += 1
            previous_elapsed = old.get("elapsed_seconds", 0)
        elif output.exists() or diagnostics.exists() or metadata.exists():
            raise ValueError("Output already exists; use --resume or choose another --output.")
    except (OSError, ValueError, KeyError) as error:
        parser.exit(1, f"Error: {error}\n")
    output.parent.mkdir(parents=True, exist_ok=True)
    if not args.resume:
        with points_path.open("w", encoding="utf-8-sig", newline="") as target:
            writer = csv.DictWriter(target, fieldnames=["kod_zsj", "lat", "lon"])
            writer.writeheader()
            writer.writerows({f: point[f] for f in writer.fieldnames} for point in zsj)
    start_clock = time.monotonic()
    workers = min(args.workers, max(1, total - len(completed)))

    def save_summary(finished=False):
        summary = {"config": config, "workers": workers, "expected_rows": total,
                   "completed_rows": len(completed), "status_counts": dict(counts),
                   "elapsed_seconds": round(previous_elapsed + time.monotonic() - start_clock, 3),
                   "complete": finished,
                   "limitation": "OTP filters and limits candidates; this is not an exhaustive global optimum."}
        temporary = metadata.with_suffix(".json.tmp")
        temporary.write_text(json.dumps(summary, ensure_ascii=False, indent=2), encoding="utf-8")
        temporary.replace(metadata)

    save_summary()
    print(f"{len(zsj)} ZSJ × {len(schools)} schools × {len(slots)} slots = {total} rows; "
          f"{day:%A %Y-%m-%d}; {workers} workers. Already completed: {len(completed)}.",
          flush=True)
    tasks = (
        (origin, school, label, start, end)
        for origin, school, (label, start, end) in product(zsj, schools, slots)
        if (origin["kod_zsj"], school["redizo"], label) not in completed
    )
    mode = "a" if args.resume else "w"
    with output.open(mode, encoding="utf-8-sig", newline="") as target, (
            diagnostics.open(mode, encoding="utf-8-sig", newline="")) as diagnostic_file, (
            ThreadPoolExecutor(max_workers=workers)) as executor:
        writer = csv.DictWriter(target, fieldnames=FIELDS, extrasaction="ignore")
        diagnostic_writer = csv.DictWriter(diagnostic_file, fieldnames=DIAGNOSTIC_FIELDS)
        if not args.resume:
            writer.writeheader()
            diagnostic_writer.writeheader()
        # Python 3.9-compatible bounded queue: at most workers futures queued.
        pending = []
        for _ in range(workers):
            task = next(tasks, None)
            if task is not None:
                pending.append(executor.submit(calculate, task, args))
        while pending:
            future = pending.pop(0)
            row = future.result()
            writer.writerow(row)
            diagnostic_writer.writerow(row)
            target.flush()
            diagnostic_file.flush()
            completed.add(key(row))
            counts[row["status"]] += 1
            task = next(tasks, None)
            if task is not None:
                pending.append(executor.submit(calculate, task, args))
            if len(completed) % 100 == 0 or not pending:
                save_summary()
                print(f"[{len(completed)}/{total}] {dict(counts)}; "
                      f"{time.monotonic() - start_clock:.1f}s", flush=True)
    save_summary(finished=len(completed) == total)
    print(f"Saved: {output}\nDiagnostics: {diagnostics}", flush=True)
    if counts["api_error"]:
        sys.exit(1)


if __name__ == "__main__":
    main()
