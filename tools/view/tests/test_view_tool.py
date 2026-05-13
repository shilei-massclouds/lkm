from __future__ import annotations

import contextlib
import io
import tempfile
import unittest
from pathlib import Path

from common import VIEW_SCHEMA, VIEW_VERSION, read_json
from derive_tool.__main__ import main as derive_main
from model_tool.__main__ import main as model_main
from parse_tool.__main__ import main as parse_main
from view_tool.__main__ import main as view_main


class ViewToolTests(unittest.TestCase):
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

    def _build_derive_json(self, tmp: str) -> Path:
        model = self._build_model_json(tmp)
        derive = Path(tmp) / "entry-prelude-object-model.derive.json"
        self.assertEqual(derive_main([str(model), "-o", str(derive)]), 0)
        return derive

    def test_object_view_writes_view_json(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            model = self._build_model_json(tmp)
            output = Path(tmp) / "object.view.json"

            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exit_code = view_main([str(model), "object", "-o", str(output)])

            self.assertEqual(exit_code, 0)
            data = read_json(output)
            self.assertEqual(data["schema"], VIEW_SCHEMA)
            self.assertEqual(data["version"], VIEW_VERSION)
            self.assertEqual(data["view"], "object")
            self.assertIn("StartupTimeline", data["nodes"])
            self.assertTrue(
                any(
                    edge["source"] == "StartupTimeline"
                    and edge["target"] == "PreparePhase"
                    and edge["kind"] == "parent"
                    for edge in data["edges"]
                )
            )

    def test_drives_view_contains_event_edges(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            model = self._build_model_json(tmp)
            output = Path(tmp) / "drives.view.json"
            self.assertEqual(view_main([str(model), "drives", "-o", str(output)]), 0)

            data = read_json(output)
            self.assertEqual(data["view"], "drives")
            self.assertEqual(data["rankdir"], "LR")
            self.assertIn("StartupTimeline.Setup", data["nodes"])
            self.assertTrue(
                any(
                    edge["source"] == "StartupTimeline.Setup"
                    and edge["target"] == "PreparePhase.Setup"
                    for edge in data["edges"]
                )
            )

    def test_timeline_view_contains_rows_metadata(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            model = self._build_model_json(tmp)
            output = Path(tmp) / "timeline.view.json"
            self.assertEqual(view_main([str(model), "timeline", "-o", str(output)]), 0)

            data = read_json(output)
            self.assertEqual(data["view"], "timeline")
            self.assertEqual(data["graph_format"], "svg")
            rows = data["metadata"]["timeline_rows"]
            self.assertTrue(any(row["phase"] == "PreparePhase" for row in rows))
            self.assertTrue(any(row["phase"] == "BootPhase" for row in rows))

    def test_trace_view_contains_cell_layout_metadata(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            derive = self._build_derive_json(tmp)
            output = Path(tmp) / "trace.view.json"
            self.assertEqual(view_main([str(derive), "trace", "-o", str(output)]), 0)

            data = read_json(output)
            self.assertEqual(data["view"], "trace")
            metadata = data["metadata"]
            self.assertTrue(
                any(column["kind"] == "gap" for column in metadata["trace_columns"])
            )
            self.assertTrue(any(row["kind"] == "gap" for row in metadata["trace_rows"]))
            self.assertTrue(
                any(
                    cell["kind"] == "event_span"
                    and cell["label"] == "StartupTimeline.Event::Setup"
                    for cell in metadata["trace_cells"]
                )
            )
            self.assertTrue(
                any(arrow["kind"] == "drives" for arrow in metadata["trace_arrows"])
            )

    def test_invalid_model_schema_returns_usage_error_code(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            model = Path(tmp) / "bad.model.json"
            output = Path(tmp) / "bad.view.json"
            model.write_text('{"schema": "wrong", "version": 1}\n', encoding="utf-8")

            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exit_code = view_main([str(model), "object", "-o", str(output)])

            self.assertEqual(exit_code, 2)
            self.assertIn("error: invalid input JSON", stderr.getvalue())

    def test_view_tool_does_not_import_pyveri(self) -> None:
        source_root = Path(__file__).resolve().parents[1] / "src" / "view_tool"

        for path in source_root.rglob("*.py"):
            text = path.read_text(encoding="utf-8")
            self.assertNotIn("from pyveri", text, str(path))
            self.assertNotIn("import pyveri", text, str(path))


if __name__ == "__main__":
    unittest.main()
