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

from check_tool.__main__ import main as check_main
from derive_tool.__main__ import main as derive_main
from model_tool.__main__ import main as model_main
from parse_tool.__main__ import main as parse_main
from pyveri.__main__ import main as driver_main
from render_tool.__main__ import main as render_main
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
        self.assertEqual(derive_main(derive_args), 0)
        check_exit = check_main([str(derivation), "-o", str(checked)])
        self.assertIn(check_exit, {0, 1})
        self.assertEqual(view_main([str(derivation), "-o", str(view)]), 0)
        self.assertEqual(render_main([str(view), "-o", str(text)]), 0)
        return read_json(derivation), read_json(checked), text.read_text(encoding="utf-8")

    def test_drives_and_emits_order(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp) / "work"
            stdout = io.StringIO()
            with contextlib.redirect_stdout(stdout):
                exit_code = driver_main(
                    [str(PIPELINE), "--signal", "Root.Start", "--work-dir", str(work)]
                )
            self.assertEqual(exit_code, 0)
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
            self.assertIn("synchronous: sender waits", stdout.getvalue())
            self.assertIn("asynchronous FIFO", stdout.getvalue())

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

    def test_protocol_identity_and_old_protocol_rejection(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            ast = root / "ast.json"
            self.assertEqual(parse_main([str(PIPELINE), "-o", str(ast)]), 0)
            data = read_json(ast)
            self.assertEqual((data["schema"], data["version"], data["producer"]), (AST_SCHEMA, AST_VERSION, PRODUCER))

            old = root / "old.ast.json"
            old.write_text(
                json.dumps(
                    {
                        "schema": AST_SCHEMA,
                        "version": 1,
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
            self.assertIn("version=2", stderr.getvalue())

            old_snapshot = root / "old.snapshot.json"
            old_snapshot.write_text(
                json.dumps(
                    {
                        "schema": SNAPSHOT_SCHEMA,
                        "version": 1,
                        "producer": PRODUCER,
                        "source": "old",
                        "snapshot": {"states": {}, "facts": [], "references": {}},
                    }
                ),
                encoding="utf-8",
            )
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
        self.assertIn("invariant_not_satisfied", text)

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
        self.assertIn("no_handler", text)

    def test_lossy_condition_rejection_is_discarded_and_never_pending(self) -> None:
        derivation, checked, _ = self.run_source(
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
        self.assertIn("truncated frontier", text)
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
                help_result.stdout.startswith("usage: tools2/bin/pyveri [-h] -t SIGNAL"),
                help_result.stdout,
            )
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
            self.assertIn("inspected(Root)", resumed_text.read_text(encoding="utf-8"))

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


if __name__ == "__main__":
    unittest.main()
