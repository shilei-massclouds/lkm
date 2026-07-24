from __future__ import annotations

from copy import deepcopy
import contextlib
import io
from pathlib import Path
import tempfile
import unittest

from animate_tool.__main__ import main as animate_main
from animate_tool.builder import build_animation
from animate_tool.html import render_html
from pyveri.__main__ import main as driver_main
from tools2_common import ANIMATION_SCHEMA, ANIMATION_VERSION, ProtocolError, read_json, write_json


ROOT = Path(__file__).resolve().parents[2]
TOOLS2 = ROOT / "tools2"
PIPELINE = TOOLS2 / "tests" / "fixtures" / "pipeline.spec"


class SignalAnimationTests(unittest.TestCase):
    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.work = self.root / "work"
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(
                driver_main(
                    [
                        str(PIPELINE),
                        "--signal",
                        "Root.Start",
                        "--max-depth",
                        "all",
                        "--max-breadth",
                        "all",
                        "--work-dir",
                        str(self.work),
                    ]
                ),
                0,
            )
        self.model = read_json(self.work / "model.json")
        self.view = read_json(self.work / "view.json")

    def test_animation_v1_projects_strict_trace_metadata(self) -> None:
        animation = build_animation(self.model, self.view)
        self.assertEqual(
            (animation["schema"], animation["version"], animation["producer"]),
            (ANIMATION_SCHEMA, ANIMATION_VERSION, "tools2"),
        )
        self.assertEqual(animation["inputs"]["model_fingerprint"], self.model["model_fingerprint"])
        self.assertEqual(animation["trace"]["total_steps"], 4)
        self.assertEqual(
            [
                (step["id"], step["source"], step["target"], step["handler"]["kind"])
                for step in animation["steps"]
            ],
            [
                ("sig-0001", "Human", "Root", "Transition"),
                ("sig-0002", "Root", "Child", "Action"),
                ("sig-0003", "Root", "Async", "Transition"),
                ("sig-0004", "Root", "Sink", "Action"),
            ],
        )
        self.assertEqual(animation["frames"], [])

    def test_html_is_self_contained_and_script_data_is_safely_encoded(self) -> None:
        animation = build_animation(self.model, self.view)
        animation["source"] = "fixture</script><!--trace"
        html = render_html(animation)
        self.assertTrue(html.startswith("<!doctype html>"))
        self.assertIn('id="lkm-signal-animation" type="application/json"', html)
        self.assertIn('id="lkm-signal-player"', html)
        self.assertIn('"schema":"lkm.spec.signal-animation"', html)
        self.assertIn("fixture\\u003c/script\\u003e\\u003c!--trace", html)
        self.assertNotIn("fixture</script>", html)
        self.assertNotIn('src="http', html)
        self.assertNotIn('href="http', html)

    def test_cli_rejects_protocol_identity_mismatch_without_output(self) -> None:
        model_path = self.root / "model.json"
        view_path = self.root / "view.json"
        output = self.root / "trace.html"
        broken = deepcopy(self.view)
        broken["version"] = 3
        write_json(model_path, self.model)
        write_json(view_path, broken)
        with contextlib.redirect_stderr(io.StringIO()) as stderr:
            self.assertEqual(
                animate_main([str(model_path), str(view_path), "-o", str(output)]), 2
            )
        self.assertIn("view protocol mismatch", stderr.getvalue())
        self.assertFalse(output.exists())

    def test_source_fingerprint_endpoint_and_parent_validation_are_strict(self) -> None:
        cases: list[tuple[str, dict, dict, str]] = []
        wrong_source = deepcopy(self.view)
        wrong_source["source"] = "other.spec"
        cases.append(("source", self.model, wrong_source, "source mismatch"))
        wrong_fingerprint = deepcopy(self.view)
        wrong_fingerprint["model_fingerprint"] = "sha256:other"
        cases.append(("fingerprint", self.model, wrong_fingerprint, "model_fingerprint mismatch"))
        missing_endpoint = deepcopy(self.view)
        missing_endpoint["signals"][0]["target"] = "Missing"
        cases.append(("endpoint", self.model, missing_endpoint, "unknown endpoint"))
        parent_cycle = deepcopy(self.model)
        parent_cycle["model"]["systems"]["Root"]["parent"] = "Child"
        cases.append(("parent", parent_cycle, self.view, "parent cycle"))
        for label, model, view, message in cases:
            with self.subTest(label=label), self.assertRaisesRegex(ProtocolError, message):
                build_animation(model, view)


if __name__ == "__main__":
    unittest.main()
