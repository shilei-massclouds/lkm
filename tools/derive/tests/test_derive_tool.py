from __future__ import annotations

import contextlib
import io
import tempfile
import unittest
from pathlib import Path

from common import DERIVE_SCHEMA, DERIVE_VERSION, read_json
from derive_tool.__main__ import main as derive_main
from model_tool.__main__ import main as model_main
from parse_tool.__main__ import main as parse_main


class DeriveToolTests(unittest.TestCase):
    def setUp(self) -> None:
        self.spec = (
            Path(__file__).resolve().parents[3]
            / "spec"
            / "entry-prelude-object-model.spec"
        )

    def _build_model_json(self, tmp: str) -> Path:
        ast = Path(tmp) / "entry-prelude-object-model.ast.json"
        model = Path(tmp) / "entry-prelude-object-model.model.json"
        self.assertEqual(parse_main([str(self.spec), "-o", str(ast)]), 0)
        self.assertEqual(model_main([str(ast), "-o", str(model)]), 0)
        return model

    def test_derive_writes_derive_json(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            model = self._build_model_json(tmp)
            derive = Path(tmp) / "entry-prelude-object-model.derive.json"

            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exit_code = derive_main([str(model), "-o", str(derive)])

            self.assertEqual(exit_code, 0)
            data = read_json(derive)
            self.assertEqual(data["schema"], DERIVE_SCHEMA)
            self.assertEqual(data["version"], DERIVE_VERSION)
            self.assertTrue(data["target"]["reached"])
            self.assertTrue(data["summary"]["ok"])
            self.assertEqual(data["summary"]["blocked"], 0)
            self.assertEqual(data["summary"]["contradiction"], 0)
            self.assertEqual(data["states"]["StartupTimeline"], "Ready")

    def test_derive_json_contains_records_transitions_and_trace(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            model = self._build_model_json(tmp)
            derive = Path(tmp) / "entry-prelude-object-model.derive.json"
            self.assertEqual(derive_main([str(model), "-o", str(derive)]), 0)

            data = read_json(derive)
            self.assertEqual(data["summary"]["transitions"], 28)
            self.assertGreater(data["summary"]["obligation"], 0)
            self.assertTrue(
                any(
                    transition["object"] == "StartupTimeline"
                    and transition["event"] == "Setup"
                    for transition in data["transitions"]
                )
            )
            self.assertEqual(len(data["trace"]), 1)
            root = data["trace"][0]
            self.assertEqual(root["object"], "StartupTimeline")
            self.assertEqual(root["event"], "Setup")
            self.assertEqual(root["source_state"], "Base")
            self.assertEqual(root["target_state"], "Ready")
            self.assertEqual(root["status"], "proved")
            self.assertGreater(len(root["children"]), 0)
            self.assertTrue(
                any(record["span"] is not None for record in data["records"])
            )

    def test_invalid_model_schema_returns_usage_error_code(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            model = Path(tmp) / "bad.model.json"
            derive = Path(tmp) / "bad.derive.json"
            model.write_text('{"schema": "wrong", "version": 1}\n', encoding="utf-8")

            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exit_code = derive_main([str(model), "-o", str(derive)])

            self.assertEqual(exit_code, 2)
            self.assertIn("error: invalid model JSON", stderr.getvalue())

    def test_derive_tool_does_not_import_pyveri(self) -> None:
        source_root = Path(__file__).resolve().parents[1] / "src" / "derive_tool"

        for path in source_root.rglob("*.py"):
            text = path.read_text(encoding="utf-8")
            self.assertNotIn("from pyveri", text, str(path))
            self.assertNotIn("import pyveri", text, str(path))


if __name__ == "__main__":
    unittest.main()
