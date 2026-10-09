#!/usr/bin/env python3
"""Extract jobs without a region specified in any workplace address.

A job qualifies when none of its
mistoVykonuPrace.pracoviste[].adresa.kraj.id values is a non-empty string.
Missing workplaces, missing addresses, null kraj and blank IDs qualify.
A job with any known workplace region is excluded, even if another is missing.
Contact addresses do not determine the job's region. Municipality and district
codes are not resolved to regions. Complete records and ordering are preserved.

Usage:
    python3 scripts/filter_jobs_without_kraj.py
    python3 scripts/filter_jobs_without_kraj.py input.json output.json

Requires only Python's standard library.
"""
import argparse
import json
from pathlib import Path

DATA_DIR = Path(__file__).resolve().parents[1] / "datasety"


def has_no_kraj(job):
    """Keep jobs with no usable workplace region ID."""
    location = job.get("mistoVykonuPrace") or {}
    for workplace in location.get("pracoviste") or []:
        address = (workplace or {}).get("adresa") or {}
        region = address.get("kraj") or {}
        region_id = region.get("id")
        if isinstance(region_id, str) and region_id.strip():
            return False
    return True


def filter_jobs(input_path, output_path):
    if input_path.resolve() == output_path.resolve():
        raise ValueError("Input and output must be different files.")
    with input_path.open(encoding="utf-8-sig") as source:
        data = json.load(source)
    if not isinstance(data, dict) or not isinstance(data.get("polozky"), list):
        raise ValueError("Expected a JSON object with a 'polozky' array.")
    jobs = data["polozky"]
    if not all(isinstance(job, dict) for job in jobs):
        raise ValueError("Every entry in 'polozky' must be a job object.")
    selected = [job for job in jobs if has_no_kraj(job)]
    output = {**data, "polozky": selected}
    with output_path.open("w", encoding="utf-8") as target:
        json.dump(output, target, ensure_ascii=False, indent=2)
        target.write("\n")
    return len(jobs), len(selected)


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("input", type=Path, nargs="?",
                        default=DATA_DIR / "volna-mista.json")
    parser.add_argument("output", type=Path, nargs="?",
                        default=DATA_DIR / "volna-mista_bez_kraje.json")
    args = parser.parse_args()
    try:
        total, count = filter_jobs(args.input, args.output)
    except (OSError, ValueError, TypeError, AttributeError) as error:
        parser.exit(1, f"Error: {error}\n")
    print(f"Saved {count} of {total} jobs to {args.output}")


if __name__ == "__main__":
    main()
