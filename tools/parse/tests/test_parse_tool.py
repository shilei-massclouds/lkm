from __future__ import annotations

import contextlib
import io
import tempfile
import unittest
from pathlib import Path

from common import AST_SCHEMA, AST_VERSION, read_json
from parse_tool.__main__ import main


class ParseToolTests(unittest.TestCase):
    def setUp(self) -> None:
        self.spec = (
            Path(__file__).resolve().parents[3]
            / "spec"
            / "entry-prelude-object-model.spec"
        )

    def test_parse_writes_ast_json(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "entry-prelude-object-model.ast.json"

            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exit_code = main([str(self.spec), "-o", str(output)])

            self.assertEqual(exit_code, 0)
            data = read_json(output)
            self.assertEqual(data["schema"], AST_SCHEMA)
            self.assertEqual(data["version"], AST_VERSION)
            document = data["document"]
            self.assertGreaterEqual(len(document["objects"]), 19)
            startup = next(
                item for item in document["objects"] if item["name"] == "StartupTimeline"
            )
            self.assertEqual(startup["kind"], "TimelineObject")

    def test_entry_spans_are_serialized(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "entry-prelude-object-model.ast.json"
            exit_code = main([str(self.spec), "-o", str(output)])

            self.assertEqual(exit_code, 0)
            data = read_json(output)
            kernel_image = next(
                item for item in data["document"]["objects"] if item["name"] == "KernelImage"
            )
            ready = next(state for state in kernel_image["states"] if state["name"] == "Ready")
            enable = next(event for event in ready["events"] if event["name"] == "Enable")
            entry = enable["depends_on"][0]["entries"][0]

            self.assertEqual(entry["text"], "EarlyVm.state == State::Online")
            self.assertEqual(entry["span"]["start_line"], 833)

    def test_output_parent_directory_is_created(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "nested" / "entry-prelude-object-model.ast.json"
            exit_code = main([str(self.spec), "-o", str(output)])

            self.assertEqual(exit_code, 0)
            self.assertTrue(output.exists())

    def test_missing_input_returns_usage_error_code(self) -> None:
        stdout = io.StringIO()
        stderr = io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            exit_code = main([str(self.spec.with_name("missing.spec")), "-o", "/tmp/out.json"])

        self.assertEqual(exit_code, 2)
        self.assertIn("error: cannot read", stderr.getvalue())

    def test_parse_tool_does_not_import_pyveri(self) -> None:
        source_root = Path(__file__).resolve().parents[1] / "src" / "parse_tool"

        for path in source_root.rglob("*.py"):
            text = path.read_text(encoding="utf-8")
            self.assertNotIn("from pyveri", text, str(path))
            self.assertNotIn("import pyveri", text, str(path))


if __name__ == "__main__":
    unittest.main()
