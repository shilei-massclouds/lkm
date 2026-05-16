from __future__ import annotations

import contextlib
import io
import tempfile
import unittest
from pathlib import Path

from common import MODEL_SCHEMA, MODEL_VERSION, read_json
from model_tool.__main__ import main as model_main
from parse_tool.__main__ import main as parse_main


class ModelToolTests(unittest.TestCase):
    def setUp(self) -> None:
        self.spec = (
            Path(__file__).resolve().parents[3]
            / "spec"
            / "entry-prelude-object-model.spec"
        )

    def test_model_writes_model_json(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            ast = Path(tmp) / "entry-prelude-object-model.ast.json"
            model = Path(tmp) / "entry-prelude-object-model.model.json"

            self.assertEqual(parse_main([str(self.spec), "-o", str(ast)]), 0)
            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 0)
            data = read_json(model)
            self.assertEqual(data["schema"], MODEL_SCHEMA)
            self.assertEqual(data["version"], MODEL_VERSION)
            self.assertTrue(data["summary"]["ok"])
            self.assertEqual(data["summary"]["objects"], 25)
            self.assertEqual(data["summary"]["errors"], 0)
            self.assertIn("StartupTimeline", data["model"]["objects"])

    def test_model_json_contains_indexed_children_and_events(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            ast = Path(tmp) / "entry-prelude-object-model.ast.json"
            model = Path(tmp) / "entry-prelude-object-model.model.json"

            self.assertEqual(parse_main([str(self.spec), "-o", str(ast)]), 0)
            self.assertEqual(model_main([str(ast), "-o", str(model)]), 0)
            data = read_json(model)
            objects = data["model"]["objects"]
            startup = objects["StartupTimeline"]
            setup = startup["states"]["Base"]["events"]["Setup"]

            self.assertEqual(startup["children"], ["PreparePhase", "BootPhase"])
            self.assertEqual(setup["source_state"], "Base")
            self.assertEqual(setup["target_state"], "Ready")
            self.assertEqual(
                objects["PhysicalMemory"]["properties"]["access"],
                "Access::ReadOnly",
            )
            self.assertEqual(
                [entry["text"] for entry in setup["drives"][0]["entries"]],
                [
                    "PreparePhase.Event::Setup",
                    "PreparePhase.Event::Enable",
                    "BootPhase.Event::Setup",
                ],
            )

    def test_model_json_preserves_entry_spans(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            ast = Path(tmp) / "entry-prelude-object-model.ast.json"
            model = Path(tmp) / "entry-prelude-object-model.model.json"

            self.assertEqual(parse_main([str(self.spec), "-o", str(ast)]), 0)
            self.assertEqual(model_main([str(ast), "-o", str(model)]), 0)
            data = read_json(model)
            kernel_image = data["model"]["objects"]["KernelImage"]
            enable = kernel_image["states"]["Ready"]["events"]["Enable"]
            entry = enable["depends_on"][0]["entries"][0]

            self.assertEqual(entry["text"], "EarlyVm.state == State::Online")
            line = self.spec.read_text(encoding="utf-8").splitlines()[
                entry["span"]["start_line"] - 1
            ]
            self.assertIn("EarlyVm.state == State::Online", line)

    def test_invalid_ast_schema_returns_usage_error_code(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            ast = Path(tmp) / "bad.ast.json"
            model = Path(tmp) / "bad.model.json"
            ast.write_text('{"schema": "wrong", "version": 1}\n', encoding="utf-8")

            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 2)
            self.assertIn("error: invalid AST JSON", stderr.getvalue())

    def test_model_tool_does_not_import_pyveri(self) -> None:
        source_root = Path(__file__).resolve().parents[1] / "src" / "model_tool"

        for path in source_root.rglob("*.py"):
            text = path.read_text(encoding="utf-8")
            self.assertNotIn("from pyveri", text, str(path))
            self.assertNotIn("import pyveri", text, str(path))


if __name__ == "__main__":
    unittest.main()
