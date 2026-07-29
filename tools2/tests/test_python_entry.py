from __future__ import annotations

import importlib
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
TEST_ENTRY = ROOT / "tools2" / "bin" / "test-python"
PROBE = "tools2.tests.test_python_entry.Tools2SourceImportProbe.test_all_packages_import"


class Tools2SourceImportProbe(unittest.TestCase):
    def test_all_packages_import(self) -> None:
        for module in (
            "animate_tool",
            "check_tool",
            "derive_tool",
            "model_tool",
            "parse_tool",
            "pyveri",
            "render_tool",
            "tools2_common",
            "view_tool",
        ):
            with self.subTest(module=module):
                importlib.import_module(module)


class Tools2PythonEntryTests(unittest.TestCase):
    @staticmethod
    def _clean_environment() -> dict[str, str]:
        environment = os.environ.copy()
        environment.pop("PYTHONPATH", None)
        environment["PYTHONDONTWRITEBYTECODE"] = "1"
        return environment

    def test_dotted_unittest_bootstraps_without_pythonpath(self) -> None:
        result = subprocess.run(
            [sys.executable, "-m", "unittest", PROBE, "-v"],
            cwd=ROOT,
            env=self._clean_environment(),
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_entry_runs_focused_test_outside_repository(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            result = subprocess.run(
                [sys.executable, str(TEST_ENTRY), PROBE, "-v"],
                cwd=temporary,
                env=self._clean_environment(),
                text=True,
                capture_output=True,
                check=False,
            )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
