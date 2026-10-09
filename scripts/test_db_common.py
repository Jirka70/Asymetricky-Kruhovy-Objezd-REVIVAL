"""Run with: python3 -m unittest discover -s scripts -p 'test_db_common.py'."""

import os
from pathlib import Path
import subprocess
import sys
import unittest


class SubprocessEncodingTests(unittest.TestCase):
    def test_unicode_round_trip_without_utf8_locale(self):
        # Start a fresh interpreter so subprocess sees the non-UTF-8 locale.
        environment = dict(os.environ, LC_ALL="C", PYTHONUTF8="0", PYTHONCOERCECLOCALE="0")
        probe = r'''
import locale
import sys
from db_common import run

assert locale.getpreferredencoding(False).lower().replace("-", "") != "utf8"
value = "P\u0159\u00edbram, \u017d\u010f\u00e1r nad S\u00e1zavou"
receiver = """
import sys
data = sys.stdin.buffer.read()
data.decode("utf-8")
sys.stdout.buffer.write(data)
sys.stderr.buffer.write(data)
"""
result = run([sys.executable, "-c", receiver], input=value)
assert result.stdout == value
assert result.stderr == value
'''
        result = subprocess.run(
            [sys.executable, "-c", probe],
            cwd=Path(__file__).resolve().parent,
            env=environment,
            capture_output=True,
            encoding="utf-8",
            timeout=15,
        )
        self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()
