from __future__ import annotations

import contextlib
import io
import tempfile
import unittest
from pathlib import Path

from check_tool.__main__ import main as check_main
from common import CHECK_SCHEMA, CHECK_VERSION, read_json
from derive_tool.__main__ import main as derive_main
from model_tool.__main__ import main as model_main
from parse_tool.__main__ import main as parse_main


class CheckToolTests(unittest.TestCase):
    def setUp(self) -> None:
        self.spec = (
            Path(__file__).resolve().parents[3]
            / "spec"
            / "entry-prelude-object-model.spec"
        )

    def _build_derive_json(self, tmp: str) -> Path:
        ast = Path(tmp) / "entry-prelude-object-model.ast.json"
        model = Path(tmp) / "entry-prelude-object-model.model.json"
        derive = Path(tmp) / "entry-prelude-object-model.derive.json"
        self.assertEqual(parse_main([str(self.spec), "-o", str(ast)]), 0)
        self.assertEqual(model_main([str(ast), "-o", str(model)]), 0)
        self.assertEqual(derive_main([str(model), "-o", str(derive)]), 0)
        return derive

    def test_check_writes_check_json_for_passing_derivation(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            derive = self._build_derive_json(tmp)
            check = Path(tmp) / "entry-prelude-object-model.check.json"

            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exit_code = check_main([str(derive), "-o", str(check)])

            self.assertEqual(exit_code, 0)
            data = read_json(check)
            self.assertEqual(data["schema"], CHECK_SCHEMA)
            self.assertEqual(data["version"], CHECK_VERSION)
            self.assertEqual(data["policy"], "default")
            self.assertEqual(data["verdict"], "passed")
            self.assertEqual(data["exit_code"], 0)
            self.assertTrue(data["summary"]["target_reached"])
            self.assertEqual(data["summary"]["blocked"], 0)
            self.assertEqual(data["summary"]["contradiction"], 0)
            self.assertGreater(data["summary"]["obligation"], 0)
            self.assertTrue(data["allowed"]["obligation"])
            self.assertIn("check: passed", stdout.getvalue())

    def test_check_fails_when_target_not_reached(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            derive = self._build_derive_json(tmp)
            data = read_json(derive)
            data["summary"]["target_reached"] = False
            data["target"]["reached"] = False
            derive.write_text(__import__("json").dumps(data), encoding="utf-8")
            check = Path(tmp) / "failed.check.json"

            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exit_code = check_main([str(derive), "-o", str(check)])

            self.assertEqual(exit_code, 1)
            result = read_json(check)
            self.assertEqual(result["verdict"], "failed")
            self.assertEqual(result["reasons"][0]["kind"], "target_not_reached")

    def test_invalid_derive_schema_returns_usage_error_code(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            derive = Path(tmp) / "bad.derive.json"
            check = Path(tmp) / "bad.check.json"
            derive.write_text('{"schema": "wrong", "version": 1}\n', encoding="utf-8")

            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exit_code = check_main([str(derive), "-o", str(check)])

            self.assertEqual(exit_code, 2)
            self.assertIn("error: invalid derive JSON", stderr.getvalue())


if __name__ == "__main__":
    unittest.main()
