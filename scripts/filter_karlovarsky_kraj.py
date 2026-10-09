#!/usr/bin/env python3
"""Write a CSV containing all study-program rows for Karlovarský kraj.

Run from any directory:
    python3 filter_karlovarsky_kraj.py
Or supply input and output paths:
    python3 filter_karlovarsky_kraj.py input.csv output.csv

Uses only Python's standard library. CSVs use commas and UTF-8 with a BOM
so Czech characters also display correctly when opened in Excel.
"""

import argparse
import csv
from pathlib import Path

DATA_DIR = Path(__file__).resolve().parent / "datasety"
DEFAULT_INPUT = DATA_DIR / "PZ2026_kolo1_skolobory_prihlasky.csv"
DEFAULT_OUTPUT = DATA_DIR / "PZ2026_kolo1_skolobory_prihlasky_karlovarsky_kraj.csv"
REGION_COLUMN = "KRAJ"
REGION_CODE = "CZ041"  # Karlovarský in the source workbook's KRAJ - NÁZEV.


def filter_csv(input_path: Path, output_path: Path) -> tuple[int, int]:
    """Preserve the header, column order and all matching rows."""
    if input_path.resolve() == output_path.resolve():
        raise ValueError("Input and output must be different files.")

    with input_path.open("r", encoding="utf-8-sig", newline="") as source:
        reader = csv.reader(source)
        header = next(reader, None)
        if not header or REGION_COLUMN not in header:
            raise ValueError(f"Input CSV must contain the {REGION_COLUMN!r} column.")
        region_index = header.index(REGION_COLUMN)
        total = 0
        selected = []
        for line_number, row in enumerate(reader, start=2):
            if len(row) != len(header):
                raise ValueError(f"CSV record {line_number} has {len(row)} columns; expected {len(header)}.")
            total += 1
            if row[region_index].strip() == REGION_CODE:
                selected.append(row)

    with output_path.open("w", encoding="utf-8-sig", newline="") as target:
        writer = csv.writer(target)
        writer.writerow(header)
        writer.writerows(selected)
    return total, len(selected)


def main() -> None:
    parser = argparse.ArgumentParser(description="Filter schools in Karlovarský kraj (CZ041).")
    parser.add_argument("input", nargs="?", type=Path, default=DEFAULT_INPUT)
    parser.add_argument("output", nargs="?", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    try:
        total, selected = filter_csv(args.input, args.output)
    except (OSError, ValueError, csv.Error) as error:
        parser.exit(1, f"Error: {error}\n")
    print(f"Saved {selected} of {total} records to {args.output}")


if __name__ == "__main__":
    main()
