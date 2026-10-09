#!/usr/bin/env python3
"""Filter jobs with at least one workplace explicitly in Karlovarský kraj.

Uses mistoVykonuPrace.pracoviste[].adresa.kraj.id == "Kraj/51".
Contact addresses do not determine where the job is located.
Jobs specified only by municipality/district are not inferred from those codes.
Preserves complete matching records, their order and the top-level structure.

Usage:
    python3 scripts/filter_jobs_karlovarsky_kraj.py
    python3 scripts/filter_jobs_karlovarsky_kraj.py input.json output.json

Requires only Python's standard library.
"""
import argparse
import json
from pathlib import Path

DATA_DIR = Path(__file__).resolve().parents[1] / "datasety"
REGION_ID = "Kraj/51"


def is_in_karlovarsky_kraj(job):
    """Include a job once if any workplace address has the requested region."""
    location = job.get("mistoVykonuPrace") or {}
    return any(
        (((workplace or {}).get("adresa") or {}).get("kraj") or {}).get("id")
        == REGION_ID
        for workplace in location.get("pracoviste") or []
    )


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
    selected = [job for job in jobs if is_in_karlovarsky_kraj(job)]
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
                        default=DATA_DIR / "volna-mista_karlovarsky_kraj.json")
    args = parser.parse_args()
    try:
        total, count = filter_jobs(args.input, args.output)
    except (OSError, ValueError, TypeError, AttributeError) as error:
        parser.exit(1, f"Error: {error}\n")
    print(f"Saved {count} of {total} jobs to {args.output}")


if __name__ == "__main__":
    main()
