#!/usr/bin/env python3
"""Download small, auditable NSP/RUIAN snapshots for the labour-market SQL import.

No database access. Uses Python 3.9+ standard library. HTTP responses are cached
under ignored backups/; --refresh discards that cache for this download.
"""

import argparse
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import time
from urllib.parse import urlencode
from urllib.request import Request, urlopen

ROOT = Path(__file__).resolve().parents[1]
JOBS = ROOT / "datasety/volna-mista_karlovarsky_kraj.json"
NSP = "https://nsp.cz/api/v1.2"
RUIAN = "https://ags.cuzk.gov.cz/arcgis/rest/services/RUIAN/MapServer/1/query"
CACHE = ROOT / "backups/pracovni_trh_mvp_http"


def save(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + ".tmp")
    tmp.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    tmp.replace(path)


def request(url, refresh=False):
    path = CACHE / (hashlib.sha256(url.encode()).hexdigest() + ".json")
    if path.exists() and not refresh:
        cached = json.loads(path.read_text(encoding="utf-8"))
        if cached["url"] != url:
            raise ValueError("Cache URL mismatch")
        return cached
    for attempt in range(4):
        try:
            req = Request(url, headers={"Accept": "application/json",
                                        "User-Agent": "OborNaDosah-MVP-data-import/1.0"})
            with urlopen(req, timeout=45) as response:
                data = json.load(response)
            if data.get("error") or (url.startswith(NSP) and data.get("code") != 200):
                raise ValueError(f"API error: {url}: {data}")
            result = {"url": url, "retrieved_at": datetime.now(timezone.utc).isoformat(),
                      "response": data}
            save(path, result)
            return result
        except (OSError, ValueError):
            if attempt == 3:
                raise
            time.sleep(2 ** attempt)


def fetch_nsp(jobs, refresh):
    groups = sorted({j["profeseCzIsco"]["id"].split("/")[-1][:3] for j in jobs})
    index = request(NSP + "/lists/iSCO", refresh)
    codes = index["response"]["data"]
    if len(codes) != index["response"]["count"]:
        raise ValueError("Incomplete ISCO dictionary")
    # NSP has duplicate *codes* under different internal IDs (e.g. 53213).
    # Keep every ID when querying descendants; only group labels must be unique.
    by_code = {r["code"]: r for r in codes if r["code"] in groups}
    if any(sum(r["code"] == g for r in codes) != 1 for g in groups):
        raise ValueError("Duplicate or missing three-digit ISCO groups")

    def group_work_units(group):
        ids = sorted(r["id"] for r in codes if r["code"].startswith(group))
        units, pages, offset, total = {}, [], 0, None
        while True:
            query = [("isco[]", i) for i in ids] + [("limit", 100), ("offset", offset)]
            page = request(NSP + "/workUnit?" + urlencode(query), refresh)
            pages.append({k: page[k] for k in ("url", "retrieved_at")})
            response = page["response"]
            if total is not None and total != response["count"]:
                raise ValueError(f"NSP changed during pagination: {group}")
            total = response["count"]
            for unit in response["data"]:
                slug = unit["urlSlug"]
                if slug in units:
                    raise ValueError(f"Duplicate paginated work unit: {group}/{slug}")
                units[slug] = {"slug": slug, "title": unit["title"]}
            offset += len(response["data"])
            if offset >= total:
                break
            if not response["data"]:
                raise ValueError(f"Incomplete work-unit listing: {group}")
        if len(units) != total:
            raise ValueError(f"Work-unit count mismatch: {group}")
        return {"code": group, "title": by_code[group]["title"],
                "queried_isco_ids": ids, "pages": pages,
                "work_units": [units[k] for k in sorted(units)]}

    result_groups = []
    with ThreadPoolExecutor(max_workers=3) as pool:
        for i, result in enumerate(pool.map(group_work_units, groups), 1):
            result_groups.append(result)
            if i % 10 == 0 or i == len(groups):
                print(f"NSP groups: {i}/{len(groups)}", flush=True)
    slugs = sorted({u["slug"] for g in result_groups for u in g["work_units"]})

    def detail(slug):
        isco = request(NSP + f"/workUnit/{slug}/isco", refresh)
        education = request(NSP + f"/workUnit/{slug}/education", refresh)
        # Keep only evidence needed for this import; full HTTP bodies stay in
        # the ignored cache. No competencies, wages or personal contacts.
        education = {**education, "response": {**education["response"], "data": {
            k: education["response"]["data"][k] for k in ("rvps", "educationNote")
        }}}
        return {"slug": slug, "isco": isco, "education": education}

    units = []
    with ThreadPoolExecutor(max_workers=3) as pool:
        for i, result in enumerate(pool.map(detail, slugs), 1):
            units.append(result)
            if i % 50 == 0 or i == len(slugs):
                print(f"NSP details: {i}/{len(slugs)}", flush=True)
    return {"source": NSP, "jobs_sha256": hashlib.sha256(JOBS.read_bytes()).hexdigest(),
            "dictionary": index,
            "suitability": request(NSP + "/lists/kKOVSuitabilityLevel", refresh),
            "groups": result_groups, "work_units": units}


def fetch_ruian(jobs, refresh):
    ids = sorted({str(w["adresa"]["kodAdresnihoMista"])
                  for j in jobs for w in j["mistoVykonuPrace"]["pracoviste"]
                  if w["adresa"].get("kodAdresnihoMista")})
    pages = []
    for start in range(0, len(ids), 100):
        batch = ids[start:start + 100]
        if not all(i.isdigit() for i in batch):
            raise ValueError("Invalid address code")
        query = urlencode({"where": "kod IN (" + ",".join(batch) + ")",
                           "outFields": "kod,adresa,nespravny,platiod,platido",
                           "returnGeometry": "true", "outSR": "4326", "f": "json"})
        page = request(RUIAN + "?" + query, refresh)
        if page["response"].get("exceededTransferLimit"):
            raise ValueError("Truncated RUIAN response")
        pages.append(page)
    return {"source": RUIAN, "requested_address_codes": ids, "pages": pages}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--refresh", action="store_true")
    args = parser.parse_args()
    jobs = json.loads(JOBS.read_text(encoding="utf-8"))["polozky"]
    nsp = fetch_nsp(jobs, args.refresh)
    save(ROOT / "datasety/nsp_mapovani_mvp.json", nsp)
    ruian = fetch_ruian(jobs, args.refresh)
    save(ROOT / "datasety/ruian_pracoviste_mvp.json", ruian)
    print("Saved NSP and RUIAN source snapshots; no database connection was made.")


if __name__ == "__main__":
    main()
