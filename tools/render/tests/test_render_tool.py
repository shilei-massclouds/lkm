from __future__ import annotations

import contextlib
import io
import tempfile
import unittest
from pathlib import Path

from model_tool.__main__ import main as model_main
from parse_tool.__main__ import main as parse_main
from render_tool.__main__ import main as render_main
from view_tool.__main__ import main as view_main


class RenderToolTests(unittest.TestCase):
    def setUp(self) -> None:
        self.spec = (
            Path(__file__).resolve().parents[3]
            / "spec"
            / "entry-prelude-object-model.spec"
        )

    def _build_view_json(self, tmp: str, view_name: str) -> Path:
        ast = Path(tmp) / "entry-prelude-object-model.ast.json"
        model = Path(tmp) / "entry-prelude-object-model.model.json"
        view = Path(tmp) / f"entry-prelude-object-model.{view_name}.view.json"
        self.assertEqual(parse_main([str(self.spec), "-o", str(ast)]), 0)
        self.assertEqual(model_main([str(ast), "-o", str(model)]), 0)
        self.assertEqual(view_main([str(model), view_name, "-o", str(view)]), 0)
        return view

    def test_render_text_to_stdout(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            view = self._build_view_json(tmp, "object")

            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exit_code = render_main([str(view), "--format", "text"])

            self.assertEqual(exit_code, 0)
            self.assertIn("object view:", stdout.getvalue())
            self.assertIn("StartupTimeline: TimelineObject", stdout.getvalue())

    def test_render_dot_writes_ascii_file(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            view = self._build_view_json(tmp, "object")
            output = Path(tmp) / "object.gv"

            exit_code = render_main([str(view), "--format", "dot", "-o", str(output)])

            self.assertEqual(exit_code, 0)
            data = output.read_bytes()
            self.assertTrue(data.startswith(b"digraph ObjectView"))
            data.decode("ascii")

    def test_render_svg_from_timeline_view(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            view = self._build_view_json(tmp, "timeline")
            output = Path(tmp) / "timeline.svg"

            exit_code = render_main([str(view), "--format", "svg", "-o", str(output)])

            self.assertEqual(exit_code, 0)
            text = output.read_text(encoding="utf-8")
            self.assertTrue(text.startswith('<?xml version="1.0" encoding="UTF-8"?>'))
            self.assertIn("<svg", text)
            self.assertIn("PreparePhase", text)
            self.assertIn("BootPhase", text)

    def test_svg_requires_timeline_view(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            view = self._build_view_json(tmp, "object")
            output = Path(tmp) / "object.svg"

            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exit_code = render_main([str(view), "--format", "svg", "-o", str(output)])

            self.assertEqual(exit_code, 2)
            self.assertIn("error: cannot render view JSON", stderr.getvalue())

    def test_invalid_view_schema_returns_usage_error_code(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            view = Path(tmp) / "bad.view.json"
            view.write_text('{"schema": "wrong", "version": 1}\n', encoding="utf-8")

            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exit_code = render_main([str(view), "--format", "text"])

            self.assertEqual(exit_code, 2)
            self.assertIn("error: cannot render view JSON", stderr.getvalue())


if __name__ == "__main__":
    unittest.main()
