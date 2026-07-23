from __future__ import annotations

import contextlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import textwrap
import unittest
from unittest import mock

from check_tool.__main__ import main as check_main
from derive_tool.__main__ import main as derive_main
from derive_tool.engine import derive
from model_tool.__main__ import main as model_main
from parse_tool.__main__ import main as parse_main
from pyveri.__main__ import main as driver_main
from render_tool.__main__ import main as render_main
from render_tool.text import render_text
from tools2_common import (
    AST_SCHEMA,
    AST_VERSION,
    CHECK_SCHEMA,
    CHECK_VERSION,
    DERIVE_SCHEMA,
    DERIVE_VERSION,
    MODEL_SCHEMA,
    MODEL_VERSION,
    PRODUCER,
    SNAPSHOT_SCHEMA,
    SNAPSHOT_VERSION,
    VIEW_SCHEMA,
    VIEW_VERSION,
    read_json,
)
from view_tool.__main__ import main as view_main


ROOT = Path(__file__).resolve().parents[2]
TOOLS2 = ROOT / "tools2"
PIPELINE = TOOLS2 / "tests" / "fixtures" / "pipeline.spec"


class SignalPipelineTests(unittest.TestCase):
    def run_source(
        self,
        source: str,
        signal: str,
        *,
        max_depth: str = "3",
        max_breadth: str = "3",
        scenario: dict | None = None,
        until: str | None = None,
        source_name: str | None = None,
    ) -> tuple[dict, dict, str]:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        spec = root / "input.spec"
        ast = root / "ast.json"
        model = root / "model.json"
        derivation = root / "derive.json"
        checked = root / "check.json"
        view = root / "view.json"
        text = root / "trace.txt"
        spec.write_text(textwrap.dedent(source), encoding="utf-8")
        self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
        self.assertEqual(model_main([str(ast), "-o", str(model)]), 0)
        derive_args = [
            str(model),
            "--signal",
            signal,
            "--max-depth",
            max_depth,
            "--max-breadth",
            max_breadth,
            "-o",
            str(derivation),
        ]
        if scenario is not None:
            scenario_path = root / "scenario.json"
            scenario_path.write_text(json.dumps(scenario), encoding="utf-8")
            derive_args.extend(["--scenario", str(scenario_path)])
        if until is not None:
            derive_args.extend(["--until", until])
        if source_name is not None:
            derive_args.extend(["--source", source_name])
        self.assertEqual(derive_main(derive_args), 0)
        check_exit = check_main([str(derivation), "-o", str(checked)])
        self.assertIn(check_exit, {0, 1})
        self.assertEqual(view_main([str(derivation), "-o", str(view)]), 0)
        with mock.patch.dict(os.environ, {"VERBOSE": "0"}):
            self.assertEqual(render_main([str(view), "-o", str(text)]), 0)
        return read_json(derivation), read_json(checked), text.read_text(encoding="utf-8")

    def test_drives_and_emits_order(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp) / "work"
            stdout = io.StringIO()
            with contextlib.redirect_stdout(stdout), mock.patch.dict(
                os.environ, {"VERBOSE": "0"}
            ):
                exit_code = driver_main(
                    [str(PIPELINE), "--signal", "Root.Start", "--work-dir", str(work)]
                )
            self.assertEqual(exit_code, 0)
            for filename, schema, version in (
                ("ast.json", AST_SCHEMA, AST_VERSION),
                ("model.json", MODEL_SCHEMA, MODEL_VERSION),
                ("derive.json", DERIVE_SCHEMA, DERIVE_VERSION),
                ("check.json", CHECK_SCHEMA, CHECK_VERSION),
                ("view.json", VIEW_SCHEMA, VIEW_VERSION),
            ):
                document = read_json(work / filename)
                self.assertEqual(
                    (document["schema"], document["version"], document["producer"]),
                    (schema, version, PRODUCER),
                )
            derivation = read_json(work / "derive.json")
            signals = derivation["signals"]
            self.assertEqual(
                [(item["target"], item["name"], item["delivery"], item["outcome"]) for item in signals],
                [
                    ("Root", "Start", "root", "completed"),
                    ("Child", "Configure", "drives", "completed"),
                    ("Async", "Run", "emits", "completed"),
                    ("Sink", "Missing", "emits", "discarded"),
                ],
            )
            self.assertEqual(signals[1]["handler"]["kind"], "Action")
            self.assertEqual(signals[1]["payload"], [{"name": "mode", "type": "Mode", "value": "Mode::Fast"}])
            self.assertEqual(signals[1]["before_snapshot"]["states"]["Child"], "Base")
            self.assertEqual(signals[1]["after_snapshot"]["states"]["Child"], "Base")
            event_positions = {
                kind: min(item["sequence"] for item in derivation["events"] if item["kind"] == kind)
                for kind in ("drives_wait_finished", "response_completed", "emits_enqueued", "emits_dequeued")
            }
            self.assertLess(event_positions["drives_wait_finished"], event_positions["emits_enqueued"])
            root_complete = next(
                item["sequence"]
                for item in derivation["events"]
                if item["kind"] == "response_completed" and item["signal_id"] == "sig-0001"
            )
            self.assertLess(root_complete, event_positions["emits_enqueued"])
            self.assertLess(event_positions["emits_enqueued"], event_positions["emits_dequeued"])
            self.assertEqual(
                stdout.getvalue(),
                "verdict: complete\n"
                "Human -- Start --> Root[Base:Ready]\n"
                "  Root -- Configure --> Child\n"
                "  Root -- Run --> Async[Base:Ready]\n"
                "  Root -- Missing --> Sink !! discarded: no_handler\n",
            )

    def test_compact_renderer_uses_hierarchy_depth_order_and_actual_states(self) -> None:
        base = {
            "states": {"Child": "Base", "Parent": "Base", "Unknown": "Base"},
            "facts": [],
            "references": {},
        }
        ready = {
            "states": {"Child": "Ready", "Parent": "Base", "Unknown": "Base"},
            "facts": [],
            "references": {},
        }
        transition = {
            "id": "Child.Transition::Preset@Base",
            "kind": "Transition",
            "source_state": "Base",
            "target_state": "Ready",
        }
        action = {
            "id": "Parent.Action::Inspect@Base",
            "kind": "Action",
            "source_state": "Base",
            "target_state": None,
        }
        view = {
            "root_request": {
                "source": "Human",
                "target": "Child",
                "signal": "Preset",
            },
            "verdict": "failed",
            "budget": {"max_depth": 3, "max_breadth": 3},
            "until_request": None,
            "boundary": None,
            "signals": [
                {
                    "id": "sig-0001",
                    "source": "Human",
                    "target": "Child",
                    "name": "Preset",
                    "delivery": "root",
                    "lossy": False,
                    "outcome": "completed",
                    "reason": None,
                    "handler": transition,
                    "before_snapshot": base,
                    "after_snapshot": ready,
                    "coordinate": {"depth": 0, "breadth": 0},
                    "cause_depth": 0,
                },
                {
                    "id": "sig-0002",
                    "source": "Child",
                    "target": "Parent",
                    "name": "Inspect",
                    "delivery": "drives",
                    "lossy": False,
                    "outcome": "completed",
                    "reason": None,
                    "handler": action,
                    "before_snapshot": ready,
                    "after_snapshot": ready,
                    "coordinate": {"depth": -1, "breadth": 0},
                    "cause_depth": 1,
                },
                {
                    "id": "sig-0003",
                    "source": "Child",
                    "target": "Unknown",
                    "name": "Missing",
                    "delivery": "emits",
                    "lossy": False,
                    "outcome": "rejected",
                    "reason": "no_handler",
                    "handler": None,
                    "compat_process_kind": "Transition",
                    "before_snapshot": ready,
                    "after_snapshot": ready,
                    "coordinate": {"depth": 1, "breadth": 0},
                    "cause_depth": 1,
                },
                {
                    "id": "sig-0004",
                    "source": "Parent",
                    "target": "Child",
                    "name": "Preset",
                    "delivery": "drives",
                    "lossy": False,
                    "outcome": "stopped",
                    "reason": "until_signal_reached",
                    "handler": transition,
                    "before_snapshot": ready,
                    "after_snapshot": ready,
                    "coordinate": {"depth": 0, "breadth": 0},
                    "cause_depth": 2,
                },
            ],
            "events": [],
            "truncated_frontier": [],
            "failure": {
                "chain": ["sig-0001", "sig-0003"],
                "reason": "no_handler",
            },
        }

        with mock.patch.dict(os.environ, {}, clear=False):
            os.environ.pop("VERBOSE", None)
            compact = render_text(view)
        self.assertEqual(
            compact,
            "verdict: failed\n"
            "  Human -- Startup --> Child[Base:Ready]\n"
            "Child -- Inspect --> Parent\n"
            "    Child -- Missing --> Unknown !! rejected: no_handler\n"
            "  Parent -- Startup --> Child[Ready:Ready] !! stopped: until_signal_reached\n"
            "failure chain: sig-0001 -> sig-0003; reason: no_handler\n",
        )
        self.assertNotIn("Unknown[", compact)

        for value in ("0", "yes", "01", " 1"):
            with self.subTest(VERBOSE=value), mock.patch.dict(os.environ, {"VERBOSE": value}):
                self.assertEqual(render_text(view), compact)
        with mock.patch.dict(os.environ, {"VERBOSE": "1"}):
            verbose = render_text(view)
        self.assertTrue(verbose.startswith("Signal derivation: Human -> Child.Preset\n"))
        self.assertIn("synchronous: sender waits for this response", verbose)
        self.assertIn("failure reason: no_handler", verbose)
        self.assertNotIn("Startup", verbose)

    def test_text_mode_is_render_only_for_driver_and_render_cli(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            results: dict[str, tuple[int, Path, Path, Path]] = {}
            for mode in ("0", "1"):
                work = root / f"work-{mode}"
                output = root / f"trace-{mode}.txt"
                snapshot = root / f"snapshot-{mode}.json"
                with mock.patch.dict(os.environ, {"VERBOSE": mode}):
                    exit_code = driver_main(
                        [
                            str(PIPELINE),
                            "--signal",
                            "Root.Start",
                            "--max-depth",
                            "all",
                            "--max-breadth",
                            "all",
                            "--work-dir",
                            str(work),
                            "--snapshot-out",
                            str(snapshot),
                            "-o",
                            str(output),
                        ]
                    )
                results[mode] = (exit_code, work, output, snapshot)

            self.assertEqual(results["0"][0], results["1"][0], 0)
            for filename in ("ast.json", "model.json", "derive.json", "check.json", "view.json"):
                self.assertEqual(
                    (results["0"][1] / filename).read_bytes(),
                    (results["1"][1] / filename).read_bytes(),
                )
            self.assertEqual(results["0"][3].read_bytes(), results["1"][3].read_bytes())

            compact = results["0"][2].read_text(encoding="utf-8")
            verbose = results["1"][2].read_text(encoding="utf-8")
            self.assertTrue(compact.startswith("verdict: complete\n"))
            self.assertNotIn("Signal derivation:", compact)
            self.assertTrue(verbose.startswith("Signal derivation: Human -> Root.Start\n"))
            self.assertIn("synchronous: sender waits", verbose)
            self.assertIn("asynchronous FIFO:", verbose)

            render_compact = root / "render-compact.txt"
            render_verbose = root / "render-verbose.txt"
            view = results["0"][1] / "view.json"
            with mock.patch.dict(os.environ, {"VERBOSE": "other"}):
                self.assertEqual(render_main([str(view), "-o", str(render_compact)]), 0)
            with mock.patch.dict(os.environ, {"VERBOSE": "1"}):
                self.assertEqual(render_main([str(view), "-o", str(render_verbose)]), 0)
            self.assertEqual(render_compact.read_text(encoding="utf-8"), compact)
            self.assertEqual(render_verbose.read_text(encoding="utf-8"), verbose)

    def test_type_lifecycle_process_is_composed_with_object_wrapper(self) -> None:
        derivation, checked, _ = self.run_source(
            """
            type Child {
                initial_state: State::Base;
                state State::Base {
                    transitions { on Transition::Setup -> State::Ready { ensures { child_ready(self); } } }
                }
                state State::Ready { invariant { child_ready(self); } }
            }
            type Wrapper {
                owned { child: Child; }
                lifecycle {
                    Transition::Setup {
                        state_effect: StateEffect::Always;
                        drives { self.child.Transition::Setup; }
                        ensures { wrapper_type_ready(self); }
                    }
                }
            }
            object Root: Wrapper {
                initial_state: State::Base;
                state State::Base {
                    transitions {
                        on Transition::Setup -> State::Ready {
                            ensures { wrapper_instance_ready(self); }
                        }
                    }
                }
                state State::Ready {
                    invariant {
                        wrapper_type_ready(self);
                        wrapper_instance_ready(self);
                    }
                }
            }
            """,
            "Root.Setup",
        )
        self.assertEqual(checked["verdict"], "complete")
        self.assertEqual(derivation["signals"][1]["target"], "Root.child")
        self.assertEqual(
            derivation["signals"][0]["handler"]["composed_type_processes"][0]["id"],
            "Wrapper.Transition::Setup@process",
        )
        self.assertEqual(derivation["last_stable_snapshot"]["states"]["Root.child"], "Ready")

    def test_has_slot_uses_instance_field_declared_type_structure(self) -> None:
        source = """
            type BaseFixMapConfig {
                slots { fdt: FixMapSlotRange<Fdt>; }
            }
            type PlatformFixMapConfig: BaseFixMapConfig { }
            object Config {
                initial_state: State::Base;
                attrs { fixmap: PlatformFixMapConfig; }
                state State::Base {
                    actions {
                        on Action::CheckFdt {
                            depends_on { has_slot(Config.fixmap, FixMapSlot::Fdt); }
                            ensures { fdt_slot_checked(); }
                        }
                        on Action::CheckMissing {
                            depends_on { has_slot(Config.fixmap, FixMapSlot::Missing); }
                        }
                    }
                }
            }
        """
        derivation, checked, _ = self.run_source(source, "Config.CheckFdt")
        self.assertEqual(checked["verdict"], "complete")
        self.assertNotIn(
            "has_slot(\"Config.fixmap\",FixMapSlot::Fdt)",
            derivation["initial_snapshot"]["facts"],
        )
        condition = next(
            event
            for event in derivation["events"]
            if event["kind"] == "condition_checked"
        )
        self.assertEqual(condition["expression"], "has_slot(Config.fixmap, FixMapSlot::Fdt)")
        self.assertTrue(condition["result"])
        self.assertEqual(condition["proof_source"], "model_structure")

        missing, missing_checked, _ = self.run_source(source, "Config.CheckMissing")
        self.assertEqual(missing_checked["verdict"], "failed")
        self.assertEqual(missing["signals"][0]["outcome"], "rejected")
        self.assertEqual(
            missing["signals"][0]["reason"],
            "condition_not_satisfied: has_slot(Config.fixmap, FixMapSlot::Missing)",
        )

    def test_protocol_identity_and_old_protocol_rejection(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            ast = root / "ast.json"
            self.assertEqual(parse_main([str(PIPELINE), "-o", str(ast)]), 0)
            data = read_json(ast)
            self.assertEqual((data["schema"], data["version"], data["producer"]), (AST_SCHEMA, AST_VERSION, PRODUCER))

            for old_version in (1, 2):
                old = root / f"old-v{old_version}.ast.json"
                old.write_text(
                    json.dumps(
                        {
                            "schema": AST_SCHEMA,
                            "version": old_version,
                            "producer": PRODUCER,
                            "source": "old",
                            "document": {},
                        }
                    ),
                    encoding="utf-8",
                )
                stderr = io.StringIO()
                with contextlib.redirect_stderr(stderr):
                    self.assertEqual(model_main([str(old), "-o", str(root / "no.json")]), 2)
                self.assertIn("version=3", stderr.getvalue())

                old_snapshot = root / f"old-v{old_version}.snapshot.json"
                old_snapshot.write_text(
                    json.dumps(
                        {
                            "schema": SNAPSHOT_SCHEMA,
                            "version": old_version,
                            "producer": PRODUCER,
                            "source": "old",
                            "snapshot": {"states": {}, "facts": [], "references": {}},
                        }
                    ),
                    encoding="utf-8",
                )
                stderr = io.StringIO()
                with contextlib.redirect_stderr(stderr):
                    self.assertEqual(
                        driver_main(
                            [str(PIPELINE), "--signal", "Root.Start", "--scenario", str(old_snapshot)]
                        ),
                        2,
                    )
                self.assertIn("snapshot protocol mismatch", stderr.getvalue())

            environment = os.environ.copy()
            environment["PYTHONPATH"] = os.pathsep.join(
                [str(ROOT / "tools" / "common" / "src"), str(ROOT / "tools" / "model" / "src")]
            )
            result = subprocess.run(
                [sys.executable, "-m", "model_tool", str(ast), "-o", str(root / "legacy.json")],
                cwd=ROOT,
                env=environment,
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("expected version 4", result.stderr)

    def test_every_tools2_consumer_rejects_wrong_producer(self) -> None:
        cases = [
            (model_main, AST_SCHEMA, AST_VERSION, []),
            (derive_main, MODEL_SCHEMA, MODEL_VERSION, ["--signal", "Root.Go"]),
            (check_main, DERIVE_SCHEMA, DERIVE_VERSION, []),
            (view_main, DERIVE_SCHEMA, DERIVE_VERSION, []),
            (render_main, VIEW_SCHEMA, VIEW_VERSION, []),
        ]
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for index, (entry, schema, version, extra) in enumerate(cases):
                source = root / f"input-{index}.json"
                source.write_text(
                    json.dumps({"schema": schema, "version": version, "producer": "tools"}),
                    encoding="utf-8",
                )
                stderr = io.StringIO()
                with contextlib.redirect_stderr(stderr):
                    exit_code = entry([str(source), *extra, "-o", str(root / f"out-{index}")])
                self.assertEqual(exit_code, 2)
                self.assertIn("producer='tools2'", stderr.getvalue())

    def test_every_tools2_consumer_rejects_v1_and_v2(self) -> None:
        cases = [
            (model_main, AST_SCHEMA, []),
            (derive_main, MODEL_SCHEMA, ["--signal", "Root.Go"]),
            (check_main, DERIVE_SCHEMA, []),
            (view_main, DERIVE_SCHEMA, []),
            (render_main, VIEW_SCHEMA, []),
        ]
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for old_version in (1, 2):
                for index, (entry, schema, extra) in enumerate(cases):
                    source = root / f"input-v{old_version}-{index}.json"
                    source.write_text(
                        json.dumps(
                            {"schema": schema, "version": old_version, "producer": PRODUCER}
                        ),
                        encoding="utf-8",
                    )
                    stderr = io.StringIO()
                    with contextlib.redirect_stderr(stderr):
                        exit_code = entry(
                            [str(source), *extra, "-o", str(root / f"out-v{old_version}-{index}")]
                        )
                    self.assertEqual(exit_code, 2)
                    self.assertIn("version=3", stderr.getvalue())

    def test_include_is_resolved_and_retains_child_source_span(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            child = root / "child.spec"
            spec = root / "root.spec"
            ast = root / "ast.json"
            model = root / "model.json"
            child.write_text(
                "system Included { initial_state: State::Base; state State::Base { actions { on Action::Go { } } } }",
                encoding="utf-8",
            )
            spec.write_text('include "child.spec";\n', encoding="utf-8")
            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            data = read_json(ast)
            included = data["document"]["systems"][0]
            self.assertEqual(included["name"], "Included")
            self.assertEqual(included["span"]["source_file"], str(child))
            self.assertEqual(model_main([str(ast), "-o", str(model)]), 0)

    def test_unsupported_syntax_has_source_span_and_fails_derivation(self) -> None:
        source = """
            system Root {
                initial_state: State::Base;
                state State::Base {
                    transitions {
                        on Transition::Start -> State::Ready {
                            unknown_handler_member { }
                        }
                    }
                }
                state State::Ready { }
            }
        """
        derivation, checked, text = self.run_source(
            source,
            "Root.Start",
        )
        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "unsupported.spec"
            ast = Path(tmp) / "ast.json"
            spec.write_text(textwrap.dedent(source), encoding="utf-8")
            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            diagnostic = read_json(ast)["document"]["diagnostics"][0]
            self.assertEqual(diagnostic["category"], "unsupported")
            self.assertEqual(diagnostic["span"]["source_file"], str(spec))
            self.assertGreater(diagnostic["span"]["start_line"], 0)
        self.assertEqual(derivation["verdict"], "failed")
        self.assertEqual(checked["exit_code"], 1)
        self.assertIn("model_has_errors_or_unsupported_syntax", text)

    def test_payload_type_error_is_failed_not_rejected(self) -> None:
        derivation, _, text = self.run_source(
            """
            enum Mode { Fast }
            system Root {
                initial_state: State::Base;
                state State::Base {
                    transitions {
                        on Transition::Start -> State::Ready {
                            drives { Child.Action::Configure(mode: "fast"); }
                        }
                    }
                }
                state State::Ready { }
            }
            system Child {
                parent: Root;
                initial_state: State::Base;
                state State::Base {
                    actions { on Action::Configure(mode: Mode) { } }
                }
            }
            """,
            "Root.Start",
        )
        self.assertEqual(derivation["verdict"], "failed")
        child = derivation["signals"][1]
        self.assertEqual(child["outcome"], "failed")
        self.assertIn("expected Mode enum payload", child["reason"])
        self.assertNotIn("pending", text.lower().replace("pending=0", ""))

    def test_invariant_failure_is_failed_and_does_not_commit_owner_state(self) -> None:
        derivation, checked, text = self.run_source(
            """
            system Root {
                initial_state: State::Base;
                state State::Base {
                    transitions { on Transition::Start -> State::Ready { } }
                }
                state State::Ready { invariant { ready_fact(self); } }
            }
            """,
            "Root.Start",
        )
        self.assertEqual(checked["verdict"], "failed")
        self.assertEqual(derivation["signals"][0]["outcome"], "failed")
        self.assertEqual(derivation["last_stable_snapshot"]["states"]["Root"], "Base")
        self.assertIn(
            "Human -- Start --> Root[Base:Base] !! failed: invariant_not_satisfied",
            text,
        )

    def test_strict_rejection_fails_with_complete_causal_chain(self) -> None:
        derivation, checked, text = self.run_source(
            """
            system Root {
                initial_state: State::Base;
                state State::Base {
                    transitions {
                        on Transition::Start -> State::Ready {
                            drives { Child.Action::Missing; }
                        }
                    }
                }
                state State::Ready { }
            }
            system Child {
                parent: Root;
                initial_state: State::Base;
                state State::Base { }
            }
            """,
            "Root.Start",
        )
        self.assertEqual(derivation["verdict"], "failed")
        self.assertEqual(checked["exit_code"], 1)
        self.assertEqual(derivation["failure"]["chain"], ["sig-0001", "sig-0002"])
        self.assertEqual(derivation["signals"][1]["outcome"], "rejected")
        self.assertIn("failure chain: sig-0001 -> sig-0002", text)
        self.assertIn("Root -- Missing --> Child !! rejected: no_handler", text)

    def test_rejected_transition_uses_unchanged_actual_snapshot_state(self) -> None:
        derivation, checked, text = self.run_source(
            """
            system Root {
                initial_state: State::Ready;
                state State::Base {
                    transitions { on Transition::Preset -> State::Ready { } }
                }
                state State::Ready { }
            }
            """,
            "Root.Preset",
        )
        self.assertEqual(checked["verdict"], "failed")
        self.assertEqual(derivation["signals"][0]["outcome"], "rejected")
        self.assertEqual(
            text.splitlines()[1],
            "Human -- Startup --> Root[Ready:Ready] !! rejected: "
            "state_not_accepted: expected State::Base, got State::Ready",
        )

    def test_lossy_condition_rejection_is_discarded_and_never_pending(self) -> None:
        derivation, checked, text = self.run_source(
            """
            system Root {
                initial_state: State::Base;
                state State::Base {
                    transitions {
                        on Transition::Start -> State::Ready {
                            emits { lossy Child.Action::Try; }
                        }
                    }
                }
                state State::Ready { }
            }
            system Child {
                parent: Root;
                initial_state: State::Base;
                state State::Base {
                    actions {
                        on Action::Try { depends_on { allowed(self); } }
                    }
                }
            }
            """,
            "Root.Start",
        )
        self.assertEqual(checked["verdict"], "complete")
        self.assertEqual(derivation["signals"][1]["outcome"], "discarded")
        self.assertIn("condition_not_satisfied", derivation["signals"][1]["reason"])
        self.assertEqual(derivation["summary"]["pending"], 0)
        self.assertNotIn("pending", {item["outcome"] for item in derivation["signals"]})
        self.assertIn("!! discarded: condition_not_satisfied", text)

    def test_strict_emits_rejection_preserves_committed_stable_snapshot(self) -> None:
        derivation, _, _ = self.run_source(
            """
            system Root {
                initial_state: State::Base;
                state State::Base {
                    transitions {
                        on Transition::Start -> State::Ready {
                            emits { Child.Action::Missing; }
                            ensures { root_committed(self); }
                        }
                    }
                }
                state State::Ready { invariant { root_committed(self); } }
            }
            system Child { parent: Root; initial_state: State::Base; state State::Base { } }
            """,
            "Root.Start",
        )
        self.assertEqual(derivation["verdict"], "failed")
        self.assertEqual(derivation["signals"][0]["outcome"], "completed")
        self.assertEqual(derivation["signals"][1]["outcome"], "rejected")
        self.assertEqual(derivation["last_stable_snapshot"]["states"]["Root"], "Ready")
        self.assertIn("root_committed(Root)", derivation["last_stable_snapshot"]["facts"])

    def test_scenario_state_fact_and_reference_overrides(self) -> None:
        derivation, checked, _ = self.run_source(
            """
            system Root {
                references { selected: System = Left; }
                initial_state: State::Base;
                state State::Base {
                    actions {
                        on Action::Dispatch {
                            depends_on {
                                ready(self);
                                self.selected == Right;
                            }
                            drives { self.selected.Action::Accept(source: self); }
                        }
                    }
                }
            }
            system Left { parent: Root; initial_state: State::Base; state State::Base { } }
            system Right {
                parent: Root;
                initial_state: State::Base;
                state State::Base {
                    actions { on Action::Accept(source: System) { ensures { accepted(self, source); } } }
                }
            }
            """,
            "Root.Dispatch",
            scenario={
                "states": {"Root": "Base"},
                "facts": ["ready(Root)"],
                "references": {"Root.selected": "Right"},
            },
        )
        self.assertEqual(checked["verdict"], "complete")
        child = derivation["signals"][1]
        self.assertEqual(child["target"], "Right")
        self.assertEqual(child["payload"], [{"name": "source", "type": "System", "value": "Root"}])
        self.assertIn("accepted(Right,Root)", derivation["last_stable_snapshot"]["facts"])

    def test_ambiguous_handler_is_rejected(self) -> None:
        derivation, _, _ = self.run_source(
            """
            system Root {
                initial_state: State::Base;
                state State::Base {
                    transitions { on Transition::Start -> State::Ready { drives { Target.Action::Go; } } }
                }
                state State::Ready { }
            }
            system Target {
                parent: Root;
                initial_state: State::Base;
                state State::Base {
                    transitions { on Transition::Go -> State::Ready { } }
                    actions { on Action::Go { } }
                }
                state State::Ready { }
            }
            """,
            "Root.Start",
        )
        self.assertEqual(derivation["signals"][1]["reason"], "ambiguous_handler")
        self.assertEqual(derivation["verdict"], "failed")

    def test_hierarchy_coordinates_cover_self_down_up_sibling_and_cross_branch(self) -> None:
        derivation, checked, _ = self.run_source(
            """
            system Root {
                initial_state: State::Base;
                state State::Base {
                    transitions {
                        on Transition::Start -> State::Ready {
                            drives {
                                Root.Action::Self;
                                Child.Action::Down;
                                LeftLeaf.Action::Cross;
                            }
                        }
                    }
                    actions { on Action::Self { } on Action::Up { } }
                }
                state State::Ready { }
            }
            system Child {
                parent: Root;
                initial_state: State::Base;
                state State::Base { actions { on Action::Down { drives { Root.Action::Up; } } } }
            }
            system Left { parent: Root; initial_state: State::Base; state State::Base { } }
            system Right { parent: Root; initial_state: State::Base; state State::Base { } }
            system LeftLeaf {
                parent: Left;
                initial_state: State::Base;
                state State::Base { actions { on Action::Cross { drives { RightLeaf.Action::Receive; } } } }
            }
            system RightLeaf {
                parent: Right;
                initial_state: State::Base;
                state State::Base { actions { on Action::Receive { } } }
            }
            """,
            "Root.Start",
        )
        self.assertEqual(checked["verdict"], "complete")
        by_name = {item["name"]: item for item in derivation["signals"]}
        self.assertEqual(by_name["Self"]["coordinate"]["depth"], 0)
        self.assertEqual(by_name["Down"]["coordinate"]["depth"], 1)
        self.assertEqual(by_name["Up"]["coordinate"]["depth"], 0)
        self.assertEqual(by_name["Cross"]["coordinate"]["depth"], 2)
        receive = by_name["Receive"]["coordinate"]
        self.assertEqual((receive["depth"], receive["breadth"]), (2, 0))
        self.assertEqual(receive["movement"], {"up": 2, "across": 1, "down": 2})

    def test_sibling_breadth_budget_and_all(self) -> None:
        source = """
            system Root { initial_state: State::Base; state State::Base { } }
            system Left {
                parent: Root; initial_state: State::Base;
                state State::Base { actions { on Action::Start { drives { Right.Action::Finish; } } } }
            }
            system Right {
                parent: Root; initial_state: State::Base;
                state State::Base { actions { on Action::Finish { } } }
            }
        """
        bounded, checked, _ = self.run_source(source, "Left.Start", max_breadth="0")
        self.assertEqual(checked["verdict"], "bounded")
        self.assertEqual(bounded["signals"][1]["outcome"], "truncated")
        self.assertEqual(bounded["signals"][1]["coordinate"]["breadth"], 1)
        complete, checked, _ = self.run_source(source, "Left.Start", max_breadth="all")
        self.assertEqual(checked["verdict"], "complete")
        self.assertEqual(complete["signals"][1]["outcome"], "completed")

    def test_default_depth_three_truncates_fourth_level_and_all_completes(self) -> None:
        source = """
            system A { initial_state: State::Base; state State::Base { actions { on Action::Go { drives { B.Action::Go; } } } } }
            system B { parent: A; initial_state: State::Base; state State::Base { actions { on Action::Go { drives { C.Action::Go; } } } } }
            system C { parent: B; initial_state: State::Base; state State::Base { actions { on Action::Go { drives { D.Action::Go; } } } } }
            system D { parent: C; initial_state: State::Base; state State::Base { actions { on Action::Go { drives { E.Action::Go; } } } } }
            system E { parent: D; initial_state: State::Base; state State::Base { actions { on Action::Go { } } } }
        """
        bounded, _, text = self.run_source(source, "A.Go")
        self.assertEqual(bounded["verdict"], "bounded")
        self.assertEqual(bounded["truncated_frontier"][0]["coordinate"]["depth"], 4)
        self.assertIn("!! truncated: propagation_budget_exceeded", text)
        complete, _, _ = self.run_source(source, "A.Go", max_depth="all")
        self.assertEqual(complete["verdict"], "complete")
        explicit, _, _ = self.run_source(source, "A.Go", max_depth="4")
        self.assertEqual(explicit["verdict"], "complete")

    def test_upward_signal_from_root_target_consumes_signed_depth_budget(self) -> None:
        source = """
            system Parent {
                initial_state: State::Base;
                state State::Base { actions { on Action::Receive { } } }
            }
            system Child {
                parent: Parent;
                initial_state: State::Base;
                state State::Base { actions { on Action::Start { drives { Parent.Action::Receive; } } } }
            }
        """
        bounded, _, _ = self.run_source(source, "Child.Start", max_depth="0")
        self.assertEqual(bounded["verdict"], "bounded")
        self.assertEqual(bounded["signals"][1]["coordinate"]["depth"], -1)
        complete, _, _ = self.run_source(source, "Child.Start", max_depth="1")
        self.assertEqual(complete["verdict"], "complete")

    def test_top_level_systems_are_siblings_under_virtual_environment(self) -> None:
        derivation, _, _ = self.run_source(
            """
            system Left {
                initial_state: State::Base;
                state State::Base { actions { on Action::Start { drives { Right.Action::Finish; } } } }
            }
            system Right {
                initial_state: State::Base;
                state State::Base { actions { on Action::Finish { } } }
            }
            """,
            "Left.Start",
        )
        self.assertEqual(derivation["verdict"], "complete")
        self.assertEqual(derivation["signals"][1]["coordinate"]["breadth"], 1)

    def test_branches_inherit_budget_independently(self) -> None:
        derivation, _, _ = self.run_source(
            """
            system Root {
                initial_state: State::Base;
                state State::Base { actions { on Action::Go { drives { Left.Action::Go; Right.Action::Go; } } } }
            }
            system Left { parent: Root; initial_state: State::Base; state State::Base { actions { on Action::Go { } } } }
            system Right { parent: Root; initial_state: State::Base; state State::Base { actions { on Action::Go { } } } }
            """,
            "Root.Go",
            max_breadth="0",
        )
        self.assertEqual(derivation["verdict"], "complete")
        self.assertEqual(
            [(item["coordinate"]["depth"], item["coordinate"]["breadth"]) for item in derivation["signals"][1:]],
            [(1, 0), (1, 0)],
        )

    def test_complete_snapshot_can_resume_and_failed_bounded_do_not_write(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            snapshot = root / "snapshot.json"
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(
                    driver_main([str(PIPELINE), "--signal", "Root.Start", "--snapshot-out", str(snapshot)]),
                    0,
                )
            saved = read_json(snapshot)
            self.assertEqual(saved["schema"], SNAPSHOT_SCHEMA)
            self.assertEqual(saved["snapshot"]["states"]["Root"], "Ready")
            resumed_work = root / "resumed"
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(
                    driver_main(
                        [
                            str(PIPELINE),
                            "--signal",
                            "Root.Inspect",
                            "--scenario",
                            str(snapshot),
                            "--work-dir",
                            str(resumed_work),
                        ]
                    ),
                    0,
                )
            self.assertIn("inspected(Root)", read_json(resumed_work / "derive.json")["last_stable_snapshot"]["facts"])

            failed_spec = root / "failed.spec"
            failed_spec.write_text(
                "system Root { initial_state: State::Base; state State::Base { actions { on Action::Go { drives { Root.Action::Missing; } } } } }",
                encoding="utf-8",
            )
            failed_snapshot = root / "failed-snapshot.json"
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(
                    driver_main([str(failed_spec), "--signal", "Root.Go", "--snapshot-out", str(failed_snapshot)]),
                    1,
                )
            self.assertFalse(failed_snapshot.exists())

            bounded_snapshot = root / "bounded-snapshot.json"
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(
                    driver_main(
                        [
                            str(PIPELINE),
                            "--signal",
                            "Root.Start",
                            "--max-depth",
                            "0",
                            "--snapshot-out",
                            str(bounded_snapshot),
                        ]
                    ),
                    1,
                )
            self.assertFalse(bounded_snapshot.exists())

    def test_repeated_runs_have_identical_ids_order_and_json(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for name in ("one", "two"):
                self.assertEqual(
                    driver_main(
                        [str(PIPELINE), "--signal", "Root.Start", "--work-dir", str(root / name), "-o", str(root / f"{name}.txt")]
                    ),
                    0,
                )
            self.assertEqual((root / "one" / "derive.json").read_bytes(), (root / "two" / "derive.json").read_bytes())

    def test_view_is_a_projection_not_a_rederivation(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(driver_main([str(PIPELINE), "--signal", "Root.Start", "--work-dir", str(work)]), 0)
            derivation = read_json(work / "derive.json")
            view = read_json(work / "view.json")
            self.assertFalse(view["metadata"]["rederived"])
            self.assertEqual(
                [{key: value for key, value in item.items() if key != "cause_depth"} for item in view["signals"]],
                derivation["signals"],
            )
            self.assertEqual(view["events"], derivation["events"])

    def test_startup_alias_is_canonical_across_derive_driver_and_shortcut(self) -> None:
        source = """
            system Root {
                initial_state: State::Base;
                state State::Base {
                    transitions {
                        on Transition::Preset -> State::Ready {
                            drives { Child.Transition::Preset; }
                        }
                    }
                }
                state State::Ready { }
            }
            system Child {
                parent: Root;
                initial_state: State::Base;
                state State::Base {
                    transitions { on Transition::Preset -> State::Ready { } }
                }
                state State::Ready { }
            }
        """
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            spec = root / "alias.spec"
            ast = root / "ast.json"
            model_path = root / "model.json"
            spec.write_text(textwrap.dedent(source), encoding="utf-8")
            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            self.assertEqual(model_main([str(ast), "-o", str(model_path)]), 0)
            model = read_json(model_path)

            api_startup = derive(
                model,
                signal="Root.Startup",
                until="Child.Startup",
                max_depth=None,
                max_breadth=None,
            )
            api_preset = derive(
                model,
                signal="Root.Preset",
                until="Child.Preset",
                max_depth=None,
                max_breadth=None,
            )
            self.assertEqual(api_startup, api_preset)
            self.assertEqual(api_startup["root_request"]["signal"], "Preset")
            self.assertEqual(api_startup["until_request"]["signal"], "Preset")
            self.assertEqual(api_startup["root_request"]["source"], "Human")

            cli_outputs = []
            for spelling in ("Startup", "Preset"):
                output = root / f"derive-{spelling}.json"
                self.assertEqual(
                    derive_main(
                        [
                            str(model_path),
                            "--signal",
                            f"Root.{spelling}",
                            "-u" if spelling == "Startup" else "--until",
                            f"Child.{spelling}",
                            "--max-depth",
                            "all",
                            "--max-breadth",
                            "all",
                            "-o",
                            str(output),
                        ]
                    ),
                    0,
                )
                cli_outputs.append(output.read_bytes())
            self.assertEqual(cli_outputs[0], cli_outputs[1])

            driver_outputs = []
            for spelling in ("Startup", "Preset"):
                work = root / f"driver-{spelling}"
                with contextlib.redirect_stdout(io.StringIO()):
                    self.assertEqual(
                        driver_main(
                            [
                                str(spec),
                                "--signal",
                                f"Root.{spelling}",
                                "-u" if spelling == "Startup" else "--until",
                                f"Child.{spelling}",
                                "--work-dir",
                                str(work),
                            ]
                        ),
                        0,
                    )
                driver_outputs.append((work / "derive.json").read_bytes())
            self.assertEqual(driver_outputs[0], driver_outputs[1])

            shortcut_outputs = []
            shortcut = TOOLS2 / "bin" / "pyveri"
            for spelling in ("Startup", "Preset"):
                work = root / f"shortcut-{spelling}"
                result = subprocess.run(
                    [
                        str(shortcut),
                        "-f",
                        str(spec),
                        "-t",
                        f"Root.{spelling}",
                        "-u",
                        f"Child.{spelling}",
                        "--work-dir",
                        str(work),
                    ],
                    cwd=root,
                    text=True,
                    capture_output=True,
                    check=False,
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                shortcut_outputs.append((work / "derive.json").read_bytes())
            self.assertEqual(shortcut_outputs[0], shortcut_outputs[1])

    def test_until_root_stops_before_signal_identity_or_handler(self) -> None:
        derivation, checked, text = self.run_source(
            """
            system Root {
                initial_state: State::Base;
                state State::Base {
                    transitions { on Transition::Preset -> State::Ready { } }
                }
                state State::Ready { }
            }
            """,
            "Root.Startup",
            until="Root.Startup",
        )
        self.assertEqual(checked["verdict"], "reached")
        self.assertEqual(checked["exit_code"], 0)
        self.assertEqual(derivation["signals"], [])
        self.assertIsNone(derivation["root_request"]["signal_id"])
        self.assertEqual(derivation["boundary"]["normalized_signal"], "Root.Preset")
        self.assertEqual(
            derivation["boundary"]["snapshot"], derivation["initial_snapshot"]
        )
        self.assertFalse(
            any(event["kind"] == "signal_sent" for event in derivation["events"])
        )
        self.assertEqual(
            text,
            "verdict: reached\n"
            "boundary: Human -- Startup --> Root (before send)\n",
        )

    def test_until_target_budget_and_handler_are_not_checked(self) -> None:
        derivation, checked, _ = self.run_source(
            """
            system Root {
                initial_state: State::Base;
                state State::Base {
                    actions { on Action::Start { drives { Target.Action::Missing; } } }
                }
            }
            system Target {
                parent: Root;
                initial_state: State::Base;
                state State::Base { }
            }
            """,
            "Root.Start",
            until="Target.Missing",
            max_depth="0",
        )
        self.assertEqual(checked["verdict"], "reached")
        self.assertEqual(derivation["truncated_frontier"], [])
        self.assertEqual(len(derivation["signals"]), 1)
        self.assertEqual(derivation["signals"][0]["outcome"], "stopped")
        self.assertFalse(
            any(
                event["kind"] in {"signal_sent", "signal_received", "signal_truncated"}
                and event.get("target") == "Target"
                for event in derivation["events"]
            )
        )

    def test_until_drives_stops_uncommitted_ancestors_without_target_signal(self) -> None:
        derivation, checked, text = self.run_source(
            """
            system Root {
                initial_state: State::Base;
                state State::Base {
                    transitions {
                        on Transition::Start -> State::Ready {
                            drives { Middle.Action::Go; }
                        }
                    }
                }
                state State::Ready { }
            }
            system Middle {
                parent: Root;
                initial_state: State::Base;
                state State::Base {
                    actions { on Action::Go { drives { Target.Action::StopHere; } } }
                }
            }
            system Target {
                parent: Middle;
                initial_state: State::Base;
                state State::Base { actions { on Action::StopHere { } } }
            }
            """,
            "Root.Start",
            until="Target.StopHere",
        )
        self.assertEqual(checked["verdict"], "reached")
        self.assertEqual(
            [(item["target"], item["name"], item["outcome"]) for item in derivation["signals"]],
            [("Root", "Start", "stopped"), ("Middle", "Go", "stopped")],
        )
        self.assertEqual(derivation["summary"]["stopped"], 2)
        self.assertEqual(derivation["summary"]["pending"], 0)
        self.assertIn(
            "Human -- Start --> Root[Base:Base] !! stopped: until_signal_reached",
            text,
        )
        self.assertIn(
            "  Root -- Go --> Middle !! stopped: until_signal_reached",
            text,
        )
        self.assertFalse(
            any(
                item["target"] == "Target" and item["name"] == "StopHere"
                for item in derivation["signals"]
            )
        )
        self.assertEqual(
            derivation["last_stable_snapshot"], derivation["boundary"]["snapshot"]
        )

    def test_until_emits_stops_before_enqueue_and_preserves_completed_responses(self) -> None:
        derivation, checked, _ = self.run_source(
            """
            system Root {
                initial_state: State::Base;
                state State::Base {
                    transitions {
                        on Transition::Start -> State::Ready {
                            drives { Child.Action::Prepare; }
                            emits { Target.Action::Run; }
                            ensures { root_ready(self); }
                        }
                    }
                }
                state State::Ready { invariant { root_ready(self); } }
            }
            system Child {
                parent: Root;
                initial_state: State::Base;
                state State::Base { actions { on Action::Prepare { } } }
            }
            system Target {
                parent: Root;
                initial_state: State::Base;
                state State::Base { actions { on Action::Run { } } }
            }
            """,
            "Root.Start",
            until="Target.Run",
        )
        self.assertEqual(checked["verdict"], "reached")
        self.assertEqual(
            [(item["target"], item["outcome"]) for item in derivation["signals"]],
            [("Root", "completed"), ("Child", "completed")],
        )
        self.assertEqual(derivation["last_stable_snapshot"]["states"]["Root"], "Ready")
        self.assertIn("root_ready(Root)", derivation["last_stable_snapshot"]["facts"])
        self.assertFalse(
            any(
                event["kind"] == "emits_enqueued"
                and event.get("child_id") not in {item["id"] for item in derivation["signals"]}
                for event in derivation["events"]
            )
        )
        self.assertFalse(any(item["target"] == "Target" for item in derivation["signals"]))

    def test_until_choice_uses_selected_dynamic_receiver_and_first_repetition(self) -> None:
        derivation, checked, _ = self.run_source(
            """
            system Root {
                references { selected: System = Right; }
                initial_state: State::Base;
                state State::Base {
                    actions {
                        on Action::Start {
                            drives {
                                Left.Action::Go || self.selected.Action::Go;
                                Right.Action::Go;
                            }
                        }
                    }
                }
            }
            system Left {
                parent: Root;
                initial_state: State::Base;
                state State::Base { }
            }
            system Right {
                parent: Root;
                initial_state: State::Base;
                state State::Base { actions { on Action::Go { } } }
            }
            """,
            "Root.Start",
            until="Right.Go",
        )
        self.assertEqual(checked["verdict"], "reached")
        self.assertEqual(len(derivation["signals"]), 1)
        self.assertEqual(derivation["signals"][0]["outcome"], "stopped")
        selected = [
            event for event in derivation["events"] if event["kind"] == "drives_choice_selected"
        ]
        self.assertEqual(len(selected), 1)
        self.assertEqual(selected[0]["candidate"], 1)
        self.assertEqual(derivation["boundary"]["target"], "Right")
        self.assertEqual(
            sum(event["kind"] == "until_signal_reached" for event in derivation["events"]),
            1,
        )

    def test_until_with_existing_fifo_marks_only_created_signal_stopped(self) -> None:
        derivation, checked, text = self.run_source(
            """
            system Root {
                initial_state: State::Base;
                state State::Base {
                    transitions {
                        on Transition::Start -> State::Ready {
                            emits {
                                First.Action::Run;
                                Boundary.Action::Run;
                                Last.Action::Run;
                            }
                        }
                    }
                }
                state State::Ready { }
            }
            system First { parent: Root; initial_state: State::Base; state State::Base { actions { on Action::Run { } } } }
            system Boundary { parent: Root; initial_state: State::Base; state State::Base { actions { on Action::Run { } } } }
            system Last { parent: Root; initial_state: State::Base; state State::Base { actions { on Action::Run { } } } }
            """,
            "Root.Start",
            until="Boundary.Run",
        )
        self.assertEqual(checked["verdict"], "reached")
        self.assertEqual(
            [(item["target"], item["outcome"]) for item in derivation["signals"]],
            [("Root", "completed"), ("First", "stopped")],
        )
        first = derivation["signals"][1]
        self.assertEqual(first["before_snapshot"], first["after_snapshot"])
        self.assertTrue(
            any(
                event["kind"] == "signal_stopped" and event["signal_id"] == first["id"]
                for event in derivation["events"]
            )
        )
        self.assertFalse(
            any(event["kind"] == "emits_dequeued" for event in derivation["events"])
        )
        self.assertNotIn("Last.Run", text)

    def test_until_failure_bounded_and_unreached_precedence_do_not_snapshot(self) -> None:
        source = """
            system Root {
                initial_state: State::Base;
                state State::Base {
                    actions {
                        on Action::FailFirst {
                            drives { Missing.Action::Run; Boundary.Action::Run; }
                        }
                        on Action::BoundFirst {
                            drives { Deep.Action::Run; Boundary.Action::Run; }
                        }
                        on Action::Complete { }
                    }
                }
            }
            system Missing { parent: Root; initial_state: State::Base; state State::Base { } }
            system Deep { parent: Root; initial_state: State::Base; state State::Base { actions { on Action::Run { } } } }
            system Boundary { parent: Root; initial_state: State::Base; state State::Base { actions { on Action::Run { } } } }
        """
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            spec = root / "precedence.spec"
            spec.write_text(textwrap.dedent(source), encoding="utf-8")
            cases = [
                ("Root.FailFirst", "Boundary.Run", "all", "failed"),
                ("Root.BoundFirst", "Boundary.Run", "0", "bounded"),
                ("Root.Complete", "Boundary.Run", "all", "until_signal_not_reached"),
            ]
            for index, (signal, until, depth, verdict) in enumerate(cases):
                work = root / f"work-{index}"
                snapshot = root / f"snapshot-{index}.json"
                with contextlib.redirect_stdout(io.StringIO()):
                    exit_code = driver_main(
                        [
                            str(spec),
                            "--signal",
                            signal,
                            "--until",
                            until,
                            "--max-depth",
                            depth,
                            "--work-dir",
                            str(work),
                            "--snapshot-out",
                            str(snapshot),
                        ]
                    )
                self.assertEqual(exit_code, 1)
                data = read_json(work / "derive.json")
                self.assertEqual(data["verdict"], verdict)
                self.assertFalse(snapshot.exists())
            bounded = read_json(root / "work-1" / "derive.json")
            self.assertIsNotNone(bounded["boundary"])
            self.assertEqual(bounded["signals"][0]["outcome"], "stopped")
            unreached_check = read_json(root / "work-2" / "check.json")
            self.assertIn("until_signal_not_reached", unreached_check["reasons"][0])

    def test_reached_snapshot_is_v3_with_boundary_provenance_and_resumes(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            work = root / "work"
            snapshot = root / "snapshot.json"
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(
                    driver_main(
                        [
                            str(PIPELINE),
                            "--signal",
                            "Root.Start",
                            "--until",
                            "Async.Run",
                            "--work-dir",
                            str(work),
                            "--snapshot-out",
                            str(snapshot),
                        ]
                    ),
                    0,
                )
            saved = read_json(snapshot)
            derivation = read_json(work / "derive.json")
            checked = read_json(work / "check.json")
            view = read_json(work / "view.json")
            self.assertEqual(saved["version"], SNAPSHOT_VERSION)
            self.assertEqual(saved["provenance"]["verdict"], "reached")
            self.assertEqual(saved["provenance"]["boundary"], derivation["boundary"])
            self.assertEqual(saved["snapshot"], derivation["boundary"]["snapshot"])
            self.assertEqual(checked["boundary"], derivation["boundary"])
            self.assertEqual(view["boundary"], derivation["boundary"])
            self.assertEqual(view["until_request"], derivation["until_request"])

            resumed = root / "resumed"
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(
                    driver_main(
                        [
                            str(PIPELINE),
                            "--signal",
                            "Async.Run",
                            "--scenario",
                            str(snapshot),
                            "--work-dir",
                            str(resumed),
                        ]
                    ),
                    0,
                )
            resumed_data = read_json(resumed / "derive.json")
            self.assertEqual(resumed_data["initial_snapshot"], saved["snapshot"])
            self.assertEqual(resumed_data["signals"][0]["target"], "Async")

    def test_pyveri_short_trigger_and_scenario_work_from_any_directory(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            shortcut = TOOLS2 / "bin" / "pyveri"
            help_result = subprocess.run(
                [str(shortcut), "-h"],
                cwd=ROOT,
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(help_result.returncode, 0, help_result.stderr)
            self.assertTrue(
                help_result.stdout.startswith(
                    "usage: tools2/bin/pyveri [-h] [-t SIGNAL] [-u SIGNAL]"
                ),
                help_result.stdout,
            )
            self.assertIn("--until SIGNAL", help_result.stdout)
            self.assertFalse((TOOLS2 / "pyveri2").exists())

            snapshot = root / "snapshot.json"
            first_work = root / "first-work"
            first = subprocess.run(
                [
                    str(shortcut),
                    "-t",
                    "Root.Start",
                    "-f",
                    str(PIPELINE),
                    "--source",
                    "TestHarness",
                    "--snapshot-out",
                    str(snapshot),
                    "--work-dir",
                    str(first_work),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(first.returncode, 0, first.stderr)
            self.assertIn("verdict: complete", first.stdout)
            first_derivation = read_json(first_work / "derive.json")
            self.assertEqual(first_derivation["verdict"], "complete")
            self.assertEqual(first_derivation["budget"], {"max_breadth": None, "max_depth": None})
            self.assertEqual(first_derivation["signals"][0]["source"], "TestHarness")
            self.assertTrue(snapshot.exists())

            resumed_text = root / "resumed.txt"
            resumed_work = root / "resumed-work"
            resumed = subprocess.run(
                [
                    str(shortcut),
                    "-t",
                    "Root.Inspect",
                    "-f",
                    str(PIPELINE),
                    "-s",
                    str(snapshot),
                    "--work-dir",
                    str(resumed_work),
                    "-o",
                    str(resumed_text),
                ],
                cwd=root,
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(resumed.returncode, 0, resumed.stderr)
            self.assertIn(
                "Human -- Inspect --> Root",
                resumed_text.read_text(encoding="utf-8"),
            )

            verbose_work = root / "verbose-work"
            verbose_environment = dict(os.environ)
            verbose_environment["VERBOSE"] = "1"
            verbose = subprocess.run(
                [
                    str(shortcut),
                    "-t",
                    "Root.Start",
                    "-f",
                    str(PIPELINE),
                    "--source",
                    "TestHarness",
                    "--work-dir",
                    str(verbose_work),
                ],
                cwd=root,
                env=verbose_environment,
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(verbose.returncode, first.returncode, verbose.stderr)
            self.assertTrue(
                verbose.stdout.startswith("Signal derivation: TestHarness -> Root.Start\n")
            )
            self.assertEqual(
                (verbose_work / "derive.json").read_bytes(),
                (first_work / "derive.json").read_bytes(),
            )

            bounded = subprocess.run(
                [
                    str(shortcut),
                    "-t",
                    "Root.Start",
                    "-f",
                    str(PIPELINE),
                    "--max-depth",
                    "0",
                ],
                cwd=root,
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(bounded.returncode, 1)
            self.assertIn("verdict: bounded", bounded.stdout)

    def test_main_model_default_request_reaches_kernel_presend_and_snapshot_resumes(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            shortcut = TOOLS2 / "bin" / "pyveri"
            work = root / "kernel-boundary"
            snapshot = root / "kernel-boundary.snapshot.json"
            reached = subprocess.run(
                [
                    str(shortcut),
                    "-u",
                    "Kernel.Startup",
                    "--work-dir",
                    str(work),
                    "--snapshot-out",
                    str(snapshot),
                ],
                cwd=root,
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(reached.returncode, 0, reached.stderr)
            derivation = read_json(work / "derive.json")
            self.assertEqual(derivation["verdict"], "reached")
            self.assertEqual(
                (derivation["root_request"]["source"], derivation["root_request"]["target"], derivation["root_request"]["signal"]),
                ("Human", "ComputerProject", "Preset"),
            )
            self.assertEqual(derivation["boundary"]["normalized_signal"], "Kernel.Preset")
            self.assertEqual(
                [
                    (item["source"], item["target"], item["name"])
                    for item in derivation["signals"]
                ],
                [
                    ("Human", "ComputerProject", "Preset"),
                    ("ComputerProject", "HardwareProject", "Preset"),
                    ("ComputerProject", "FirmwareProject", "Preset"),
                    ("ComputerProject", "KernelProject", "Preset"),
                    ("ComputerProject", "ComputerProject", "Setup"),
                    ("ComputerProject", "HardwareProject", "Setup"),
                    ("ComputerProject", "FirmwareProject", "Setup"),
                    ("ComputerProject", "KernelProject", "Setup"),
                    ("KernelProject", "Config", "Preset"),
                    ("Config", "Config", "Setup"),
                    ("Config", "Config", "Enable"),
                    ("KernelProject", "Lds", "Preset"),
                    ("Lds", "Lds", "Setup"),
                    ("Lds", "Lds", "Enable"),
                    ("ComputerProject", "ComputerProject", "Enable"),
                    ("ComputerProject", "Computer", "Preset"),
                    ("Computer", "Computer", "Setup"),
                    ("Computer", "Computer", "Enable"),
                    ("Computer", "Riscv64Platform", "Preset"),
                    ("Riscv64Platform", "Riscv64Platform", "Setup"),
                    ("Riscv64Platform", "Riscv64Platform", "Enable"),
                    ("Riscv64Platform", "OpenSBI", "Preset"),
                    ("OpenSBI", "OpenSBI", "Setup"),
                    ("OpenSBI", "OpenSBI", "Enable"),
                ],
            )
            self.assertFalse(
                any(
                    item["target"] == "Kernel" and item["name"] == "Preset"
                    for item in derivation["signals"]
                )
            )
            self.assertFalse(
                any(
                    item["target"] in {"BootArgs", "BootCpuRegisters"}
                    for item in derivation["signals"]
                )
            )
            boundary_states = derivation["boundary"]["snapshot"]["states"]
            self.assertEqual(
                {
                    name: boundary_states[name]
                    for name in (
                        "HardwareProject",
                        "FirmwareProject",
                        "KernelProject",
                        "BootArgs",
                        "Config",
                        "Lds",
                        "Computer",
                        "Riscv64Platform",
                        "BootCpuRegisters",
                        "OpenSBI",
                        "Kernel",
                    )
                },
                {
                    "HardwareProject": "Ready",
                    "FirmwareProject": "Ready",
                    "KernelProject": "Ready",
                    "BootArgs": "Online",
                    "Config": "Online",
                    "Lds": "Online",
                    "Computer": "Online",
                    "Riscv64Platform": "Online",
                    "BootCpuRegisters": "Online",
                    "OpenSBI": "Online",
                    "Kernel": "Base",
                },
            )
            self.assertIn(
                "computer_assembled_from(Riscv64Platform,OpenSBI,Kernel)",
                derivation["boundary"]["snapshot"]["facts"],
            )
            self.assertIn(
                "assert:BootCpuRegisters.a0 == BootArgs.boot_hartid",
                derivation["boundary"]["snapshot"]["facts"],
            )
            self.assertIn(
                "assert:BootCpuRegisters.a1 == BootArgs.dtb_pa",
                derivation["boundary"]["snapshot"]["facts"],
            )
            saved = read_json(snapshot)
            self.assertEqual(saved["snapshot"], derivation["boundary"]["snapshot"])

            normal_work = root / "normal-closure"
            normal = subprocess.run(
                [
                    str(shortcut),
                    "--work-dir",
                    str(normal_work),
                    "-o",
                    str(root / "normal-closure.txt"),
                ],
                cwd=root,
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertIn(normal.returncode, {0, 1}, normal.stderr)
            normal_data = read_json(normal_work / "derive.json")
            self.assertIsNone(normal_data["until_request"])
            self.assertIsNone(normal_data["boundary"])
            self.assertTrue(
                any(
                    item["target"] == "Kernel" and item["name"] == "Preset"
                    for item in normal_data["signals"]
                )
            )
            fixmap = next(
                item
                for item in normal_data["signals"]
                if item["target"] == "FixMap" and item["name"] == "Preset"
            )
            self.assertEqual(fixmap["outcome"], "completed")
            normal_states = normal_data["last_stable_snapshot"]["states"]
            self.assertEqual(normal_states["ComputerProject"], "Online")
            self.assertEqual(normal_states["HardwareProject"], "Ready")
            self.assertEqual(normal_states["FirmwareProject"], "Ready")
            self.assertEqual(normal_states["KernelProject"], "Ready")
            self.assertEqual(normal_states["BootArgs"], "Online")
            self.assertEqual(normal_states["Config"], "Online")
            self.assertEqual(normal_states["Lds"], "Online")
            self.assertEqual(normal_states["Computer"], "Online")
            self.assertEqual(normal_states["Riscv64Platform"], "Online")
            self.assertEqual(normal_states["BootCpuRegisters"], "Online")
            self.assertEqual(normal_states["OpenSBI"], "Online")

            resumed_work = root / "kernel-resumed"
            resumed = subprocess.run(
                [
                    str(shortcut),
                    "-t",
                    "Kernel.Startup",
                    "-s",
                    str(snapshot),
                    "--max-depth",
                    "0",
                    "--work-dir",
                    str(resumed_work),
                ],
                cwd=root,
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertIn(resumed.returncode, {0, 1}, resumed.stderr)
            resumed_data = read_json(resumed_work / "derive.json")
            self.assertEqual(resumed_data["initial_snapshot"], saved["snapshot"])
            self.assertEqual(
                (resumed_data["root_request"]["target"], resumed_data["root_request"]["signal"]),
                ("Kernel", "Preset"),
            )
            self.assertIsNone(resumed_data["until_request"])


if __name__ == "__main__":
    unittest.main()
