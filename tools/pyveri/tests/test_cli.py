from __future__ import annotations

import contextlib
import io
import tempfile
import unittest
from pathlib import Path

from pyveri.__main__ import main


class CliTests(unittest.TestCase):
    def setUp(self) -> None:
        self.spec = (
            Path(__file__).resolve().parents[3]
            / "spec"
            / "entry-prelude-object-model.spec"
        )

    def test_graph_output_file_is_ascii_dot(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "object.gv"
            exit_code = main([str(self.spec), "--graph", "object", "-o", str(output)])

            self.assertEqual(exit_code, 0)
            data = output.read_bytes()
            self.assertTrue(data.startswith(b"digraph ObjectView"))
            data.decode("ascii")

    def test_strict_derivation_succeeds_for_current_spec(self) -> None:
        stdout = io.StringIO()
        stderr = io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            exit_code = main([str(self.spec), "--derive", "--strict"])

        self.assertEqual(exit_code, 0)

    def test_parse_command_prints_parse_summary(self) -> None:
        stdout = io.StringIO()
        stderr = io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            exit_code = main(["parse", str(self.spec)])

        self.assertEqual(exit_code, 0)
        self.assertIn("parse: ok", stdout.getvalue())

    def test_model_command_prints_model_summary(self) -> None:
        stdout = io.StringIO()
        stderr = io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            exit_code = main(["model", str(self.spec)])

        self.assertEqual(exit_code, 0)
        self.assertIn("model: ok", stdout.getvalue())

    def test_derive_command_prints_derivation_report(self) -> None:
        stdout = io.StringIO()
        stderr = io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            exit_code = main(["derive", str(self.spec), "--strict"])

        self.assertEqual(exit_code, 0)
        text = stdout.getvalue()
        self.assertIn("derive: ok", text)
        self.assertIn("trace:", text)
        self.assertIn("> StartupTimeline.Event::Setup State::Base", text)
        self.assertIn("< StartupTimeline.Event::Setup State::Ready", text)

    def test_check_command_uses_strict_derivation_exit_code(self) -> None:
        stdout = io.StringIO()
        stderr = io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            exit_code = main(["check", str(self.spec)])

        self.assertEqual(exit_code, 0)
        self.assertIn("target_reached: yes", stdout.getvalue())

    def test_view_command_prints_text_view(self) -> None:
        stdout = io.StringIO()
        stderr = io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            exit_code = main(["view", str(self.spec), "object"])

        self.assertEqual(exit_code, 0)
        self.assertIn("object view:", stdout.getvalue())

    def test_view_trace_command_prints_debug_layout(self) -> None:
        stdout = io.StringIO()
        stderr = io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            exit_code = main(["view", str(self.spec), "trace"])

        self.assertEqual(exit_code, 0)
        text = stdout.getvalue()
        self.assertIn("trace view:", text)
        self.assertIn("columns:", text)
        self.assertIn("StartupTimeline.Event::Setup", text)

    def test_render_command_writes_ascii_dot(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "object.gv"
            exit_code = main(
                ["render", str(self.spec), "object", "--format", "dot", "-o", str(output)]
            )

            self.assertEqual(exit_code, 0)
            data = output.read_bytes()
            self.assertTrue(data.startswith(b"digraph ObjectView"))
            data.decode("ascii")

    def test_work_dir_keeps_derivation_intermediates(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp) / "build"
            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exit_code = main(
                    [str(self.spec), "--derive", "--strict", "--work-dir", str(work)]
                )

            self.assertEqual(exit_code, 0)
            stem = self.spec.stem
            self.assertTrue((work / f"{stem}.ast.json").is_file())
            self.assertTrue((work / f"{stem}.model.json").is_file())
            self.assertTrue((work / f"{stem}.derive.json").is_file())

    def test_work_dir_keeps_render_intermediates(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp) / "build"
            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exit_code = main(
                    [
                        "render",
                        str(self.spec),
                        "object",
                        "--format",
                        "dot",
                        "--work-dir",
                        str(work),
                    ]
                )

            self.assertEqual(exit_code, 0)
            stem = self.spec.stem
            self.assertTrue((work / f"{stem}.ast.json").is_file())
            self.assertTrue((work / f"{stem}.model.json").is_file())
            self.assertTrue((work / f"{stem}.object.view.json").is_file())
            self.assertTrue((work / f"{stem}.object.gv").is_file())
            self.assertTrue(stdout.getvalue().startswith("digraph ObjectView"))

    def test_render_command_honors_explicit_dot_format_for_timeline(self) -> None:
        stdout = io.StringIO()
        stderr = io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            exit_code = main(["render", str(self.spec), "timeline", "--format", "dot"])

        self.assertEqual(exit_code, 0)
        self.assertTrue(stdout.getvalue().startswith("digraph TimelineView"))

    def test_render_trace_command_writes_svg(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "trace.svg"
            exit_code = main(
                ["render", str(self.spec), "trace", "--format", "svg", "-o", str(output)]
            )

            self.assertEqual(exit_code, 0)
            text = output.read_text(encoding="utf-8")
            self.assertTrue(text.startswith('<?xml version="1.0" encoding="UTF-8"?>'))
            self.assertIn("StartupTimeline.Setup", text)

    def test_render_trace_command_accepts_annotations(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            annotations = Path(tmp) / "notes.json"
            annotations.write_text(
                '{"events": {"Vm.Event::Setup": "build early mappings"}}\n',
                encoding="utf-8",
            )
            output = Path(tmp) / "trace.svg"
            exit_code = main(
                [
                    "render",
                    str(self.spec),
                    "trace",
                    "--format",
                    "svg",
                    "--annotations",
                    str(annotations),
                    "-o",
                    str(output),
                ]
            )

            self.assertEqual(exit_code, 0)
            self.assertIn("build early mappings", output.read_text(encoding="utf-8"))

    def test_legacy_graph_timeline_keeps_svg_behavior(self) -> None:
        stdout = io.StringIO()
        stderr = io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            exit_code = main([str(self.spec), "--graph", "timeline"])

        self.assertEqual(exit_code, 0)
        self.assertTrue(stdout.getvalue().startswith("<?xml"))

    def test_missing_input_returns_usage_error_code(self) -> None:
        stdout = io.StringIO()
        stderr = io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            exit_code = main(["parse", str(self.spec.with_name("missing.spec"))])

        self.assertEqual(exit_code, 2)
        self.assertIn("error: cannot read", stderr.getvalue())


if __name__ == "__main__":
    unittest.main()
