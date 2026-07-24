from __future__ import annotations

from copy import deepcopy
import contextlib
import io
from pathlib import Path
import subprocess
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
        self.assertEqual(
            animation["initial_frame"]["nodes"],
            [
                {
                    "id": "Human",
                    "parent": None,
                    "kind": "external",
                    "state": None,
                    "structural": False,
                    "first_seen": 0,
                }
            ],
        )
        self.assertEqual(len(animation["frames"]), 4)
        self.assertEqual(
            [(node["id"], node["state"]) for node in animation["frames"][0]["nodes"]],
            [("Human", None), ("Root", "Ready")],
        )
        self.assertNotIn("before_snapshot", animation["steps"][0])
        self.assertNotIn("after_snapshot", animation["steps"][0])
        self.assertEqual(
            animation["steps"][0]["response"],
            {"before_state": "Base", "after_state": "Ready"},
        )
        self.assertEqual(
            animation["steps"][1]["response"],
            {"before_state": None, "after_state": None},
        )

    def test_frames_reveal_targets_and_keep_first_seen_sibling_order(self) -> None:
        animation = build_animation(self.model, self.view)
        self.assertEqual(animation["initial_frame"]["sibling_order"]["$root"], ["Human"])
        self.assertEqual(animation["frames"][0]["sibling_order"]["$root"], ["Human", "Root"])
        child_frame = animation["frames"][1]
        self.assertEqual(child_frame["sibling_order"]["$root"], ["Human", "Root"])
        self.assertEqual(child_frame["sibling_order"]["Root"], ["Child"])
        self.assertEqual(animation["frames"][2]["sibling_order"]["Root"], ["Child", "Async"])
        self.assertEqual(
            animation["frames"][3]["sibling_order"]["Root"], ["Child", "Async", "Sink"]
        )
        self.assertEqual(animation["frames"][1]["nodes"][2]["state"], "Base")

    def test_same_parent_signal_does_not_reorder_existing_siblings(self) -> None:
        view = deepcopy(self.view)
        view["signals"][1]["source"] = "Child"
        view["signals"][1]["target"] = "Async"
        animation = build_animation(self.model, view)
        self.assertEqual(animation["frames"][1]["sibling_order"]["Root"], ["Child", "Async"])

        later_view = deepcopy(self.view)
        later_view["signals"][1]["source"] = "Child"
        later_view["signals"][1]["target"] = "Async"
        later_view["signals"][2]["target"] = "Sink"
        later_view["signals"][3]["source"] = "Child"
        later_view["signals"][3]["target"] = "Async"
        later = build_animation(self.model, later_view)
        self.assertEqual(
            later["frames"][3]["sibling_order"]["Root"],
            ["Child", "Async", "Sink"],
        )

    def test_cross_branch_signal_keeps_first_seen_ancestor_order(self) -> None:
        model = deepcopy(self.model)
        view = deepcopy(self.view)
        model["model"]["systems"]["ChildLeaf"] = {
            **deepcopy(model["model"]["systems"]["Child"]),
            "parent": "Child",
        }
        model["model"]["systems"]["AsyncLeaf"] = {
            **deepcopy(model["model"]["systems"]["Async"]),
            "parent": "Async",
        }
        for snapshot in [
            view["initial_snapshot"],
            *(signal["before_snapshot"] for signal in view["signals"]),
            *(signal["after_snapshot"] for signal in view["signals"]),
        ]:
            snapshot["states"]["ChildLeaf"] = snapshot["states"]["Child"]
            snapshot["states"]["AsyncLeaf"] = snapshot["states"]["Async"]
        view["signals"][1]["source"] = "ChildLeaf"
        view["signals"][1]["target"] = "AsyncLeaf"
        animation = build_animation(model, view)
        frame = animation["frames"][1]
        self.assertEqual(frame["sibling_order"]["Root"], ["Child", "Async"])
        self.assertEqual(frame["sibling_order"]["Child"], ["ChildLeaf"])
        self.assertEqual(frame["sibling_order"]["Async"], ["AsyncLeaf"])

    def test_self_and_ancestor_signals_keep_first_seen_order(self) -> None:
        animation = build_animation(self.model, self.view)
        self.assertEqual(animation["frames"][2]["sibling_order"]["Root"], ["Child", "Async"])

        descendant_view = deepcopy(self.view)
        descendant_view["signals"][2]["source"] = "Async"
        descendant_view["signals"][2]["target"] = "Root"
        descendant = build_animation(self.model, descendant_view)
        self.assertEqual(descendant["frames"][2]["sibling_order"]["Root"], ["Child", "Async"])

        self_view = deepcopy(self.view)
        self_view["signals"][3]["source"] = "Sink"
        self_view["signals"][3]["target"] = "Sink"
        self_signal = build_animation(self.model, self_view)
        self.assertEqual(
            self_signal["frames"][3]["sibling_order"]["Root"],
            ["Child", "Async", "Sink"],
        )

    def test_four_levels_use_reveal_order_independent_of_signal_direction(self) -> None:
        model = deepcopy(self.model)
        view = deepcopy(self.view)
        parents = {
            "Level2First": "Root",
            "Level2Second": "Root",
            "Level3First": "Level2First",
            "Level3Second": "Level2First",
            "Level4First": "Level3First",
            "Level4Second": "Level3First",
        }
        for name, parent in parents.items():
            model["model"]["systems"][name] = {
                **deepcopy(model["model"]["systems"]["Child"]),
                "parent": parent,
            }
        for snapshot in [
            view["initial_snapshot"],
            *(signal["before_snapshot"] for signal in view["signals"]),
            *(signal["after_snapshot"] for signal in view["signals"]),
        ]:
            for name in parents:
                snapshot["states"][name] = "Base"
        view["signals"][1]["source"] = "Level4First"
        view["signals"][1]["target"] = "Level4Second"
        view["signals"][2]["source"] = "Level3Second"
        view["signals"][2]["target"] = "Level2Second"

        animation = build_animation(model, view)
        frame = animation["frames"][2]
        self.assertEqual(frame["sibling_order"]["$root"], ["Human", "Root"])
        self.assertEqual(
            frame["sibling_order"]["Root"], ["Level2First", "Level2Second"]
        )
        self.assertEqual(
            frame["sibling_order"]["Level2First"], ["Level3First", "Level3Second"]
        )
        self.assertEqual(
            frame["sibling_order"]["Level3First"], ["Level4First", "Level4Second"]
        )
        for siblings in frame["sibling_order"].values():
            seen = {
                node["id"]: node["first_seen"]
                for node in frame["nodes"]
                if node["id"] in siblings
            }
            self.assertEqual(siblings, sorted(siblings, key=seen.__getitem__))
        self.assertEqual(animation, build_animation(model, view))

    def test_initial_model_source_has_stateless_ancestors_and_real_source_state(self) -> None:
        view = deepcopy(self.view)
        view["root_request"]["source"] = "Child"
        view["signals"][0]["source"] = "Child"
        animation = build_animation(self.model, view)
        self.assertEqual(
            [
                (node["id"], node["state"], node["structural"])
                for node in animation["initial_frame"]["nodes"]
            ],
            [("Root", None, True), ("Child", "Base", False)],
        )

    def test_stateless_system_uses_null_state_without_becoming_structural(self) -> None:
        model = deepcopy(self.model)
        view = deepcopy(self.view)
        model["model"]["systems"]["Child"]["states"] = {}
        model["model"]["systems"]["Child"]["initial_state"] = None
        for snapshot in [
            view["initial_snapshot"],
            *(signal["before_snapshot"] for signal in view["signals"]),
            *(signal["after_snapshot"] for signal in view["signals"]),
        ]:
            snapshot["states"]["Child"] = None
        animation = build_animation(model, view)
        child_node = next(
            node for node in animation["frames"][1]["nodes"] if node["id"] == "Child"
        )
        self.assertIsNone(child_node["state"])
        self.assertFalse(child_node["structural"])

    def test_stateless_transition_preserves_real_null_before_and_after_state(self) -> None:
        model = deepcopy(self.model)
        view = deepcopy(self.view)
        model["model"]["systems"]["Root"]["states"] = {}
        model["model"]["systems"]["Root"]["initial_state"] = None
        for snapshot in [
            view["initial_snapshot"],
            *(signal["before_snapshot"] for signal in view["signals"]),
            *(signal["after_snapshot"] for signal in view["signals"]),
        ]:
            snapshot["states"]["Root"] = None
        animation = build_animation(model, view)
        self.assertEqual(
            animation["steps"][0]["response"],
            {"before_state": None, "after_state": None},
        )
        root_node = next(
            node for node in animation["frames"][0]["nodes"] if node["id"] == "Root"
        )
        self.assertIsNone(root_node["state"])
        self.assertFalse(root_node["structural"])

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

    def test_cli_rejects_schema_version_and_producer_mismatch_without_output(self) -> None:
        model_path = self.root / "model.json"
        view_path = self.root / "view.json"
        cases = (
            ("schema", "model", "schema", "other.model", "model protocol mismatch"),
            ("version", "view", "version", 3, "view protocol mismatch"),
            ("producer", "view", "producer", "tools", "view protocol mismatch"),
        )
        for label, owner, field, value, message in cases:
            with self.subTest(label=label):
                model = deepcopy(self.model)
                view = deepcopy(self.view)
                (model if owner == "model" else view)[field] = value
                output = self.root / f"trace-{label}.html"
                write_json(model_path, model)
                write_json(view_path, view)
                with contextlib.redirect_stderr(io.StringIO()) as stderr:
                    self.assertEqual(
                        animate_main([str(model_path), str(view_path), "-o", str(output)]), 2
                    )
                self.assertIn(message, stderr.getvalue())
                self.assertFalse(output.exists())

    def test_driver_html_output_composes_with_text_snapshot_scenario_and_work_dir(self) -> None:
        scenario = self.root / "scenario.json"
        scenario.write_text("{}\n", encoding="utf-8")
        text = self.root / "trace.txt"
        html = self.root / "trace.html"
        snapshot = self.root / "snapshot.json"
        work = self.root / "combined-work"
        with contextlib.redirect_stdout(io.StringIO()):
            exit_code = driver_main(
                [
                    str(PIPELINE),
                    "--signal",
                    "Root.Start",
                    "--scenario",
                    str(scenario),
                    "--max-depth",
                    "all",
                    "--max-breadth",
                    "all",
                    "--work-dir",
                    str(work),
                    "--snapshot-out",
                    str(snapshot),
                    "-o",
                    str(text),
                    "--html-out",
                    str(html),
                ]
            )
        self.assertEqual(exit_code, 0)
        self.assertIn("verdict: complete", text.read_text(encoding="utf-8"))
        self.assertTrue(snapshot.is_file())
        self.assertTrue(html.read_text(encoding="utf-8").startswith("<!doctype html>"))
        self.assertTrue((work / "model.json").is_file())
        self.assertTrue((work / "view.json").is_file())

    def test_driver_html_output_preserves_check_one_and_animation_error_becomes_two(self) -> None:
        bounded_html = self.root / "bounded.html"
        with contextlib.redirect_stdout(io.StringIO()):
            bounded_exit = driver_main(
                [
                    str(PIPELINE),
                    "--signal",
                    "Root.Start",
                    "--max-depth",
                    "0",
                    "--html-out",
                    str(bounded_html),
                ]
            )
        self.assertEqual(bounded_exit, 1)
        self.assertTrue(bounded_html.is_file())

        output_directory = self.root / "not-a-file"
        output_directory.mkdir()
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(
            io.StringIO()
        ) as stderr:
            animation_exit = driver_main(
                [
                    str(PIPELINE),
                    "--signal",
                    "Root.Start",
                    "--html-out",
                    str(output_directory),
                ]
            )
        self.assertEqual(animation_exit, 2)
        self.assertIn("lkm-animate:", stderr.getvalue())
        self.assertTrue(output_directory.is_dir())

    def test_shortcut_passes_html_output_from_another_working_directory(self) -> None:
        scenario = self.root / "scenario.json"
        scenario.write_text("{}\n", encoding="utf-8")
        html = self.root / "shortcut.html"
        result = subprocess.run(
            [
                str(TOOLS2 / "bin" / "pyveri"),
                "-t",
                "Root.Start",
                "-f",
                str(PIPELINE),
                "-s",
                str(scenario),
                "--html-out",
                str(html),
            ],
            cwd=self.root,
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("verdict: complete", result.stdout)
        self.assertIn("lkm.spec.signal-animation", html.read_text(encoding="utf-8"))

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
        unknown_handler = deepcopy(self.view)
        unknown_handler["signals"][0]["handler"]["kind"] = "Effect"
        cases.append(("handler", self.model, unknown_handler, "unknown structure"))
        unknown_outcome = deepcopy(self.view)
        unknown_outcome["signals"][0]["outcome"] = "pending"
        cases.append(("outcome", self.model, unknown_outcome, "not an animation v1 outcome"))
        damaged_snapshot = deepcopy(self.view)
        damaged_snapshot["signals"][0]["before_snapshot"]["facts"] = {}
        cases.append(("snapshot", self.model, damaged_snapshot, "facts must be a string list"))
        for label, model, view, message in cases:
            with self.subTest(label=label), self.assertRaisesRegex(ProtocolError, message):
                build_animation(model, view)

    def test_all_animation_outcomes_and_repeated_generation_are_deterministic(self) -> None:
        for outcome in ("completed", "rejected", "failed", "truncated", "stopped"):
            with self.subTest(outcome=outcome):
                view = deepcopy(self.view)
                view["signals"][0]["outcome"] = outcome
                view["signals"][0]["reason"] = None if outcome == "completed" else "fixture"
                first = build_animation(self.model, view)
                second = build_animation(self.model, view)
                self.assertEqual(first, second)
                self.assertEqual(first["steps"][0]["outcome"], outcome)
                self.assertEqual(render_html(first), render_html(second))


if __name__ == "__main__":
    unittest.main()
