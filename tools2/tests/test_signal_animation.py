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
EMITS_NESTED = TOOLS2 / "tests" / "fixtures" / "animation-emits-nested.spec"


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

    @staticmethod
    def _snapshots(view: dict) -> list[dict]:
        snapshots = [view["initial_snapshot"]]
        for signal in view["signals"]:
            snapshots.extend((signal["before_snapshot"], signal["after_snapshot"]))
        for event in view["events"]:
            if isinstance(event.get("before"), dict):
                snapshots.append(event["before"])
            if isinstance(event.get("after"), dict):
                snapshots.append(event["after"])
        boundary = view.get("boundary")
        if isinstance(boundary, dict) and isinstance(boundary.get("snapshot"), dict):
            snapshots.append(view["boundary"]["snapshot"])
        return snapshots

    @staticmethod
    def _signal_event(view: dict, signal_id: str, kind: str) -> dict:
        return next(
            event
            for event in view["events"]
            if event["kind"] == kind and event.get("signal_id") == signal_id
        )

    def _set_endpoints(
        self, view: dict, signal_index: int, *, source: str | None = None, target: str | None = None
    ) -> None:
        signal = view["signals"][signal_index]
        sent = self._signal_event(view, signal["id"], "signal_sent")
        if source is not None:
            signal["source"] = source
            sent["source"] = source
        if target is not None:
            signal["target"] = target
            sent["target"] = target
            received = next(
                (
                    event
                    for event in view["events"]
                    if event["kind"] == "signal_received"
                    and event.get("signal_id") == signal["id"]
                ),
                None,
            )
            if received is not None:
                received["target"] = target

    @staticmethod
    def _resequence(view: dict) -> None:
        for sequence, event in enumerate(view["events"], start=1):
            event["sequence"] = sequence

    def _single_signal_view(self, outcome: str) -> dict:
        view = deepcopy(self.view)
        signal = view["signals"][0]
        signal["outcome"] = outcome
        signal["reason"] = None if outcome == "completed" else "fixture"
        if outcome != "completed":
            signal["after_snapshot"] = deepcopy(signal["before_snapshot"])
        if outcome in {"rejected", "truncated"}:
            signal["handler"] = None
        sent = deepcopy(self._signal_event(view, signal["id"], "signal_sent"))
        received = deepcopy(self._signal_event(view, signal["id"], "signal_received"))
        if outcome == "completed":
            terminal = deepcopy(self._signal_event(view, signal["id"], "response_completed"))
            terminal["before"] = deepcopy(signal["before_snapshot"])
            terminal["after"] = deepcopy(signal["after_snapshot"])
            events = [sent, received, terminal]
        else:
            terminal_kinds = {
                "rejected": "signal_rejected",
                "failed": "signal_failed",
                "truncated": "signal_truncated",
                "stopped": "response_stopped",
            }
            terminal = {
                "kind": terminal_kinds[outcome],
                "signal_id": signal["id"],
            }
            if outcome != "truncated":
                terminal["reason"] = signal["reason"]
            events = [sent, terminal] if outcome == "truncated" else [sent, received, terminal]
        view["signals"] = [signal]
        view["events"] = events
        view["initial_snapshot"] = deepcopy(signal["before_snapshot"])
        view["boundary"] = None
        self._resequence(view)
        return view

    def test_animation_v3_projects_request_feedback_and_settle_moments(self) -> None:
        animation = build_animation(self.model, self.view)
        self.assertEqual(
            (animation["schema"], animation["version"], animation["producer"]),
            (ANIMATION_SCHEMA, ANIMATION_VERSION, "tools2"),
        )
        self.assertEqual(animation["inputs"]["model_fingerprint"], self.model["model_fingerprint"])
        self.assertEqual(animation["trace"]["total_signals"], 4)
        self.assertEqual(animation["trace"]["total_moments"], 8)
        self.assertEqual(
            [
                (moment["id"], moment["kind"], moment["event_sequence"])
                for moment in animation["moments"]
            ],
            [
                ("sig-0001:request", "request", 2),
                ("sig-0002:request", "request", 6),
                ("sig-0002:feedback", "feedback", 8),
                ("sig-0001:feedback", "feedback", 11),
                ("sig-0003:request", "request", 17),
                ("sig-0003:settle", "settle", 21),
                ("sig-0004:request", "request", 23),
                ("sig-0004:settle", "settle", 25),
            ],
        )
        self.assertEqual(animation["initial_frame"]["nodes"], [])
        self.assertEqual(len(animation["frames"]), 8)
        self.assertEqual(
            [(node["id"], node["state"]) for node in animation["frames"][0]["nodes"]],
            [("Human", None), ("Root", "Base")],
        )
        self.assertEqual(
            next(node for node in animation["frames"][3]["nodes"] if node["id"] == "Root")[
                "state"
            ],
            "Ready",
        )
        self.assertNotIn("before_snapshot", animation["moments"][0])
        self.assertNotIn("after_snapshot", animation["moments"][0])
        self.assertEqual(
            animation["moments"][0]["response"],
            {"before_state": "Base", "after_state": "Ready"},
        )
        self.assertEqual(
            animation["moments"][1]["response"],
            {"before_state": None, "after_state": None},
        )
        self.assertEqual(
            animation["moments"][0]["transfer"], {"from": "Human", "to": "Root"}
        )
        self.assertEqual(
            animation["moments"][3]["transfer"], {"from": "Root", "to": "Human"}
        )
        self.assertIsNone(animation["moments"][5]["transfer"])

    def test_frames_reveal_targets_and_keep_first_seen_sibling_order(self) -> None:
        animation = build_animation(self.model, self.view)
        self.assertEqual(animation["initial_frame"]["sibling_order"], {})
        self.assertEqual(animation["frames"][0]["sibling_order"]["$root"], ["Human", "Root"])
        child_frame = animation["frames"][1]
        self.assertEqual(child_frame["sibling_order"]["$root"], ["Human", "Root"])
        self.assertEqual(child_frame["sibling_order"]["Root"], ["Child"])
        self.assertEqual(animation["frames"][4]["sibling_order"]["Root"], ["Child", "Async"])
        self.assertEqual(
            animation["frames"][6]["sibling_order"]["Root"], ["Child", "Async", "Sink"]
        )
        self.assertEqual(animation["frames"][1]["nodes"][2]["state"], "Base")

    def test_same_parent_signal_does_not_reorder_existing_siblings(self) -> None:
        view = deepcopy(self.view)
        self._set_endpoints(view, 1, source="Child", target="Async")
        animation = build_animation(self.model, view)
        self.assertEqual(animation["frames"][1]["sibling_order"]["Root"], ["Child", "Async"])

        later_view = deepcopy(self.view)
        self._set_endpoints(later_view, 1, source="Child", target="Async")
        self._set_endpoints(later_view, 2, target="Sink")
        self._set_endpoints(later_view, 3, source="Child", target="Async")
        later = build_animation(self.model, later_view)
        self.assertEqual(
            later["frames"][-1]["sibling_order"]["Root"],
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
        for snapshot in self._snapshots(view):
            snapshot["states"]["ChildLeaf"] = snapshot["states"]["Child"]
            snapshot["states"]["AsyncLeaf"] = snapshot["states"]["Async"]
        self._set_endpoints(view, 1, source="ChildLeaf", target="AsyncLeaf")
        animation = build_animation(model, view)
        frame = animation["frames"][1]
        self.assertEqual(frame["sibling_order"]["Root"], ["Child", "Async"])
        self.assertEqual(frame["sibling_order"]["Child"], ["ChildLeaf"])
        self.assertEqual(frame["sibling_order"]["Async"], ["AsyncLeaf"])

    def test_self_and_ancestor_signals_keep_first_seen_order(self) -> None:
        animation = build_animation(self.model, self.view)
        self.assertEqual(animation["frames"][4]["sibling_order"]["Root"], ["Child", "Async"])

        descendant_view = deepcopy(self.view)
        self._set_endpoints(descendant_view, 2, source="Async", target="Root")
        descendant = build_animation(self.model, descendant_view)
        self.assertEqual(descendant["frames"][4]["sibling_order"]["Root"], ["Child", "Async"])

        self_view = deepcopy(self.view)
        self._set_endpoints(self_view, 3, source="Sink", target="Sink")
        self_signal = build_animation(self.model, self_view)
        self.assertEqual(
            self_signal["frames"][6]["sibling_order"]["Root"],
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
        for snapshot in self._snapshots(view):
            for name in parents:
                snapshot["states"][name] = "Base"
        self._set_endpoints(view, 1, source="Level4First", target="Level4Second")
        self._set_endpoints(view, 2, source="Level3Second", target="Level2Second")

        animation = build_animation(model, view)
        frame = animation["frames"][4]
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

    def test_first_request_has_stateless_ancestors_and_real_source_state(self) -> None:
        view = deepcopy(self.view)
        view["root_request"]["source"] = "Child"
        view["root_request"]["target"] = "Child"
        self._set_endpoints(view, 0, source="Child", target="Child")
        animation = build_animation(self.model, view)
        self.assertEqual(
            [
                (node["id"], node["state"], node["structural"])
                for node in animation["frames"][0]["nodes"]
            ],
            [("Root", None, True), ("Child", "Base", False)],
        )

    def test_stateless_system_uses_null_state_without_becoming_structural(self) -> None:
        model = deepcopy(self.model)
        view = deepcopy(self.view)
        model["model"]["systems"]["Child"]["states"] = {}
        model["model"]["systems"]["Child"]["initial_state"] = None
        for snapshot in self._snapshots(view):
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
        for snapshot in self._snapshots(view):
            snapshot["states"]["Root"] = None
        animation = build_animation(model, view)
        self.assertEqual(
            animation["moments"][0]["response"],
            {"before_state": None, "after_state": None},
        )
        root_node = next(
            node for node in animation["frames"][3]["nodes"] if node["id"] == "Root"
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
        cases.append(("outcome", self.model, unknown_outcome, "not an animation v3 outcome"))
        damaged_snapshot = deepcopy(self.view)
        damaged_snapshot["signals"][0]["before_snapshot"]["facts"] = {}
        cases.append(("snapshot", self.model, damaged_snapshot, "facts must be a string list"))
        for label, model, view, message in cases:
            with self.subTest(label=label), self.assertRaisesRegex(ProtocolError, message):
                build_animation(model, view)

    def test_event_protocol_rejects_missing_duplicate_out_of_order_and_snapshot_mismatch(self) -> None:
        cases: list[tuple[str, dict, str]] = []

        missing = deepcopy(self.view)
        missing["events"] = [
            event
            for event in missing["events"]
            if not (
                event["kind"] == "response_completed"
                and event.get("signal_id") == "sig-0004"
            )
        ]
        self._resequence(missing)
        cases.append(("missing", missing, "no terminal event"))

        duplicate = deepcopy(self.view)
        sent = deepcopy(self._signal_event(duplicate, "sig-0001", "signal_sent"))
        duplicate["events"].insert(1, sent)
        self._resequence(duplicate)
        cases.append(("duplicate", duplicate, "duplicate signal_sent"))

        out_of_order = deepcopy(self.view)
        first = next(i for i, event in enumerate(out_of_order["events"]) if event["kind"] == "signal_sent")
        second = next(
            i
            for i, event in enumerate(out_of_order["events"])
            if event["kind"] == "signal_sent" and event.get("signal_id") == "sig-0002"
        )
        out_of_order["events"][first], out_of_order["events"][second] = (
            out_of_order["events"][second],
            out_of_order["events"][first],
        )
        self._resequence(out_of_order)
        cases.append(("order", out_of_order, "creation order"))

        snapshot = deepcopy(self.view)
        snapshot["signals"][0]["before_snapshot"]["states"]["Root"] = "Ready"
        cases.append(("snapshot", snapshot, "causal replay state"))

        outcome = deepcopy(self.view)
        outcome["signals"][0]["outcome"] = "failed"
        outcome["signals"][0]["reason"] = "fixture"
        cases.append(("outcome", outcome, "does not match Signal"))

        for label, view, message in cases:
            with self.subTest(label=label), self.assertRaisesRegex(ProtocolError, message):
                build_animation(self.model, view)

    def test_all_animation_outcomes_and_repeated_generation_are_deterministic(self) -> None:
        for outcome in ("completed", "rejected", "failed", "truncated", "stopped"):
            with self.subTest(outcome=outcome):
                view = self._single_signal_view(outcome)
                first = build_animation(self.model, view)
                second = build_animation(self.model, view)
                self.assertEqual(first, second)
                if outcome == "truncated":
                    self.assertEqual([moment["kind"] for moment in first["moments"]], ["terminal"])
                    self.assertEqual(first["frames"][0]["nodes"], [])
                else:
                    self.assertEqual(first["moments"][0]["kind"], "request")
                    self.assertEqual(
                        first["moments"][-1]["kind"],
                        "terminal" if outcome == "stopped" else "feedback",
                    )
                self.assertEqual(first["moments"][-1]["outcome"], outcome)
                if outcome not in {"completed", "truncated"}:
                    self.assertEqual(
                        first["frames"][0]["nodes"], first["frames"][-1]["nodes"]
                    )
                self.assertEqual(render_html(first), render_html(second))

    def test_unreceived_stopped_signal_has_only_terminal_and_reveals_nothing(self) -> None:
        view = self._single_signal_view("stopped")
        signal = view["signals"][0]
        view["events"] = [
            deepcopy(self._signal_event(self.view, signal["id"], "signal_sent")),
            {
                "kind": "signal_stopped",
                "signal_id": signal["id"],
                "reason": signal["reason"],
            },
        ]
        self._resequence(view)
        animation = build_animation(self.model, view)
        self.assertEqual([moment["kind"] for moment in animation["moments"]], ["terminal"])
        self.assertEqual(animation["frames"][0]["nodes"], [])

    def test_emits_settle_follows_nested_drives_feedback(self) -> None:
        work = self.root / "emits-nested-work"
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(
                driver_main(
                    [
                        str(EMITS_NESTED),
                        "--signal",
                        "Root.Start",
                        "--max-depth",
                        "all",
                        "--max-breadth",
                        "all",
                        "--work-dir",
                        str(work),
                    ]
                ),
                0,
            )
        animation = build_animation(read_json(work / "model.json"), read_json(work / "view.json"))
        self.assertEqual(
            [
                (moment["source"], moment["target"], moment["kind"])
                for moment in animation["moments"]
            ],
            [
                ("Human", "Root", "request"),
                ("Human", "Root", "feedback"),
                ("Root", "Worker", "request"),
                ("Worker", "Child", "request"),
                ("Worker", "Child", "feedback"),
                ("Root", "Worker", "settle"),
            ],
        )
        self.assertEqual(
            next(
                node["state"]
                for node in animation["frames"][-1]["nodes"]
                if node["id"] == "Worker"
            ),
            "Ready",
        )

    def test_kernel_enable_boundary_has_15_signals_and_30_causal_moments(self) -> None:
        work = self.root / "kernel-boundary-work"
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(
                driver_main(
                    [
                        str(ROOT / "spec" / "model" / "main.spec"),
                        "--until",
                        "Kernel.Enable",
                        "--max-depth",
                        "all",
                        "--max-breadth",
                        "all",
                        "--work-dir",
                        str(work),
                    ]
                ),
                0,
            )
        model = read_json(work / "model.json")
        derivation = read_json(work / "derive.json")
        view = read_json(work / "view.json")
        self.assertEqual(
            {
                model["model_fingerprint"],
                derivation["model_fingerprint"],
                view["model_fingerprint"],
            },
            {derivation["model_fingerprint"]},
        )
        animation = build_animation(model, view)
        self.assertEqual(
            animation["inputs"]["model_fingerprint"],
            derivation["model_fingerprint"],
        )
        self.assertEqual(animation["trace"]["total_signals"], 15)
        self.assertEqual(animation["trace"]["total_moments"], 30)
        self.assertEqual(
            {kind: sum(moment["kind"] == kind for moment in animation["moments"])
             for kind in ("request", "feedback", "settle")},
            {"request": 15, "feedback": 12, "settle": 3},
        )
        self.assertEqual(
            [moment["id"] for moment in animation["moments"] if moment["kind"] == "feedback"],
            [
                f"sig-{index:04d}:feedback"
                for index in (2, 3, 4, 1, 6, 7, 9, 10, 8, 5, 15, 14)
            ],
        )
        self.assertEqual(
            [moment["id"] for moment in animation["moments"] if moment["kind"] == "settle"],
            [f"sig-{index:04d}:settle" for index in range(11, 14)],
        )
        self.assertEqual(
            [
                (
                    moment["signal_id"],
                    moment["source"],
                    moment["target"],
                    moment["signal"],
                )
                for moment in animation["moments"]
                if moment["kind"] == "request"
            ],
            [
                (item["id"], item["source"], item["target"], item["name"])
                for item in derivation["signals"]
            ],
        )
        self.assertEqual(animation["trace"]["boundary"]["normalized_signal"], "Kernel.Enable")
        self.assertFalse(
            any(
                moment["target"] == "Kernel" and moment["signal"] == "Enable"
                for moment in animation["moments"]
            )
        )

        frames = {
            moment["id"]: frame
            for moment, frame in zip(animation["moments"], animation["frames"], strict=True)
        }

        def state(moment_id: str, system: str) -> str | None:
            return next(
                node["state"] for node in frames[moment_id]["nodes"] if node["id"] == system
            )

        self.assertEqual(state("sig-0004:feedback", "Computer"), "Base")
        self.assertEqual(state("sig-0001:feedback", "Computer"), "Prepared")
        self.assertEqual(state("sig-0010:feedback", "Kernel"), "Prepared")
        self.assertEqual(state("sig-0008:feedback", "Kernel"), "Ready")
        self.assertEqual(state("sig-0005:feedback", "Computer"), "Ready")
        self.assertEqual(state("sig-0011:settle", "Computer"), "Online")
        self.assertEqual(state("sig-0012:request", "Computer"), "Online")
        computer_states = [
            next(
                (node["state"] for node in frame["nodes"] if node["id"] == "Computer"),
                None,
            )
            for frame in animation["frames"]
        ]
        stable_states = [
            state
            for index, state in enumerate(computer_states)
            if state is not None and (index == 0 or state != computer_states[index - 1])
        ]
        self.assertEqual(stable_states, ["Base", "Prepared", "Ready", "Online"])

    def test_boot_init_setup_boundary_has_52_signals_and_104_causal_moments(self) -> None:
        work = self.root / "boot-init-setup-boundary-work"
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(
                driver_main(
                    [
                        str(ROOT / "spec" / "model" / "main.spec"),
                        "--until",
                        "BootInitFlow.Setup",
                        "--max-depth",
                        "all",
                        "--max-breadth",
                        "all",
                        "--work-dir",
                        str(work),
                    ]
                ),
                0,
            )
        model = read_json(work / "model.json")
        derivation = read_json(work / "derive.json")
        view = read_json(work / "view.json")
        self.assertEqual(
            {
                model["model_fingerprint"],
                derivation["model_fingerprint"],
                view["model_fingerprint"],
            },
            {"sha256:552e742222dc1c65610e023c52130fbb211d0b6f95878c2931e8b6958d2ce36d"},
        )
        animation = build_animation(model, view)
        self.assertEqual(animation["trace"]["total_signals"], 52)
        self.assertEqual(animation["trace"]["total_moments"], 104)
        self.assertEqual(
            {
                kind: sum(moment["kind"] == kind for moment in animation["moments"])
                for kind in ("request", "feedback", "settle", "terminal")
            },
            {"request": 52, "feedback": 48, "settle": 3, "terminal": 1},
        )
        self.assertEqual(
            [moment["id"] for moment in animation["moments"] if moment["kind"] == "settle"],
            [f"sig-{index:04d}:settle" for index in range(11, 14)],
        )
        self.assertEqual(
            [moment["id"] for moment in animation["moments"] if moment["kind"] == "terminal"],
            ["sig-0016:terminal"],
        )
        terminal = next(
            moment for moment in animation["moments"] if moment["kind"] == "terminal"
        )
        self.assertEqual(
            (
                terminal["signal_id"], terminal["source"], terminal["target"],
                terminal["signal"], terminal["outcome"], terminal["reason"],
            ),
            (
                "sig-0016", "OpenSBI", "Kernel", "Enable", "stopped",
                "until_signal_reached",
            ),
        )
        self.assertEqual(
            [
                (moment["signal_id"], moment["source"], moment["target"], moment["signal"])
                for moment in animation["moments"]
                if moment["kind"] == "request"
            ],
            [
                (item["id"], item["source"], item["target"], item["name"])
                for item in derivation["signals"]
            ],
        )
        moment_index = {
            moment["id"]: index for index, moment in enumerate(animation["moments"])
        }
        self.assertGreater(
            moment_index["sig-0020:feedback"],
            max(moment_index[f"sig-{index:04d}:feedback"] for index in range(21, 53)),
        )
        self.assertEqual(
            animation["trace"]["boundary"]["normalized_signal"],
            "BootInitFlow.Setup",
        )
        self.assertFalse(
            any(
                moment["target"] == "BootInitFlow" and moment["signal"] == "Setup"
                for moment in animation["moments"]
            )
        )
        feedback_index = moment_index["sig-0020:feedback"]
        feedback_frame = animation["frames"][feedback_index]
        self.assertEqual(
            next(
                node["state"]
                for node in feedback_frame["nodes"]
                if node["id"] == "BootInitFlow"
            ),
            "Prepared",
        )


if __name__ == "__main__":
    unittest.main()
