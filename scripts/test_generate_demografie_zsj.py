"""Run with: python3 -m unittest discover -s scripts -p 'test_generate_demografie_zsj.py'."""

import csv
import itertools
import json
import random
import tempfile
import unittest
from pathlib import Path

from generate_demografie_zsj import load_sources, round_matrix


class ControlledRoundingTests(unittest.TestCase):
    def check_margins(self, rows, columns):
        matrix = round_matrix(rows, columns)
        self.assertEqual([sum(row) for row in matrix], rows)
        self.assertEqual([sum(row[j] for row in matrix) for j in range(len(columns))], columns)
        total = sum(rows)
        for i, row in enumerate(matrix):
            for j, value in enumerate(row):
                self.assertIsInstance(value, int)
                self.assertGreaterEqual(value, 0)
                if total:
                    self.assertLess(abs(value * total - rows[i] * columns[j]), total)
        return matrix

    def test_zero_and_integral_populations(self):
        self.assertEqual(self.check_margins([0, 0], [0, 0, 0]), [[0, 0, 0], [0, 0, 0]])
        self.assertEqual(self.check_margins([0, 10], [3, 0, 7]), [[0, 0, 0], [3, 0, 7]])

    def test_rounding_requires_coordination_between_zsj(self):
        # Independently rounding each identical ZSJ would assign both to one age.
        self.check_margins([1, 1], [1, 1])
        self.check_margins([1, 1, 1], [1, 1, 1])
        self.check_margins([2, 3, 4], [1, 3, 5])

    def test_minimum_error_against_exhaustive_solutions(self):
        for rows, columns in [([1, 1, 1], [1, 1, 1]), ([2, 3], [2, 1, 2]),
                              ([2, 5, 2], [4, 5]), ([1, 4, 2], [3, 2, 2])]:
            with self.subTest(rows=rows, columns=columns):
                total = sum(rows)
                options = []
                for r in rows:
                    for c in columns:
                        floor, remainder = divmod(r * c, total)
                        options.append((floor, floor + 1) if remainder else (floor,))
                best = None
                for flat in itertools.product(*options):
                    matrix = [flat[i:i + len(columns)] for i in range(0, len(flat), len(columns))]
                    if [sum(row) for row in matrix] != rows:
                        continue
                    if [sum(row[j] for row in matrix) for j in range(len(columns))] != columns:
                        continue
                    error = sum(abs(matrix[i][j] * total - r * c)
                                for i, r in enumerate(rows) for j, c in enumerate(columns))
                    best = error if best is None else min(best, error)
                actual = self.check_margins(rows, columns)
                actual_error = sum(abs(actual[i][j] * total - r * c)
                                   for i, r in enumerate(rows) for j, c in enumerate(columns))
                self.assertIsNotNone(best)
                self.assertEqual(actual_error, best)

    def test_varied_margins_and_deterministic_ties(self):
        randomizer = random.Random(2021)
        for _ in range(100):
            rows = [randomizer.randrange(20) for _ in range(randomizer.randrange(1, 8))]
            columns = [0] * randomizer.randrange(1, 8)
            for _ in range(sum(rows)):
                columns[randomizer.randrange(len(columns))] += 1
            first = self.check_margins(rows, columns)
            self.assertEqual(first, round_matrix(rows, columns))

    def test_invalid_margins_are_rejected(self):
        for rows, columns in [([1], [2]), ([-1, 2], [1]), ([1.0], [1])]:
            with self.assertRaises(ValueError):
                round_matrix(rows, columns)


class SourceValidationTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        root = Path(self.directory.name)
        self.geo, self.population, self.ages = [root / p for p in ("zsj.json", "population.csv", "ages.csv")]
        self.geo.write_text(json.dumps({"type": "FeatureCollection", "features": [
            {"properties": {"kod_zsj": "000019", "kod_obec": "554979",
                            "nuts3_kraj": "CZ041", "kod_kraj": "3051"}},
            {"properties": {"kod_zsj": "000027", "kod_obec": "555177",
                            "nuts3_kraj": "CZ041", "kod_kraj": "3051"}},
            {"properties": {"kod_zsj": "999999", "kod_obec": "999999",
                            "nuts3_kraj": "CZ010", "kod_kraj": "3018"}},
        ]}), encoding="utf-8")
        self.common = ["uzemi_cis", "uzemi_kod", "ukaz_kod", "sldb_rok", "sldb_datum", "hodnota"]
        self.pop_rows = [
            ["47", "000019", "3162", "2021", "2021-03-26", "5"],
            ["47", "000027", "3162", "2021", "2021-03-26", "0"],
            ["42", "000019", "3162", "2021", "2021-03-26", "999"],
            ["47", "000019", "2607", "2021", "2021-03-26", "999"],
        ]
        self.age_rows = []
        for level, code in [("43", "554979"), ("100", "3051")]:
            for age in [None, *range(0, 101, 5)]:
                label = "" if age is None else "100 a více let" if age == 100 else f"{age} - {age + 4} let"
                age_code = "" if age is None else f"age-{age}"
                for sex in ["", "1", "2"]:
                    count = 5 if age in [None, 15] and sex != "2" else 0
                    self.age_rows.append([level, code, "3162", "2021", "2021-03-26", str(count),
                                          "1035" if age is not None else "", age_code, label, sex])
        self.write_files()

    def write_files(self):
        for path, fields, rows in [
            (self.population, self.common, self.pop_rows),
            (self.ages, self.common + ["vek_cis", "vek_kod", "vek_txt", "pohlavi_kod"], self.age_rows),
        ]:
            with path.open("w", encoding="utf-8-sig", newline="") as target:
                writer = csv.writer(target)
                writer.writerow(fields)
                writer.writerows(rows)

    def load(self):
        return load_sources(self.geo, self.population, self.ages)

    def test_territory_filters_codes_and_verified_zero_exception(self):
        census = self.load()
        self.assertEqual(census.zsj_population, {"000019": 5, "000027": 0})
        self.assertEqual(census.zero_missing, ["555177"])
        self.assertEqual(sum(census.municipality_age["555177"].values()), 0)
        self.assertEqual(census.municipality_age["554979"]["age-15"], 5)

    def test_missing_ages_for_populated_municipality_are_rejected(self):
        self.pop_rows[1][-1] = "1"
        self.write_files()
        with self.assertRaisesRegex(ValueError, "Missing ages for populated municipality"):
            self.load()

    def test_duplicate_population_is_rejected(self):
        self.pop_rows.append(self.pop_rows[0])
        self.write_files()
        with self.assertRaisesRegex(ValueError, "Duplicate population"):
            self.load()

    def test_partial_age_data_are_rejected(self):
        self.age_rows = [r for r in self.age_rows if not (r[0] == "43" and r[7] == "age-15")]
        self.write_files()
        with self.assertRaisesRegex(ValueError, "Incomplete ages/sexes"):
            self.load()

    def test_conflicting_municipality_population_is_rejected(self):
        self.pop_rows[0][-1] = "6"
        self.write_files()
        with self.assertRaisesRegex(ValueError, "ZSJ sum differs"):
            self.load()


if __name__ == "__main__":
    unittest.main()
