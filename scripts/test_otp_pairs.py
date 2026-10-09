#!/usr/bin/env python3
"""Sample points in Karlovarský kraj and test every directed pair against OTP.

Python 3.9+; standard library only. Run from any directory:
    python3 scripts/test_otp_pairs.py --points 20 --workers 6

Default arrival window: 2026-10-12 07:30:00 <= arrival < 08:00:00,
Europe/Prague. Choose the shortest duration among OTP's returned candidates;
break ties by latest arrival. This is not an exhaustive global-optimum search.
Default modes: transit with walking, plus walking-only when OTP suggests it.
"""
import argparse
import csv
import json
import math
import random
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timedelta
from itertools import permutations
from pathlib import Path
from zoneinfo import ZoneInfo

ROOT = Path(__file__).resolve().parents[1]
PRAGUE = ZoneInfo("Europe/Prague")
BOUNDARY_URL = "https://nominatim.openstreetmap.org/search?" + urllib.parse.urlencode({
    "q": "Karlovarský kraj, Czechia", "format": "jsonv2",
    "polygon_geojson": 1, "limit": 1,
})
QUERY = """
query Pair($origin: PlanLabeledLocationInput!,
           $destination: PlanLabeledLocationInput!,
           $dateTime: PlanDateTimeInput!, $first: Int!, $window: Duration!) {
  planConnection(origin: $origin, destination: $destination,
                 dateTime: $dateTime, first: $first, searchWindow: $window) {
    routingErrors { code description inputField }
    edges { node {
      start end duration numberOfTransfers walkDistance
      legs {
        mode transitLeg duration
        start { estimated { time } }
        end { estimated { time } }
        route { shortName longName }
        from { name lat lon }
        to { name lat lon }
      }
    } }
  }
}
"""
CSV_FIELDS = [
    "origin", "destination", "origin_lat", "origin_lon",
    "destination_lat", "destination_lon", "status", "departure", "arrival",
    "duration_minutes", "minutes_before_deadline", "transfers", "walk_meters",
    "modes", "routes", "candidates_returned", "candidates_in_arrival_window",
    "api_seconds", "errors",
]


def fetch_json(url, payload=None, timeout=60):
    data = None if payload is None else json.dumps(payload).encode("utf-8")
    headers = {"User-Agent": "Karlovarsky-OTP-Test/1.0"}
    if data is not None:
        headers["Content-Type"] = "application/json"
    request = urllib.request.Request(url, data=data, headers=headers)
    with urllib.request.urlopen(request, timeout=timeout) as response:
        return json.load(response)


def load_boundary(path):
    if not path.exists():
        results = fetch_json(BOUNDARY_URL)
        match = next((item for item in results
                      if item.get("osm_type") == "relation"
                      and item.get("osm_id") == 442314), None)
        if not match or match["geojson"]["type"] not in ("Polygon", "MultiPolygon"):
            raise ValueError("Could not retrieve Karlovarský kraj's administrative boundary.")
        feature = {
            "type": "Feature",
            "properties": {
                "name": match["display_name"],
                "osm_relation": 442314,
                "source": BOUNDARY_URL,
                "attribution": "© OpenStreetMap contributors; ODbL 1.0",
                "downloaded_at": datetime.now(PRAGUE).isoformat(),
            },
            "geometry": match["geojson"],
        }
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(feature, ensure_ascii=False), encoding="utf-8")
    document = json.loads(path.read_text(encoding="utf-8"))
    geometry = document.get("geometry", document)
    if geometry["type"] == "Polygon":
        return [geometry["coordinates"]]
    if geometry["type"] == "MultiPolygon":
        return geometry["coordinates"]
    raise ValueError("Boundary must be a GeoJSON Polygon or MultiPolygon.")


def in_ring(lon, lat, ring):
    inside = False
    previous = ring[-1]
    for current in ring:
        x1, y1 = previous[:2]
        x2, y2 = current[:2]
        if (y1 > lat) != (y2 > lat):
            if lon < (x2 - x1) * (lat - y1) / (y2 - y1) + x1:
                inside = not inside
        previous = current
    return inside


def in_region(lon, lat, polygons):
    return any(in_ring(lon, lat, polygon[0])
               and not any(in_ring(lon, lat, hole) for hole in polygon[1:])
               for polygon in polygons)


def generate_points(polygons, count, seed):
    coordinates = [point for polygon in polygons for point in polygon[0]]
    min_lon = min(p[0] for p in coordinates)
    max_lon = max(p[0] for p in coordinates)
    min_lat = min(p[1] for p in coordinates)
    max_lat = max(p[1] for p in coordinates)
    rng = random.Random(seed)
    points = []
    seen = set()
    # Uniform longitude and sin(latitude) approximate uniform surface area.
    for _ in range(max(10000, count * 1000)):
        lon = round(rng.uniform(min_lon, max_lon), 6)
        lat = round(math.degrees(math.asin(rng.uniform(
            math.sin(math.radians(min_lat)),
            math.sin(math.radians(max_lat))))), 6)
        if (lon, lat) not in seen and in_region(lon, lat, polygons):
            points.append({"id": f"P{len(points) + 1:03d}", "lat": lat, "lon": lon})
            seen.add((lon, lat))
            if len(points) == count:
                return points
    raise ValueError("Could not generate enough distinct points inside the boundary.")


def parse_time(value):
    result = datetime.fromisoformat(value.replace("Z", "+00:00"))
    if result.tzinfo is None:
        raise ValueError("OTP returned a timestamp without a time zone.")
    return result.astimezone(PRAGUE)


def select_best(candidates, arrival, early_minutes):
    earliest = arrival - timedelta(minutes=early_minutes)
    eligible = [item for item in candidates
                if earliest <= parse_time(item["end"]) < arrival
                and parse_time(item["start"]).date() == arrival.date()]
    best = min(eligible, key=lambda item: (
        float(item["duration"]), -parse_time(item["end"]).timestamp()
    ), default=None)
    return best, len(eligible)


def location(point):
    return {"label": point["id"], "location": {"coordinate": {
        "latitude": point["lat"], "longitude": point["lon"],
    }}}


def route_pair(origin, destination, args, arrival):
    row = {
        "origin": origin["id"], "destination": destination["id"],
        "origin_lat": origin["lat"], "origin_lon": origin["lon"],
        "destination_lat": destination["lat"], "destination_lon": destination["lon"],
    }
    started = time.monotonic()
    payload = {
        "query": QUERY,
        "variables": {
            "origin": location(origin), "destination": location(destination),
            "dateTime": {"latestArrival": (arrival - timedelta(seconds=1)).isoformat()},
            "first": args.candidates, "window": f"PT{args.search_window_minutes}M",
        },
    }
    details = {"origin": origin, "destination": destination}
    try:
        response = fetch_json(args.url, payload, args.timeout)
        details["response"] = response
        if response.get("errors"):
            row.update(status="api_error",
                       errors=json.dumps(response["errors"], ensure_ascii=False))
        else:
            plan = (response.get("data") or {}).get("planConnection")
            if plan is None:
                raise ValueError("API response has no planConnection.")
            candidates = [edge["node"] for edge in plan["edges"] or []
                          if edge and edge.get("node")]
            best, eligible_count = select_best(candidates, arrival, args.max_early_minutes)
            errors = plan.get("routingErrors", [])
            row.update(candidates_returned=len(candidates),
                       candidates_in_arrival_window=eligible_count,
                       errors=json.dumps(errors, ensure_ascii=False))
            if best is None:
                row["status"] = ("no_route" if not candidates
                                 else "no_route_in_arrival_window")
            else:
                details["selected"] = best
                legs = best["legs"]
                route_names = [
                    (leg["route"].get("shortName") or leg["route"].get("longName") or "")
                    for leg in legs if leg.get("route")
                ]
                row.update(
                    status="ok",
                    departure=parse_time(best["start"]).isoformat(),
                    arrival=parse_time(best["end"]).isoformat(),
                    duration_minutes=round(float(best["duration"]) / 60, 2),
                    minutes_before_deadline=round(
                        (arrival - parse_time(best["end"])).total_seconds() / 60, 2),
                    transfers=best["numberOfTransfers"],
                    walk_meters=round(best["walkDistance"], 1),
                    modes=" > ".join(leg["mode"] for leg in legs),
                    routes=" > ".join(route_names),
                )
    except (OSError, ValueError, KeyError, TypeError) as error:
        row.update(status="api_error", errors=str(error))
        details["error"] = str(error)
    row["api_seconds"] = round(time.monotonic() - started, 3)
    details["summary"] = row
    return row, details


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--points", type=int, default=20, help="X points; X*(X-1) requests")
    parser.add_argument("--seed", type=int, default=42)
    parser.add_argument("--workers", type=int, default=1,
                        help="Maximum concurrent route requests (default: 1)")
    parser.add_argument("--arrival", default="2026-10-12T08:00:00",
                        help="Exclusive arrival deadline, interpreted in Europe/Prague")
    parser.add_argument("--max-early-minutes", type=float, default=30,
                        help="How far before the deadline arrival may be (default: 30)")
    parser.add_argument("--candidates", type=int, default=20,
                        help="Maximum itinerary candidates requested per pair")
    parser.add_argument("--search-window-minutes", type=int, default=120)
    parser.add_argument("--timeout", type=float, default=60)
    parser.add_argument("--url", default="http://localhost:8080/otp/gtfs/v1")
    parser.add_argument("--boundary", type=Path,
                        default=ROOT / "location_data/karlovarsky_kraj.geojson")
    parser.add_argument("--output-dir", type=Path)
    args = parser.parse_args()
    if (args.points < 2 or args.candidates < 1 or args.search_window_minutes < 1
            or args.max_early_minutes <= 0 or args.timeout <= 0 or args.workers < 1):
        parser.error("Need at least 2 points and positive worker, candidate, window and timeout values.")
    try:
        arrival = datetime.fromisoformat(args.arrival)
        arrival = (arrival.replace(tzinfo=PRAGUE) if arrival.tzinfo is None
                   else arrival.astimezone(PRAGUE))
        if args.max_early_minutes > args.search_window_minutes:
            parser.error("Arrival tolerance must not exceed the search window.")
        # Preflight avoids executing every pair when OTP is unavailable.
        preflight = fetch_json(args.url, {"query": "{ __typename }"}, args.timeout)
        if preflight.get("errors") or not preflight.get("data"):
            raise ValueError(f"OTP preflight failed: {preflight}")
        polygons = load_boundary(args.boundary)
        points = generate_points(polygons, args.points, args.seed)
    except (OSError, ValueError, KeyError) as error:
        parser.exit(1, f"Error: {error}\n")

    output = args.output_dir or ROOT / "location_data/api_tests" / datetime.now(
        PRAGUE).strftime("%Y%m%d_%H%M%S_%f")
    output.mkdir(parents=True, exist_ok=True)
    features = [{"type": "Feature", "properties": {"id": point["id"]},
                 "geometry": {"type": "Point", "coordinates": [point["lon"], point["lat"]]}}
                for point in points]
    (output / "points.geojson").write_text(json.dumps(
        {"type": "FeatureCollection", "features": features}, ensure_ascii=False,
        indent=2), encoding="utf-8")
    total = args.points * (args.points - 1)
    print(f"Testing {args.points} points, {total} directed pairs with {args.workers} workers; "
          f"arrival {arrival.isoformat()}.",
          flush=True)
    counts = Counter()
    fastest = None
    started = time.monotonic()
    with (output / "routes.csv").open("w", encoding="utf-8-sig", newline="") as csv_file, (
            output / "routes.jsonl").open("w", encoding="utf-8") as json_file, (
            ThreadPoolExecutor(max_workers=args.workers)) as executor:
        writer = csv.DictWriter(csv_file, fieldnames=CSV_FIELDS)
        writer.writeheader()
        # Requests run in worker threads; only this thread writes outputs.
        # map preserves pair order, keeping CSV and JSONL ordering reproducible.
        results = executor.map(
            lambda pair: route_pair(pair[0], pair[1], args, arrival),
            permutations(points, 2),
        )
        for index, (row, details) in enumerate(results, 1):
            writer.writerow(row)
            json_file.write(json.dumps(details, ensure_ascii=False) + "\n")
            csv_file.flush()
            json_file.flush()
            counts[row["status"]] += 1
            if row["status"] == "ok" and (
                    fastest is None or row["duration_minutes"] < fastest["duration_minutes"]):
                fastest = row
            print(f"[{index}/{total}] {row['origin']} -> {row['destination']}: "
                  f"{row['status']}"
                  + (f" ({row['duration_minutes']} min)" if row["status"] == "ok" else ""),
                  flush=True)
    summary = {
        "points": args.points, "directed_pairs": total, "seed": args.seed,
        "workers": args.workers,
        "arrival_deadline_exclusive": arrival.isoformat(),
        "earliest_arrival": (arrival - timedelta(minutes=args.max_early_minutes)).isoformat(),
        "same_day_departures_only": True, "api_url": args.url,
        "boundary": str(args.boundary), "candidates_per_pair": args.candidates,
        "search_window_minutes": args.search_window_minutes,
        "status_counts": dict(counts), "fastest_pair": fastest,
        "elapsed_seconds": round(time.monotonic() - started, 3),
        "selection": "Minimum duration among returned candidates in the arrival window; "
                     "ties prefer later arrival. OTP filtering and candidate limits apply.",
    }
    (output / "summary.json").write_text(json.dumps(summary, ensure_ascii=False, indent=2),
                                       encoding="utf-8")
    print(f"Results: {output}\nStatuses: {dict(counts)}", flush=True)
    if counts["api_error"]:
        sys.exit(1)


if __name__ == "__main__":
    main()
