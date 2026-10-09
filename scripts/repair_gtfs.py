#!/usr/bin/env python3
"""Remove stop_times rows referencing absent trips; preserve the original ZIP.

Usage: python3 scripts/repair_gtfs.py location_data/vlaky.gtfs.zip
The original is kept as <archive>.original and reused on subsequent runs.
"""
import csv
import io
import sys
import shutil
import tempfile
import zipfile
from pathlib import Path


def repair(path: Path) -> None:
    original = path.with_name(path.name + ".original")
    source = original if original.exists() else path
    removed = 0
    with tempfile.TemporaryDirectory() as directory:
        temporary = Path(directory) / path.name
        with zipfile.ZipFile(source) as archive:
            with archive.open("trips.txt") as data:
                trips = {
                    row["trip_id"]
                    for row in csv.DictReader(
                        io.TextIOWrapper(data, encoding="utf-8-sig", newline="")
                    )
                }
            with zipfile.ZipFile(temporary, "w", compression=zipfile.ZIP_DEFLATED) as output:
                for info in archive.infolist():
                    if info.filename != "stop_times.txt":
                        # Stream large entries rather than holding the whole feed in memory.
                        with archive.open(info) as src, output.open(info, "w") as dst:
                            shutil.copyfileobj(src, dst)
                        continue
                    with archive.open(info) as src, output.open(info, "w") as dst:
                        reader = csv.DictReader(
                            io.TextIOWrapper(src, encoding="utf-8-sig", newline="")
                        )
                        text = io.TextIOWrapper(dst, encoding="utf-8", newline="")
                        writer = csv.DictWriter(text, fieldnames=reader.fieldnames)
                        writer.writeheader()
                        for row in reader:
                            if row["trip_id"] in trips:
                                writer.writerow(row)
                            else:
                                removed += 1
                        text.flush()
                        text.detach()
        if not original.exists():
            path.rename(original)
        shutil.copyfile(temporary, path)
    print(f"Removed {removed} orphan stop-time rows; original preserved at {original}")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit("Usage: python3 scripts/repair_gtfs.py feed.gtfs.zip")
    repair(Path(sys.argv[1]))
