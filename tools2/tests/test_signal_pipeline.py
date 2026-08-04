from __future__ import annotations

import contextlib
from copy import deepcopy
import hashlib
import io
import json
import os
from pathlib import Path
import runpy
import subprocess
import sys
import tempfile
import textwrap
import time
import unittest
from unittest import mock

from check_tool.__main__ import main as check_main
from check_tool.policy import check_derivation
from animate_tool.builder import build_animation
from animate_tool.html import render_html
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
    write_json,
)
from view_tool.__main__ import main as view_main
from view_tool.builder import build_view


ROOT = Path(__file__).resolve().parents[2]
TOOLS2 = ROOT / "tools2"
PIPELINE = TOOLS2 / "tests" / "fixtures" / "pipeline.spec"
KERNEL_ENABLE_SCENARIO = TOOLS2 / "scenarios" / "Kernel.Enable.snapshot.json"
BOOT_INIT_SETUP_SCENARIO = TOOLS2 / "scenarios" / "BootInitFlow.Setup.snapshot.json"
CPU0_SCHEDULER_SCHEDULE_SCENARIO = (
    TOOLS2 / "scenarios" / "Cpu0Scheduler.Schedule.snapshot.json"
)
SHORTCUT_MAIN = runpy.run_path(
    str(TOOLS2 / "bin" / "pyveri"), run_name="tools2_test_shortcut"
)["main"]


class _ShortcutTestSupport:
    def run_shortcut_focused(
        self, arguments: list[str], *, downstream_exit: int = 0
    ) -> tuple[int, list[str] | None, str, str]:
        downstream_arguments = None

        def downstream(values: list[str]) -> int:
            nonlocal downstream_arguments
            downstream_arguments = values
            return downstream_exit

        stdout = io.StringIO()
        stderr = io.StringIO()
        with mock.patch("pyveri.__main__.main", side_effect=downstream), \
             contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            try:
                result = SHORTCUT_MAIN(arguments)
            except SystemExit as error:
                result = int(error.code)
        return result, downstream_arguments, stdout.getvalue(), stderr.getvalue()


class SignalPipelineTests(_ShortcutTestSupport, unittest.TestCase):
    def run_source(
        self,
        source: str,
        signal: str | None,
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
            "--max-depth",
            max_depth,
            "--max-breadth",
            max_breadth,
            "-o",
            str(derivation),
        ]
        if signal is not None:
            derive_args.extend(["--signal", signal])
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
        checked_data = read_json(checked)
        expected_check_exit = 0 if checked_data["allowed"] else 1
        self.assertEqual(check_exit, expected_check_exit)
        self.assertEqual(view_main([str(derivation), "-o", str(view)]), 0)
        with mock.patch.dict(os.environ, {"VERBOSE": "0"}):
            self.assertEqual(render_main([str(view), "-o", str(text)]), 0)
        rendered = text.read_text(encoding="utf-8")
        model_diagnostics = read_json(model).get("diagnostics", [])
        if model_diagnostics:
            rendered += "\nmodel diagnostics: " + json.dumps(model_diagnostics, sort_keys=True)
        return read_json(derivation), checked_data, rendered

    @staticmethod
    def dispatch_context(snapshot: dict, flow: str) -> dict:
        dispatched = deepcopy(snapshot)
        record = dispatched["context_continuations"][flow]
        ordinal = int(record.get("dispatch_ordinal", 0)) + 1
        record["dispatch_ordinal"] = ordinal
        record["dispatch_proof"] = {
            "flow": flow,
            "cpu": record["cpu"],
            "task_ref": record["task_ref"],
            "generation": record["generation"],
            "context_epoch": record["context_epoch"],
            "dispatch_ordinal": ordinal,
            "consumed": False,
            "signal_id": "test-dispatch",
        }
        return dispatched

    @classmethod
    def save_and_dispatch_yield_context(cls, snapshot: dict, flow: str) -> dict:
        saved = deepcopy(snapshot)
        tokens = [
            token
            for token in saved["yield_tokens"].values()
            if token.get("flow_ref") == flow
            and token.get("status") == "awaiting-resume"
        ]
        if len(tokens) != 1:
            raise AssertionError(f"expected one pending yield token for {flow}")
        token = tokens[0]
        epoch = int(token.get("context_epoch", 0)) + 1
        token["context_epoch"] = epoch
        lane = saved["task_flow_lanes"][token["lane"]]
        lane.update(
            {
                "cpu": token["cpu"],
                "task_ref": token["task_ref"],
                "generation": token["generation"],
                "context_epoch": epoch,
            }
        )
        saved["context_continuations"][flow] = {
            "flow": flow,
            "lane": token["lane"],
            "cpu": token["cpu"],
            "task_ref": token["task_ref"],
            "generation": token["generation"],
            "context_epoch": epoch,
            "dispatch_ordinal": 0,
            "kind": "yield",
            "coordinate": {
                "token_id": token["id"],
                "resume": deepcopy(token["resume_coordinate"]),
            },
            "status": "available",
            "dispatch_proof": None,
            "published_by": "test-save",
        }
        return cls.dispatch_context(saved, flow)

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
                    ("Sink", "Observe", "emits", "completed"),
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
                "boundaries: deferred=0 trimmed=0 occurrences=0 obligations=0\n"
                "Human -- Start --> Root[Base:Ready]\n"
                "  Root -- Configure --> Child\n"
                "  Root -- Run --> Async[Base:Ready]\n"
                "  Root -- Observe --> Sink\n",
            )

    def test_yields_is_immediate_non_fifo_and_has_no_implicit_machine_delta(self) -> None:
        source = """
            system Flow {
                initial_state: State::Online;
                state State::Online {
                    actions {
                        on Action::Run {
                            state_effect: StateEffect::None;
                            yields { Target.Action::Noop; }
                        }
                        on Action::Continue { state_effect: StateEffect::None; }
                    }
                }
            }
            system Target {
                initial_state: State::Online;
                state State::Online {
                    actions {
                        on Action::Noop { state_effect: StateEffect::None; }
                    }
                }
            }
        """
        derivation, checked, _ = self.run_source(source, "Flow.Run")
        self.assertEqual(checked["verdict"], "complete", derivation.get("failure"))
        self.assertEqual(
            [(item["target"], item["delivery"], item["outcome"]) for item in derivation["signals"]],
            [("Flow", "root", "completed"), ("Target", "yields", "completed")],
        )
        self.assertFalse(
            any(event["kind"].startswith("emits_") for event in derivation["events"])
        )
        before = derivation["signals"][0]["before_snapshot"]
        after = derivation["signals"][0]["after_snapshot"]
        for field in ("states", "facts", "references", "contextual_bindings", "instances"):
            self.assertEqual(before[field], after[field], field)
        self.assertEqual(after["contextual_bindings"], {})
        self.assertEqual(after["yield_tokens"], {})
        consumed = next(
            event
            for event in derivation["events"]
            if event["kind"] == "yield_token_consumed"
        )
        token = consumed["token"]
        self.assertEqual(
            (token["outcome"], token["identity_return"], token["context_epoch"]),
            ("resumed", True, 0),
        )
        positions = {
            kind: next(
                event["sequence"]
                for event in derivation["events"]
                if event["kind"] == kind
            )
            for kind in (
                "yield_token_created",
                "yield_default_resume_attempt",
                "yield_token_resumed",
                "yield_token_consumed",
            )
        }
        target_completed = next(
            event["sequence"]
            for event in derivation["events"]
            if event["kind"] == "response_completed" and event["signal_id"] == "sig-0002"
        )
        self.assertLess(positions["yield_token_created"], target_completed)
        self.assertLess(target_completed, positions["yield_default_resume_attempt"])
        self.assertLess(positions["yield_default_resume_attempt"], positions["yield_token_resumed"])
        self.assertLess(positions["yield_token_resumed"], positions["yield_token_consumed"])
        serialized = json.dumps(after, sort_keys=True)
        for forbidden in ('"ra"', '"sp"', '"s0"', "CurrentTask", "CurrentStack", "runqueue"):
            self.assertNotIn(forbidden, serialized)

    def test_yields_resumes_exactly_once_through_contextual_enter(self) -> None:
        source = """
            predicate source_tail_completed<S>(source: S) -> bool;
            type TaskFlow: PhaseObject { }
            system FlowA: TaskFlow {
                initial_state: State::Online;
                state State::Online {
                    transitions {
                        on Transition::Park -> State::Ready { }
                    }
                    actions {
                        on Action::Run {
                            state_effect: StateEffect::None;
                            yields { Scheduler.Action::Schedule; }
                            ensures { source_tail_completed(self); }
                        }
                    }
                }
                state State::Ready {
                    transitions {
                        on Transition::Restore -> State::Online { }
                    }
                    actions {
                        on Action::Enter {
                            state_effect: StateEffect::None;
                            contextual_entry: true;
                            drives { FlowA.Transition::Restore; }
                        }
                    }
                }
            }
            system Scheduler {
                initial_state: State::Online;
                state State::Online {
                    actions {
                        on Action::Schedule {
                            state_effect: StateEffect::None;
                            drives { FlowA.Transition::Park; }
                        }
                    }
                }
            }
        """
        yielded, yielded_check, rendered = self.run_source(source, "FlowA.Run")
        self.assertEqual(yielded_check["verdict"], "yielded", rendered)
        self.assertEqual(
            [item["delivery"] for item in yielded["signals"]],
            ["root", "yields", "drives"],
        )
        dispatched = self.save_and_dispatch_yield_context(
            yielded["last_stable_snapshot"], "FlowA"
        )
        derivation, checked, rendered = self.run_source(
            source, "FlowA.Enter", scenario=dispatched
        )
        self.assertEqual(checked["verdict"], "complete", rendered)
        self.assertEqual(
            [item["delivery"] for item in derivation["signals"]],
            ["root", "drives"],
        )
        self.assertEqual(derivation["last_stable_snapshot"]["yield_tokens"], {})
        token = next(
            event["token"]
            for event in derivation["events"]
            if event["kind"] == "yield_token_consumed"
        )
        self.assertEqual(token["outcome"], "resumed")
        self.assertFalse(token["identity_return"])
        self.assertEqual(token["contextual_entry_occurrences"], 1)
        self.assertIn(
            "source_tail_completed(FlowA)", derivation["last_stable_snapshot"]["facts"]
        )
        self.assertEqual(
            sum(event["kind"] == "yield_token_resumed" for event in derivation["events"]),
            1,
        )

    def test_contextual_enter_consumes_declared_initial_handler_coordinate_once(self) -> None:
        source = """
            predicate kernel_init_body_running<F>(flow: F) -> bool;
            type TaskFlow: PhaseObject { }
            system Flow: TaskFlow {
                initial_context: Action::RunKernelInit;
                initial_state: State::Ready;
                state State::Ready {
                    transitions {
                        on Transition::Enable -> State::Online { }
                    }
                }
                state State::Online {
                    actions {
                        on Action::Enter {
                            state_effect: StateEffect::None;
                            contextual_entry: true;
                        }
                        on Action::RunKernelInit {
                            state_effect: StateEffect::None;
                            ensures { kernel_init_body_running(self); }
                        }
                    }
                }
            }
        """
        enabled, enabled_check, rendered = self.run_source(source, "Flow.Enable")
        self.assertEqual(enabled_check["verdict"], "complete", rendered)
        continuation = enabled["last_stable_snapshot"]["context_continuations"]["Flow"]
        self.assertEqual(
            (continuation["kind"], continuation["coordinate"]["action"]),
            ("handler", "RunKernelInit"),
        )

        dispatched = self.dispatch_context(enabled["last_stable_snapshot"], "Flow")
        first, first_check, rendered = self.run_source(
            source, "Flow.Enter", scenario=dispatched
        )
        self.assertEqual(first_check["verdict"], "complete", rendered)
        self.assertEqual(
            [(item["name"], item["delivery"]) for item in first["signals"]],
            [("Enter", "root"), ("RunKernelInit", "drives")],
        )
        self.assertTrue(first["signals"][0]["handler"]["contextual_entry"])
        self.assertIn(
            "kernel_init_body_running(Flow)",
            first["last_stable_snapshot"]["facts"],
        )

        duplicate, duplicate_check, _ = self.run_source(
            source, "Flow.Enter", scenario=first["last_stable_snapshot"]
        )
        self.assertEqual(duplicate_check["verdict"], "failed")
        self.assertIn("duplicate_contextual_enter", duplicate["failure"]["reason"])

    def test_context_entry_declaration_and_handler_constraints(self) -> None:
        cases = {
            "non-taskflow": (
                "system Flow { initial_context: Action::Boot;",
                "on Action::Boot { state_effect: StateEffect::None; }",
                "initial_context is allowed only on a TaskFlow declaration",
            ),
            "wrong-effect": (
                "type TaskFlow: PhaseObject { } system Flow: TaskFlow { initial_context: Action::Boot;",
                "on Action::Boot { state_effect: StateEffect::Conditional; }",
                "must target an Online-eligible StateEffect::None Action::Boot",
            ),
            "removed-handler-property": (
                "type TaskFlow: PhaseObject { } system Flow: TaskFlow {",
                "on Action::Boot { state_effect: StateEffect::None; initial_context_entry: true; }",
                "initial_context_entry has been removed; declare initial_context: Action::<Name>",
            ),
            "handler-property": (
                "type TaskFlow: PhaseObject { } system Flow: TaskFlow {",
                "on Action::Boot { state_effect: StateEffect::None; initial_context: Action::Boot; }",
                "initial_context is a TaskFlow declaration property, not a handler property",
            ),
            "missing-local-action": (
                "type TaskFlow: PhaseObject { } system Flow: TaskFlow { initial_context: Action::Missing;",
                "on Action::Boot { state_effect: StateEffect::None; }",
                "must resolve to exactly one Action::Missing owned by the same TaskFlow",
            ),
        }
        for name, (declaration, handler, message) in cases.items():
            with self.subTest(name=name):
                _derived, _checked, rendered = self.run_source(
                    f"""
                    {declaration}
                        initial_state: State::Online;
                        state State::Online {{ actions {{ {handler} }} }}
                    }}
                    """,
                    "Flow.Enter",
                )
                self.assertIn(message, rendered)

    def test_yields_generic_nonempty_target_and_nested_resume_order(self) -> None:
        source = """
            predicate a_tail_done() -> bool;
            predicate b_tail_done() -> bool;
            predicate c_body_done() -> bool;
            system FlowA {
                initial_state: State::Online;
                state State::Online {
                    actions {
                        on Action::Run {
                            state_effect: StateEffect::None;
                            yields { FlowB.Action::Run; }
                            ensures { a_tail_done(); }
                        }
                    }
                }
            }
            system FlowB {
                initial_state: State::Online;
                state State::Online {
                    actions {
                        on Action::Run {
                            state_effect: StateEffect::None;
                            yields { FlowC.Action::Run; }
                            ensures { b_tail_done(); }
                        }
                    }
                }
            }
            system FlowC {
                initial_state: State::Online;
                state State::Online {
                    actions {
                        on Action::Run {
                            state_effect: StateEffect::None;
                            ensures { c_body_done(); }
                        }
                    }
                }
            }
        """
        derivation, checked, rendered = self.run_source(source, "FlowA.Run")
        self.assertEqual(checked["verdict"], "complete", rendered)
        self.assertEqual(
            [(item["target"], item["delivery"], item["outcome"]) for item in derivation["signals"]],
            [
                ("FlowA", "root", "completed"),
                ("FlowB", "yields", "completed"),
                ("FlowC", "yields", "completed"),
            ],
        )
        completions = [
            event["signal_id"]
            for event in derivation["events"]
            if event["kind"] == "response_completed"
            and event["signal_id"] in {"sig-0001", "sig-0002", "sig-0003"}
        ]
        self.assertEqual(completions, ["sig-0003", "sig-0002", "sig-0001"])
        self.assertEqual(
            sum(event["kind"] == "yield_token_created" for event in derivation["events"]),
            2,
        )
        self.assertEqual(
            sum(event["kind"] == "yield_token_consumed" for event in derivation["events"]),
            2,
        )
        self.assertEqual(derivation["last_stable_snapshot"]["yield_tokens"], {})
        self.assertTrue(
            {"a_tail_done", "b_tail_done", "c_body_done"}.issubset(
                derivation["last_stable_snapshot"]["facts"]
            )
        )

    def test_yields_target_failure_and_duplicate_resume_are_terminal(self) -> None:
        target_failure = """
            predicate source_tail_done() -> bool;
            predicate child_allowed() -> bool;
            system Source {
                initial_state: State::Online;
                state State::Online {
                    actions {
                        on Action::Run {
                            state_effect: StateEffect::None;
                            yields { Target.Action::Work; }
                            ensures { source_tail_done(); }
                        }
                    }
                }
            }
            system Target {
                initial_state: State::Online;
                state State::Online {
                    actions {
                        on Action::Work {
                            state_effect: StateEffect::None;
                            drives { Child.Action::Fail; }
                        }
                    }
                }
            }
            system Child {
                initial_state: State::Online;
                state State::Online {
                    actions {
                        on Action::Fail {
                            state_effect: StateEffect::None;
                            depends_on { child_allowed(); }
                        }
                    }
                }
            }
        """
        failed, failed_check, _ = self.run_source(target_failure, "Source.Run")
        self.assertEqual(failed_check["verdict"], "failed")
        self.assertTrue(
            any(event["kind"] == "yield_token_created" for event in failed["events"])
        )
        self.assertFalse(
            any(event["kind"] == "yield_token_consumed" for event in failed["events"])
        )
        self.assertNotIn("source_tail_done", failed["last_stable_snapshot"]["facts"])

        duplicate = """
            type TaskFlow: PhaseObject {
                processes {
                    Action::Enter {
                        state_effect: StateEffect::None;
                        contextual_entry: true;
                    }
                }
            }
            system Source: TaskFlow {
                initial_state: State::Online;
                state State::Online {
                    actions {
                        on Action::Run {
                            state_effect: StateEffect::None;
                            yields { Target.Action::Work; }
                        }
                    }
                    transitions {
                        on Transition::Park -> State::Ready { }
                    }
                }
                state State::Ready {
                    transitions {
                        on Transition::Restore -> State::Online { }
                    }
                }
            }
            system Target {
                initial_state: State::Online;
                state State::Online {
                    actions {
                        on Action::Work {
                            state_effect: StateEffect::None;
                            drives { Source.Transition::Park; }
                        }
                    }
                }
            }
        """
        yielded, yielded_check, _ = self.run_source(duplicate, "Source.Run")
        self.assertEqual(yielded_check["verdict"], "yielded")
        dispatched = self.save_and_dispatch_yield_context(
            yielded["last_stable_snapshot"], "Source"
        )
        resumed, resumed_check, _ = self.run_source(
            duplicate, "Source.Enter", scenario=dispatched
        )
        self.assertEqual(resumed_check["verdict"], "complete", resumed.get("failure"))
        repeated, repeated_check, _ = self.run_source(
            duplicate, "Source.Enter", scenario=resumed["last_stable_snapshot"]
        )
        self.assertEqual(repeated_check["verdict"], "failed")
        self.assertIn("duplicate_contextual_enter", repeated["failure"]["reason"])

    def test_yield_token_snapshot_round_trip_and_terminal_resume_errors(self) -> None:
        source = """
            predicate source_tail_completed<S>(source: S) -> bool;
            type TaskFlow: PhaseObject { }
            system FlowA: TaskFlow {
                initial_state: State::Online;
                state State::Online {
                    transitions {
                        on Transition::Park -> State::Ready { }
                    }
                    actions {
                        on Action::Run {
                            state_effect: StateEffect::None;
                            yields { Scheduler.Action::Schedule; }
                            ensures { source_tail_completed(self); }
                        }
                    }
                }
                state State::Ready {
                    transitions {
                        on Transition::Restore -> State::Online { }
                    }
                    actions {
                        on Action::Enter {
                            state_effect: StateEffect::None;
                            contextual_entry: true;
                            drives { FlowA.Transition::Restore; }
                        }
                    }
                }
            }
            system Scheduler {
                initial_state: State::Online;
                state State::Online {
                    actions {
                        on Action::Schedule {
                            state_effect: StateEffect::None;
                            drives { FlowA.Transition::Park; }
                        }
                    }
                }
            }
        """
        yielded, yielded_check, _ = self.run_source(source, "FlowA.Run")
        self.assertEqual(
            (yielded["verdict"], yielded_check["allowed"]),
            ("yielded", True),
            yielded.get("failure"),
        )
        snapshot = self.save_and_dispatch_yield_context(
            yielded["last_stable_snapshot"], "FlowA"
        )
        token_id = next(iter(snapshot["yield_tokens"]))
        self.assertEqual(snapshot["yield_tokens"][token_id]["outcome"], "yielded")

        resumed, resumed_check, _ = self.run_source(
            source, "FlowA.Enter", scenario=snapshot
        )
        self.assertEqual(resumed_check["verdict"], "complete")
        self.assertEqual(resumed["last_stable_snapshot"]["yield_tokens"], {})
        self.assertIn(
            "source_tail_completed(FlowA)", resumed["last_stable_snapshot"]["facts"]
        )

        for container, field, value, expected in (
            ("record", "generation", 99, "context_continuation_generation_mismatch"),
            ("record", "cpu", "WrongCPU", "context_continuation_cpu_mismatch"),
            ("proof", "flow", "FlowB", "dispatch_proof_flow_mismatch"),
            ("proof", "context_epoch", 99, "dispatch_proof_context_epoch_mismatch"),
            ("proof", "dispatch_ordinal", 99, "dispatch_proof_dispatch_ordinal_mismatch"),
            ("proof", "task_ref", "WrongTaskRef", "dispatch_proof_task_ref_mismatch"),
        ):
            damaged = deepcopy(snapshot)
            record = damaged["context_continuations"]["FlowA"]
            target = record if container == "record" else record["dispatch_proof"]
            target[field] = value
            failed, failed_check, _ = self.run_source(
                source, "FlowA.Enter", scenario=damaged
            )
            self.assertEqual(failed_check["verdict"], "failed", (container, field))
            self.assertIn(expected, failed["failure"]["reason"], (container, field))

    def test_yields_model_constraints_and_precommit_rejection(self) -> None:
        cases = {
            "transition": (
                "on Transition::Run -> State::Ready",
                "state_effect: StateEffect::None; yields { Scheduler.Action::Schedule; }",
                "yields is allowed only in Action handlers",
            ),
            "state-effect": (
                "on Action::Run",
                "state_effect: StateEffect::Conditional; yields { Scheduler.Action::Schedule; }",
                "yields requires state_effect: StateEffect::None",
            ),
            "multiple": (
                "on Action::Run",
                "state_effect: StateEffect::None; yields { Scheduler.Action::Schedule; } yields { Scheduler.Action::Schedule; }",
                "at most one yields call",
            ),
        }
        for name, (declaration, body, message) in cases.items():
            with self.subTest(name=name):
                source = f"""
                    system Flow {{
                        initial_state: State::Base;
                        state State::Base {{ actions {{ {declaration} {{ {body} }} }} }}
                        state State::Ready {{ }}
                    }}
                    system Scheduler {{
                        initial_state: State::Online;
                        state State::Online {{
                            actions {{ on Action::Schedule {{ state_effect: StateEffect::None; }} }}
                        }}
                    }}
                """
                derivation, checked, rendered = self.run_source(source, "Flow.Run")
                self.assertEqual(checked["verdict"], "failed")
                self.assertIn(message, rendered)

        rejected_source = """
            predicate schedule_allowed() -> bool;
            system Flow {
                initial_state: State::Online;
                state State::Online {
                    actions {
                        on Action::Run {
                            state_effect: StateEffect::None;
                            yields { Scheduler.Action::Schedule; }
                        }
                    }
                }
            }
            system Scheduler {
                initial_state: State::Online;
                state State::Online {
                    actions {
                        on Action::Schedule {
                            state_effect: StateEffect::None;
                            depends_on { schedule_allowed(); }
                        }
                    }
                }
            }
        """
        rejected, rejected_check, _ = self.run_source(rejected_source, "Flow.Run")
        self.assertEqual(rejected_check["verdict"], "failed")
        self.assertEqual(len(rejected["signals"]), 1)
        self.assertEqual(rejected["last_stable_snapshot"]["yield_tokens"], {})
        self.assertEqual(rejected["last_stable_snapshot"]["task_flow_lanes"], {})

        postcommit_source = """
            system Flow {
                initial_state: State::Online;
                state State::Online {
                    actions {
                        on Action::Run {
                            state_effect: StateEffect::None;
                            yields { Scheduler.Action::Schedule; }
                        }
                    }
                }
            }
            system Missing {
                initial_state: State::Online;
                state State::Online { }
            }
            system Scheduler {
                initial_state: State::Online;
                state State::Online {
                    actions {
                        on Action::Schedule {
                            state_effect: StateEffect::None;
                            drives { Missing.Action::Continue; }
                        }
                    }
                }
            }
        """
        postcommit, postcommit_check, _ = self.run_source(postcommit_source, "Flow.Run")
        self.assertEqual(postcommit_check["verdict"], "failed")
        token = next(iter(postcommit["last_stable_snapshot"]["yield_tokens"].values()))
        self.assertEqual((token["status"], token["outcome"]), ("terminal-failed", "failed"))
        lane = next(iter(postcommit["last_stable_snapshot"]["task_flow_lanes"].values()))
        self.assertEqual(lane["state"], "terminal-failed")

    def test_handler_model_comments_are_display_metadata_through_view(self) -> None:
        source = """
            system Root {
                initial_state: State::Base;
                state State::Base {
                    transitions {
                        /*
                         * First close every branch gate.
                         * Then clear every pending signal.
                         */
                        on Transition::Start -> State::Ready { }
                    }
                }
                state State::Ready {
                    actions {
                        // Observe the stable result.
                        // Do not change lifecycle state.
                        on Action::Inspect { }
                    }
                }
            }
        """
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            spec = root / "comments.spec"
            work = root / "work"
            spec.write_text(textwrap.dedent(source), encoding="utf-8")
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(
                    driver_main(
                        [
                            str(spec),
                            "--signal",
                            "Root.Start",
                            "--work-dir",
                            str(work),
                        ]
                    ),
                    0,
                )
            ast = read_json(work / "ast.json")["document"]
            ast_handlers = {
                handler["name"]: handler
                for state in ast["systems"][0]["states"]
                for handler in state["handlers"]
            }
            self.assertEqual(
                ast_handlers["Start"]["description"],
                "First close every branch gate.\nThen clear every pending signal.",
            )
            self.assertEqual(
                ast_handlers["Inspect"]["description"],
                "Observe the stable result.\nDo not change lifecycle state.",
            )
            model = read_json(work / "model.json")["model"]
            self.assertEqual(
                model["systems"]["Root"]["states"]["Base"]["handlers"][0][
                    "description"
                ],
                ast_handlers["Start"]["description"],
            )
            derivation = read_json(work / "derive.json")
            view = read_json(work / "view.json")
            for payload in (derivation, view):
                self.assertEqual(
                    payload["signals"][0]["handler"]["description"],
                    ast_handlers["Start"]["description"],
                )

    def test_external_orchestration_drives_then_enqueues_emits_and_explicit_is_single(self) -> None:
        source = """
            external Human {
                drives {
                    Computer.Transition::Preset;
                    Computer.Transition::Setup;
                }
                emits {
                    Computer.Transition::Enable;
                }
            }

            system Computer {
                initial_state: State::Base;
                state State::Base {
                    transitions { on Transition::Preset -> State::Prepared { } }
                }
                state State::Prepared {
                    transitions { on Transition::Setup -> State::Ready { } }
                }
                state State::Ready {
                    transitions { on Transition::Enable -> State::Online { } }
                }
                state State::Online { }
            }
        """
        derivation, checked, _ = self.run_source(
            source, None, max_depth="all", max_breadth="all"
        )
        self.assertEqual(checked["verdict"], "complete")
        self.assertEqual(
            [
                (
                    item["source"],
                    item["target"],
                    item["name"],
                    item["delivery"],
                    item["cause_id"],
                )
                for item in derivation["signals"]
            ],
            [
                ("Human", "Computer", "Preset", "drives", None),
                ("Human", "Computer", "Setup", "drives", None),
                ("Human", "Computer", "Enable", "emits", None),
            ],
        )
        self.assertEqual(derivation["root_request"]["signal_id"], "sig-0001")
        enqueue = next(
            item["sequence"]
            for item in derivation["events"]
            if item["kind"] == "emits_enqueued" and item["child_id"] == "sig-0003"
        )
        dequeue = next(
            item["sequence"]
            for item in derivation["events"]
            if item["kind"] == "emits_dequeued" and item["signal_id"] == "sig-0003"
        )
        self.assertLess(enqueue, dequeue)

        explicit, explicit_checked, _ = self.run_source(source, "Computer.Preset")
        self.assertEqual(explicit_checked["verdict"], "complete")
        self.assertEqual(len(explicit["signals"]), 1)
        self.assertEqual(explicit["signals"][0]["delivery"], "root")
        self.assertEqual(explicit["last_stable_snapshot"]["states"]["Computer"], "Prepared")

    def test_external_synchronous_failure_short_circuits_later_drives_and_emits(self) -> None:
        source = """
            predicate preset_allowed() -> bool;
            external Human {
                drives {
                    Computer.Transition::Preset;
                    Computer.Transition::Setup;
                }
                emits {
                    Computer.Transition::Enable;
                }
            }
            system Computer {
                initial_state: State::Base;
                state State::Base {
                    transitions {
                        on Transition::Preset -> State::Prepared {
                            depends_on { preset_allowed(); }
                        }
                    }
                }
                state State::Prepared {
                    transitions { on Transition::Setup -> State::Ready { } }
                }
                state State::Ready {
                    transitions { on Transition::Enable -> State::Online { } }
                }
                state State::Online { }
            }
        """
        derivation, checked, _ = self.run_source(source, None)
        self.assertEqual(checked["verdict"], "failed")
        self.assertEqual(
            [(item["name"], item["outcome"]) for item in derivation["signals"]],
            [("Preset", "rejected")],
        )
        self.assertFalse(
            any(item["kind"] == "emits_enqueued" for item in derivation["events"])
        )

    def test_external_parse_and_semantic_errors_are_reported(self) -> None:
        cases = {
            "duplicate-block": (
                """
                    external Human {
                        drives { Computer.Transition::Preset; }
                        drives { Computer.Transition::Setup; }
                        emits { Computer.Transition::Enable; }
                    }
                """,
                "duplicate external drives block",
            ),
            "illegal-member": (
                """
                    external Human {
                        depends_on { allowed(); }
                        drives { Computer.Transition::Preset; }
                        emits { Computer.Transition::Enable; }
                    }
                """,
                "external members must be drives or emits blocks",
            ),
        }
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for name, (source, expected) in cases.items():
                spec = root / f"{name}.spec"
                ast = root / f"{name}.ast.json"
                spec.write_text(textwrap.dedent(source), encoding="utf-8")
                self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
                messages = [
                    item["message"] for item in read_json(ast)["document"]["diagnostics"]
                ]
                self.assertTrue(any(expected in message for message in messages), messages)

            semantic = root / "semantic.spec"
            semantic_ast = root / "semantic.ast.json"
            semantic_model = root / "semantic.model.json"
            semantic.write_text(
                textwrap.dedent(
                    """
                    external Human {
                        drives { Missing.Transition::Preset; }
                        emits { Computer.Transition::Enable; }
                    }
                    external Operator {
                        drives { Computer.Transition::Preset; }
                        emits { Computer.Transition::Enable; }
                    }
                    system Human {
                        initial_state: State::Base;
                        state State::Base { }
                    }
                    system Computer {
                        initial_state: State::Base;
                        state State::Base {
                            transitions { on Transition::Preset -> State::Ready { } }
                        }
                        state State::Ready {
                            transitions { on Transition::Enable -> State::Online { } }
                        }
                        state State::Online { }
                    }
                    """
                ),
                encoding="utf-8",
            )
            self.assertEqual(parse_main([str(semantic), "-o", str(semantic_ast)]), 0)
            self.assertEqual(
                model_main([str(semantic_ast), "-o", str(semantic_model)]), 0
            )
            model_data = read_json(semantic_model)
            self.assertFalse(model_data["summary"]["ok"])
            messages = [item["message"] for item in model_data["diagnostics"]]
            self.assertTrue(any("only one external" in message for message in messages))
            self.assertTrue(any("conflicts with system Human" in message for message in messages))
            self.assertTrue(any("unknown receiver Missing" in message for message in messages))

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
            "boundaries: deferred=0 trimmed=0 occurrences=0 obligations=0\n"
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

    def test_multilevel_lifecycle_contributions_keep_guard_drive_fact_and_emit_order(self) -> None:
        derivation, checked, _ = self.run_source(
            """
            type Leaf {
                initial_state: State::Base;
                state State::Base {
                    transitions { on Transition::Preset -> State::Ready { } }
                }
                state State::Ready { }
            }
            type BaseFlow {
                lifecycle {
                    Transition::Preset {
                        state_effect: StateEffect::Always;
                        depends_on { base_guard(self); }
                        drives { BaseLeaf.Transition::Preset; }
                        ensures { base_done(self); }
                        emits { Transition::Setup; }
                    }
                    Transition::Setup {
                        state_effect: StateEffect::Always;
                    }
                }
            }
            type DerivedFlow: BaseFlow {
                lifecycle {
                    Transition::Preset {
                        state_effect: StateEffect::Always;
                        depends_on { derived_guard(self); }
                        drives { DerivedLeaf.Transition::Preset; }
                        ensures { derived_done(self); }
                    }
                }
            }
            object BaseLeaf: Leaf { }
            object DerivedLeaf: Leaf { }
            object InstanceLeaf: Leaf { }
            object Root: DerivedFlow {
                initial_state: State::Base;
                state State::Base {
                    invariant {
                        base_guard(self);
                        derived_guard(self);
                        instance_guard(self);
                    }
                    transitions {
                        on Transition::Preset -> State::Prepared {
                            depends_on { instance_guard(self); }
                            drives { InstanceLeaf.Transition::Preset; }
                            ensures { instance_done(self); }
                        }
                    }
                }
                state State::Prepared {
                    invariant {
                        base_done(self);
                        derived_done(self);
                        instance_done(self);
                    }
                    transitions { on Transition::Setup -> State::Ready { } }
                }
                state State::Ready { }
            }
            """,
            "Root.Preset",
            max_depth="all",
            max_breadth="all",
        )
        self.assertEqual(checked["verdict"], "complete")
        self.assertEqual(
            [
                (item["target"], item["name"], item["delivery"], item["outcome"])
                for item in derivation["signals"]
            ],
            [
                ("Root", "Preset", "root", "completed"),
                ("BaseLeaf", "Preset", "drives", "completed"),
                ("DerivedLeaf", "Preset", "drives", "completed"),
                ("InstanceLeaf", "Preset", "drives", "completed"),
                ("Root", "Setup", "emits", "completed"),
            ],
        )
        self.assertEqual(
            [
                item["owner"]
                for item in derivation["signals"][0]["handler"][
                    "composed_type_processes"
                ]
            ],
            ["BaseFlow", "DerivedFlow"],
        )
        self.assertEqual(
            derivation["last_stable_snapshot"]["states"]["Root"], "Ready"
        )
        self.assertTrue(
            {
                "base_done(Root)",
                "derived_done(Root)",
                "instance_done(Root)",
            }.issubset(derivation["last_stable_snapshot"]["facts"])
        )
        root_response = next(
            item["sequence"]
            for item in derivation["events"]
            if item["kind"] == "response_completed" and item["signal_id"] == "sig-0001"
        )
        setup_enqueue = next(
            item["sequence"]
            for item in derivation["events"]
            if item["kind"] == "emits_enqueued" and item["signal_id"] == "sig-0001"
        )
        self.assertLess(root_response, setup_enqueue)

    def test_duplicate_inherited_side_effect_is_a_tools2_model_error(self) -> None:
        source = """
            type BaseFlow {
                lifecycle {
                    Transition::Preset {
                        state_effect: StateEffect::Always;
                        emits { Transition::Setup; }
                    }
                    Transition::Setup { state_effect: StateEffect::Always; }
                }
            }
            type DerivedFlow: BaseFlow {
                lifecycle {
                    Transition::Preset {
                        state_effect: StateEffect::Always;
                        emits { Transition::Setup; }
                    }
                }
            }
            object Root: DerivedFlow {
                initial_state: State::Base;
                state State::Base {
                    transitions { on Transition::Preset -> State::Prepared { } }
                }
                state State::Prepared {
                    transitions { on Transition::Setup -> State::Ready { } }
                }
                state State::Ready { }
            }
        """
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            spec = root / "duplicate.spec"
            ast = root / "ast.json"
            model = root / "model.json"
            spec.write_text(textwrap.dedent(source), encoding="utf-8")
            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            self.assertEqual(model_main([str(ast), "-o", str(model)]), 0)
            modeled = read_json(model)
            self.assertFalse(modeled["summary"]["ok"])
            self.assertTrue(
                any(
                    "duplicate inherited emits entry" in item["message"]
                    for item in modeled["diagnostics"]
                )
            )

    def test_explicit_type_state_transition_contributes_source_target_and_body(self) -> None:
        derivation, checked, _ = self.run_source(
            """
            type BaseCarrier {
                initial_state: State::Base;
                invariant { base_guard(self); }
                state State::Base {
                    transitions {
                        on Transition::Preset -> State::Ready {
                            depends_on { base_guard(self); }
                            ensures { base_done(self); }
                        }
                    }
                }
                state State::Ready { invariant { base_done(self); derived_done(self); } }
            }
            type DerivedCarrier: BaseCarrier {
                invariant { derived_guard(self); }
                lifecycle {
                    Transition::Preset {
                        state_effect: StateEffect::Always;
                        depends_on { derived_guard(self); }
                        ensures { derived_done(self); }
                    }
                }
            }
            object Root: DerivedCarrier { }
            """,
            "Root.Preset",
        )
        self.assertEqual(checked["verdict"], "complete")
        root = derivation["signals"][0]
        self.assertEqual(
            (root["handler"]["source_state"], root["handler"]["target_state"]),
            ("Base", "Ready"),
        )
        self.assertEqual(
            [item["owner"] for item in root["handler"]["composed_type_processes"]],
            ["BaseCarrier", "DerivedCarrier"],
        )
        self.assertTrue(
            {"base_done(Root)", "derived_done(Root)"}.issubset(
                derivation["last_stable_snapshot"]["facts"]
            )
        )

        conflicting = """
            type BaseCarrier {
                initial_state: State::Base;
                state State::Base {
                    transitions { on Transition::Preset -> State::Ready { } }
                }
                state State::Ready { }
            }
            type DerivedCarrier: BaseCarrier {
                initial_state: State::Base;
                state State::Base {
                    transitions { on Transition::Preset -> State::Prepared { } }
                }
                state State::Prepared { }
            }
            object Root: DerivedCarrier { }
        """
        with tempfile.TemporaryDirectory() as tmp:
            root_dir = Path(tmp)
            spec = root_dir / "conflict.spec"
            ast = root_dir / "ast.json"
            model = root_dir / "model.json"
            spec.write_text(textwrap.dedent(conflicting), encoding="utf-8")
            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            self.assertEqual(model_main([str(ast), "-o", str(model)]), 0)
            modeled = read_json(model)
            self.assertTrue(
                any(
                    item["message"]
                    == "conflicting inherited target state: DerivedCarrier.Transition::Preset"
                    for item in modeled["diagnostics"]
                )
            )

    def test_lifecycle_override_requires_and_uses_a_complete_replacement(self) -> None:
        complete = """
            type Carrier {
                initial_state: State::Base;
                state State::Base {
                    transitions { on Transition::Preset -> State::Ready { } }
                }
                state State::Ready { }
            }
            object Root: Carrier {
                lifecycle_override: true;
                initial_state: State::Online;
                state State::Online {
                    actions { on Action::Inspect { ensures { override_used(self); } } }
                }
            }
        """
        derivation, checked, _ = self.run_source(complete, "Root.Inspect")
        self.assertEqual(checked["verdict"], "complete")
        self.assertEqual(set(derivation["initial_snapshot"]["states"].values()), {"Online"})
        self.assertIn(
            "override_used(Root)", derivation["last_stable_snapshot"]["facts"]
        )

        partial = """
            type Carrier {
                initial_state: State::Base;
                state State::Base {
                    transitions { on Transition::Preset -> State::Ready { } }
                }
                state State::Ready { }
            }
            object Root: Carrier {
                lifecycle_override: true;
                initial_state: State::Online;
            }
        """
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            spec = root / "partial.spec"
            ast = root / "ast.json"
            model = root / "model.json"
            spec.write_text(textwrap.dedent(partial), encoding="utf-8")
            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            self.assertEqual(model_main([str(ast), "-o", str(model)]), 0)
            modeled = read_json(model)
            self.assertFalse(modeled["summary"]["ok"])
            self.assertTrue(
                any(
                    item["message"]
                    == "lifecycle_override on Root requires both initial_state and states"
                    for item in modeled["diagnostics"]
                )
            )

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

            for old_version in range(1, 10):
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
                self.assertIn("version=10", stderr.getvalue())

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
            self.assertIn("error:", result.stderr)

    def test_boundary_inventory_occurrences_and_obligations_are_noncausal(self) -> None:
        derivation, checked, text = self.run_source(
            """
            type ProbeType {
                processes {
                    Action::Inspect {
                        ensures { probe_evidence(self); }
                        deferred probe.001 {
                            category: DeferredCategory::Proof;
                            summary: "Proved evidence remains a read-only observation.";
                            evidence { probe_evidence(self); }
                            close_when: "The proof boundary is replaced by a permanent invariant.";
                        }
                        within AuditContext {
                            deferred probe.002 {
                                category: DeferredCategory::Proof;
                                summary: "Missing evidence produces an obligation.";
                                evidence { missing_probe_evidence(self); }
                                close_when: "The missing evidence is formally established and tested.";
                            }
                        }
                    }
                }
            }
            context AuditContext { }
            system Root {
                initial_state: State::Base;
                state State::Base {
                    transitions {
                        on Transition::Start -> State::Ready {
                            drives {
                                Probe.Action::Inspect;
                                Probe.Action::Inspect;
                            }
                            ensures { root_ready(self); }
                        }
                    }
                }
                state State::Ready { invariant { root_ready(self); } }
            }
            object Probe: ProbeType {
                parent: Root;
                initial_state: State::Base;
                state State::Base { }
            }
            """,
            "Root.Start",
            max_depth="all",
            max_breadth="all",
        )
        self.assertEqual(derivation["verdict"], "complete")
        self.assertFalse(checked["allowed"])
        self.assertEqual(checked["exit_code"], 1)
        self.assertEqual(derivation["summary"]["inventory_deferred"], 2)
        self.assertEqual(derivation["summary"]["inventory_trimmed"], 0)
        self.assertEqual(derivation["summary"]["boundary_occurrences"], 4)
        self.assertEqual(derivation["summary"]["unresolved_obligations"], 2)
        self.assertEqual(len(derivation["boundary_inventory"]), 2)
        self.assertEqual(
            {item["owner"] for item in derivation["boundary_inventory"]}, {"ProbeType"}
        )
        self.assertEqual(
            {item["owner"] for item in derivation["boundary_occurrences"]}, {"Probe"}
        )
        self.assertEqual(
            [(item["boundary_id"], item["occurrence_index"]) for item in derivation["boundary_occurrences"]],
            [("probe.001", 1), ("probe.002", 1), ("probe.001", 2), ("probe.002", 2)],
        )
        self.assertTrue(
            all(
                occurrence["proofs"][0]["result"]
                for occurrence in derivation["boundary_occurrences"]
                if occurrence["boundary_id"] == "probe.001"
            )
        )
        self.assertTrue(
            all(
                not occurrence["proofs"][0]["result"]
                for occurrence in derivation["boundary_occurrences"]
                if occurrence["boundary_id"] == "probe.002"
            )
        )
        self.assertEqual([item["outcome"] for item in derivation["signals"]], ["completed"] * 3)
        self.assertEqual(len(derivation["signals"]), 3)
        self.assertFalse(
            any(
                fact.startswith("missing_probe_evidence(")
                for fact in derivation["last_stable_snapshot"]["facts"]
            )
        )
        self.assertNotIn("boundary_obligation", {item["outcome"] for item in derivation["signals"]})
        self.assertIn("obligations=2", text)

    def test_declaration_state_and_transition_boundaries_use_candidate_snapshots(self) -> None:
        derivation, checked, _ = self.run_source(
            """
            system Root {
                initial_state: State::Base;
                facts { root_declared(self); }
                deferred boundary_location.001 {
                    category: DeferredCategory::Proof;
                    summary: "Declaration evidence is checked when the instance exists.";
                    evidence { root_declared(self); }
                    close_when: "The declaration fact becomes a permanent model invariant.";
                }
                state State::Base {
                    trimmed boundary_location.002 {
                        category: TrimmedCategory::ReferenceInput;
                        summary: "The initial reference input selects Base.";
                        evidence { self.state == State::Base; }
                        revisit_when: "The reference input selects another initial state.";
                    }
                    transitions {
                        on Transition::Start -> State::Ready {
                            ensures { root_ready(self); }
                            deferred boundary_location.003 {
                                category: DeferredCategory::Proof;
                                summary: "Transition evidence sees earlier candidate effects.";
                                evidence { root_ready(self); }
                                close_when: "The transition proof is a permanent postcondition.";
                            }
                        }
                    }
                }
                state State::Ready {
                    deferred boundary_location.004 {
                        category: DeferredCategory::Proof;
                        summary: "State evidence is checked on entry.";
                        evidence { root_ready(self); }
                        close_when: "The state proof is a permanent invariant.";
                    }
                    invariant { root_ready(self); }
                }
            }
            """,
            "Root.Start",
        )
        self.assertTrue(checked["allowed"])
        self.assertEqual(derivation["verdict"], "complete")
        self.assertEqual(derivation["summary"]["inventory_deferred"], 3)
        self.assertEqual(derivation["summary"]["inventory_trimmed"], 1)
        self.assertEqual(derivation["summary"]["boundary_occurrences"], 4)
        self.assertEqual(derivation["obligations"], [])
        self.assertEqual(
            [item["boundary_id"] for item in derivation["boundary_occurrences"]],
            [
                "boundary_location.001",
                "boundary_location.002",
                "boundary_location.003",
                "boundary_location.004",
            ],
        )
        self.assertEqual(
            [item["location"] for item in derivation["boundary_occurrences"]],
            ["declaration", "state", "transition", "state"],
        )
        self.assertIsNone(derivation["boundary_occurrences"][0]["signal_id"])
        self.assertEqual(derivation["boundary_occurrences"][-1]["state"], "Ready")

    def test_boundary_structure_is_strict_and_inventory_is_unique(self) -> None:
        invalid_members = {
            "invalid id": "deferred legacy_id",
            "invalid category": "deferred strict.001",
            "missing summary": "deferred strict.002",
            "empty evidence": "deferred strict.003",
            "wrong resolution": "trimmed strict.004",
        }
        blocks = {
            "invalid id": """
                { category: DeferredCategory::Proof; summary: "x";
                  evidence { known(self); } close_when: "closed"; }
            """,
            "invalid category": """
                { category: TrimmedCategory::BuildConfig; summary: "x";
                  evidence { known(self); } close_when: "closed"; }
            """,
            "missing summary": """
                { category: DeferredCategory::Proof;
                  evidence { known(self); } close_when: "closed"; }
            """,
            "empty evidence": """
                { category: DeferredCategory::Proof; summary: "x";
                  evidence { } close_when: "closed"; }
            """,
            "wrong resolution": """
                { category: TrimmedCategory::BuildConfig; summary: "x";
                  evidence { known(self); } close_when: "closed"; }
            """,
        }
        for label, declaration in invalid_members.items():
            with self.subTest(label=label), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                spec = root / "invalid.spec"
                ast = root / "ast.json"
                model = root / "model.json"
                spec.write_text(
                    textwrap.dedent(
                        f"""
                        system Root {{
                            initial_state: State::Base;
                            state State::Base {{
                                actions {{
                                    on Action::Inspect {{
                                        {declaration} {blocks[label]}
                                    }}
                                }}
                            }}
                        }}
                        """
                    ),
                    encoding="utf-8",
                )
                self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
                self.assertEqual(model_main([str(ast), "-o", str(model)]), 0)
                self.assertFalse(read_json(model)["summary"]["ok"])

        duplicate_source = """
            system Root {
                initial_state: State::Base;
                state State::Base {
                    actions {
                        on Action::Inspect {
                            deferred duplicate.001 {
                                category: DeferredCategory::Proof;
                                category: DeferredCategory::Proof;
                                summary: "x";
                                evidence { known(self); }
                                evidence { known(self); }
                                close_when: "closed";
                            }
                            deferred duplicate.001 {
                                category: DeferredCategory::Proof;
                                summary: "y";
                                evidence { known(self); }
                                close_when: "closed";
                            }
                        }
                    }
                }
            }
        """
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            spec = root / "duplicate.spec"
            ast = root / "ast.json"
            model = root / "model.json"
            spec.write_text(textwrap.dedent(duplicate_source), encoding="utf-8")
            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            parsed = read_json(ast)
            self.assertTrue(any("duplicate deferred" in item["message"] for item in parsed["document"]["diagnostics"]))
            self.assertEqual(model_main([str(ast), "-o", str(model)]), 0)
            modeled = read_json(model)
            self.assertEqual(len(modeled["model"]["boundary_inventory"]), 1)
            self.assertTrue(any("duplicate boundary id" in item["message"] for item in modeled["diagnostics"]))

    def test_obligation_blocks_snapshot_create_or_overwrite_and_zero_allows_v10(self) -> None:
        template = """
            system Root {{
                initial_state: State::Base;
                {facts}
                state State::Base {{
                    actions {{
                        on Action::Inspect {{
                            deferred snapshot_gate.001 {{
                                category: DeferredCategory::Proof;
                                summary: "Snapshot export requires this evidence.";
                                evidence {{ snapshot_gate_ready(self); }}
                                close_when: "The evidence is a permanent checked invariant.";
                            }}
                        }}
                    }}
                }}
            }}
        """
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            blocked_spec = root / "blocked.spec"
            blocked_snapshot = root / "blocked.snapshot.json"
            blocked_snapshot.write_text("sentinel\n", encoding="utf-8")
            blocked_spec.write_text(
                textwrap.dedent(template.format(facts="")), encoding="utf-8"
            )
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(
                    driver_main(
                        [
                            str(blocked_spec), "--signal", "Root.Inspect",
                            "--snapshot-out", str(blocked_snapshot),
                            "--work-dir", str(root / "blocked-work"),
                        ]
                    ),
                    1,
                )
            self.assertEqual(blocked_snapshot.read_text(encoding="utf-8"), "sentinel\n")
            blocked_check = read_json(root / "blocked-work" / "check.json")
            self.assertEqual(blocked_check["verdict"], "complete")
            self.assertFalse(blocked_check["allowed"])

            allowed_spec = root / "allowed.spec"
            allowed_snapshot = root / "allowed.snapshot.json"
            allowed_spec.write_text(
                textwrap.dedent(
                    template.format(facts="facts { snapshot_gate_ready(self); }")
                ),
                encoding="utf-8",
            )
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(
                    driver_main(
                        [
                            str(allowed_spec), "--signal", "Root.Inspect",
                            "--snapshot-out", str(allowed_snapshot),
                            "--work-dir", str(root / "allowed-work"),
                        ]
                    ),
                    0,
                )
            saved = read_json(allowed_snapshot)
            self.assertEqual(saved["version"], SNAPSHOT_VERSION)
            self.assertEqual(saved["version"], 10)
            self.assertTrue(read_json(root / "allowed-work" / "check.json")["allowed"])

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

    def test_every_tools2_consumer_rejects_pre_v10_protocols(self) -> None:
        cases = [
            (model_main, AST_SCHEMA, []),
            (derive_main, MODEL_SCHEMA, ["--signal", "Root.Go"]),
            (check_main, DERIVE_SCHEMA, []),
            (view_main, DERIVE_SCHEMA, []),
            (render_main, VIEW_SCHEMA, []),
        ]
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for old_version in range(1, 10):
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
                    self.assertIn("version=10", stderr.getvalue())

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

    def test_failed_emitted_successor_does_not_roll_back_committed_source(self) -> None:
        derivation, checked, _ = self.run_source(
            """
            system Root {
                initial_state: State::Base;
                state State::Base {
                    transitions {
                        on Transition::Start -> State::Online {
                            emits { Successor.Action::Commit; }
                            ensures { root_online(self); }
                        }
                    }
                }
                state State::Online { invariant { root_online(self); } }
            }
            system Successor {
                parent: Root;
                initial_state: State::Base;
                state State::Base { }
            }
            """,
            "Root.Start",
        )
        self.assertEqual(checked["verdict"], "failed")
        self.assertEqual(derivation["verdict"], "failed")
        self.assertEqual(derivation["signals"][0]["outcome"], "completed")
        self.assertEqual(derivation["signals"][1]["outcome"], "rejected")
        self.assertEqual(
            derivation["last_stable_snapshot"]["states"]["Root"], "Online"
        )
        self.assertIn(
            "root_online(Root)", derivation["last_stable_snapshot"]["facts"]
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
            text.splitlines()[2],
            "Human -- Startup --> Root[Ready:Ready] !! rejected: "
            "state_not_accepted: expected State::Base, got State::Ready",
        )

    def test_async_condition_rejection_fails_root_and_never_stays_pending(self) -> None:
        derivation, checked, text = self.run_source(
            """
            system Root {
                initial_state: State::Base;
                state State::Base {
                    transitions {
                        on Transition::Start -> State::Ready {
                            emits { Child.Action::Try; }
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
        self.assertEqual(checked["verdict"], "failed")
        self.assertEqual(derivation["signals"][1]["outcome"], "rejected")
        self.assertIn("condition_not_satisfied", derivation["signals"][1]["reason"])
        self.assertEqual(derivation["summary"]["pending"], 0)
        self.assertNotIn("pending", {item["outcome"] for item in derivation["signals"]})
        self.assertIn("!! rejected: condition_not_satisfied", text)

    def test_lossy_signal_syntax_is_rejected_by_v10_model(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            spec = root / "input.spec"
            ast = root / "ast.json"
            model = root / "model.json"
            spec.write_text(
                textwrap.dedent(
                    """
                    system Root {
                        initial_state: State::Base;
                        state State::Base {
                            actions { on Action::Start { emits { lossy Root.Action::Start; } } }
                        }
                    }
                    """
                ),
                encoding="utf-8",
            )
            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            self.assertEqual(model_main([str(ast), "-o", str(model)]), 0)
            diagnostics = read_json(model)["diagnostics"]
            self.assertTrue(diagnostics)
            self.assertTrue(any("invalid process call" in item["message"] for item in diagnostics))

    def test_on_cpu_lifecycle_is_task_only(self) -> None:
        def diagnostics_for(source: str) -> list[dict]:
            temporary = tempfile.TemporaryDirectory()
            self.addCleanup(temporary.cleanup)
            root = Path(temporary.name)
            spec = root / "input.spec"
            ast = root / "ast.json"
            model = root / "model.json"
            spec.write_text(textwrap.dedent(source), encoding="utf-8")
            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            self.assertEqual(model_main([str(ast), "-o", str(model)]), 0)
            return read_json(model)["diagnostics"]

        valid = diagnostics_for(
            """
            type Task { }
            object Worker: Task {
                initial_state: State::OnCpu;
                state State::OnCpu {
                    transitions { on Transition::Suspend -> State::Online { } }
                }
                state State::Online {
                    transitions { on Transition::Dispatch -> State::OnCpu { } }
                }
            }
            """
        )
        self.assertFalse(valid)

        invalid = diagnostics_for(
            """
            type PhaseObject { }
            object Phase: PhaseObject {
                initial_state: State::OnCpu;
                state State::OnCpu {
                    transitions { on Transition::Suspend -> State::Online { } }
                }
                state State::Online {
                    transitions { on Transition::Dispatch -> State::OnCpu { } }
                }
            }
            """
        )
        messages = [item["message"] for item in invalid]
        self.assertTrue(any("Task-only lifecycle state on non-Task" in item for item in messages))
        self.assertTrue(any("Task-only lifecycle transition on non-Task" in item for item in messages))

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

    def test_indexed_owned_declaration_is_canonical_and_snapshot_resumable(self) -> None:
        source = """
            type LogicId { }
            type CPU {
                initial_state: State::Base;
                state State::Base {
                    transitions {
                        on Transition::Preset -> State::Prepared {
                            ensures { cpu_ready(self); }
                        }
                    }
                }
                state State::Prepared {
                    actions { on Action::Inspect { depends_on { cpu_ready(self); } } }
                }
            }
            type CpuGroupObject {
                owned { indexed cpus[key: LogicId]: CPU; }
                initial_state: State::Base;
                state State::Base {
                    transitions {
                        on Transition::Preset -> State::Prepared {
                            drives {
                                declare self.cpus[0] of CPU;
                                self.cpus[0].Transition::Preset;
                            }
                            ensures { cpu_ref_targets(BootCPURef, self.cpus[0]); }
                        }
                    }
                }
                state State::Prepared { }
            }
            object CpuGroup: CpuGroupObject { }
        """
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            spec = root / "indexed.spec"
            ast = root / "ast.json"
            model = root / "model.json"
            first = root / "first.json"
            scenario = root / "snapshot.json"
            resumed = root / "resumed.json"
            spec.write_text(textwrap.dedent(source), encoding="utf-8")
            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            self.assertEqual(model_main([str(ast), "-o", str(model)]), 0)
            self.assertEqual(
                derive_main(
                    [
                        str(model), "--signal", "CpuGroup.Preset", "--max-depth", "all",
                        "--max-breadth", "all", "-o", str(first),
                    ]
                ),
                0,
            )
            data = read_json(first)
            self.assertEqual(data["verdict"], "complete")
            stable = data["last_stable_snapshot"]
            self.assertEqual(stable["states"]["CpuGroup.cpus[0]"], "Prepared")
            self.assertEqual(stable["references"]["CpuGroup.cpus[0]"], "CpuGroup.cpus[0]")
            self.assertEqual(
                stable["instances"]["CpuGroup.cpus[0]"],
                {
                    "declared_type": "CPU",
                    "indexed": True,
                    "key": 0,
                    "owned_field": "cpus",
                    "parent": "CpuGroup",
                    "span": stable["instances"]["CpuGroup.cpus[0]"]["span"],
                },
            )
            self.assertEqual(data["signals"][1]["target"], "CpuGroup.cpus[0]")
            self.assertEqual(
                [
                    event["identity"]
                    for event in data["events"]
                    if event["kind"] == "indexed_instance_declared"
                ],
                ["CpuGroup.cpus[0]"],
            )

            model_data = read_json(model)
            scenario.write_text(
                json.dumps(
                    {
                        "schema": SNAPSHOT_SCHEMA,
                        "version": SNAPSHOT_VERSION,
                        "producer": PRODUCER,
                        "source": str(spec),
                        "model_fingerprint": model_data["model_fingerprint"],
                        "snapshot": stable,
                    }
                ),
                encoding="utf-8",
            )
            self.assertEqual(
                derive_main(
                    [
                        str(model), "--signal", "CpuGroup.cpus[0].Inspect",
                        "--scenario", str(scenario), "-o", str(resumed),
                    ]
                ),
                0,
            )
            resumed_data = read_json(resumed)
            self.assertEqual(resumed_data["verdict"], "complete")
            self.assertEqual(resumed_data["signals"][0]["target"], "CpuGroup.cpus[0]")

    def test_dynamic_occurrence_generation_parent_and_stale_reference_are_enforced(self) -> None:
        source = """
            type TrapResourceType: ResourceObject {
                initial_state: State::Online;
                state State::Online { }
            }
            type TrapFlowRef {
                processes {
                    Action::Bind(flow: TrapFlowType) {
                        state_effect: StateEffect::None;
                        ensures {
                            trap_flow_ref_targets(self, flow);
                            trap_flow_ref_generation_valid(self);
                        }
                    }
                    Action::Inspect {
                        state_effect: StateEffect::None;
                        depends_on { trap_flow_ref_generation_valid(self); }
                    }
                }
            }
            type TrapFlowType: FlowObject {
                parent: TrapResourceType;
                initial_state: State::Base;
                processes {
                    Action::Bind(parent_trap: TrapResourceType, root_ref: TrapFlowRef) {
                        state_effect: StateEffect::None;
                        structural_binding: true;
                        ensures {
                            trap_flow_parent_is(self, parent_trap);
                            trap_flow_ref_targets(root_ref, self);
                            trap_flow_ref_generation_valid(root_ref);
                        }
                    }
                }
                state State::Base {
                    transitions {
                        on Transition::Preset -> State::Prepared {
                            depends_on {
                                trap_flow_occurrence_fresh(self);
                                trap_flow_generation_nonzero(self);
                            }
                        }
                    }
                }
                state State::Prepared {
                    transitions { on Transition::Setup -> State::Ready { } }
                }
                state State::Ready {
                    transitions { on Transition::Enable -> State::Online { } }
                }
                state State::Online {
                    transitions { on Transition::Disable -> State::Offline { } }
                }
                state State::Offline {
                    transitions { on Transition::Cleanup -> State::Destroyed { } }
                }
                state State::Destroyed { }
            }
            type HarnessType: FlowObject {
                associations { mutable saved_ref: TrapFlowRef; }
                initial_state: State::Base;
                state State::Base {
                    transitions {
                        on Transition::Create -> State::Ready {
                            drives {
                                declare root_flow of TrapFlowType;
                                declare root_ref of TrapFlowRef;
                                root_flow.Action::Bind(
                                    parent_trap: TrapResource,
                                    root_ref: root_ref
                                );
                                root_ref.Action::Bind(flow: root_flow);
                                root_flow.Transition::Preset;
                                root_flow.Transition::Setup;
                                root_flow.Transition::Enable;
                                root_flow.Transition::Disable;
                                root_flow.Transition::Cleanup;
                            }
                            updates { self.saved_ref = root_ref; }
                            ensures {
                                task_breakpoint_flow_ref_generation_valid(self);
                            }
                        }
                    }
                }
                state State::Ready {
                    actions {
                        on Action::InspectAggregate {
                            depends_on {
                                task_breakpoint_flow_ref_generation_valid(self);
                            }
                        }
                        on Action::InspectStale {
                            drives { self.saved_ref.Action::Inspect; }
                        }
                    }
                }
            }
            object TrapResource: TrapResourceType { }
            object Harness: HarnessType { }

            predicate trap_flow_ref_targets<R: TrapFlowRef, F: TrapFlowType>(reference: R, flow: F) -> bool;
            predicate trap_flow_ref_generation_valid<R: TrapFlowRef>(reference: R) -> bool;
            predicate trap_flow_parent_is<F: TrapFlowType, T: TrapResourceType>(flow: F, trap: T) -> bool;
            predicate trap_flow_occurrence_fresh<F: TrapFlowType>(flow: F) -> bool;
            predicate trap_flow_generation_nonzero<F: TrapFlowType>(flow: F) -> bool;
            predicate task_breakpoint_flow_ref_generation_valid<T: HarnessType>(task: T) -> bool;
        """
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            spec = root / "occurrence.spec"
            ast = root / "ast.json"
            model = root / "model.json"
            created = root / "created.json"
            scenario = root / "created.snapshot.json"
            aggregate = root / "aggregate.json"
            stale = root / "stale.json"
            spec.write_text(textwrap.dedent(source), encoding="utf-8")
            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            self.assertEqual(model_main([str(ast), "-o", str(model)]), 0)
            self.assertEqual(
                derive_main(
                    [
                        str(model), "--signal", "Harness.Create", "--max-depth", "all",
                        "--max-breadth", "all", "-o", str(created),
                    ]
                ),
                0,
            )
            data = read_json(created)
            self.assertEqual(data["verdict"], "complete", data["failure"])
            declared = {
                event["alias"]: event["identity"]
                for event in data["events"]
                if event["kind"] == "dynamic_declared"
            }
            flow = declared["root_flow"]
            reference = declared["root_ref"]
            stable = data["last_stable_snapshot"]
            self.assertGreater(stable["instances"][flow]["generation"], 0)
            self.assertEqual(stable["instances"][flow]["parent"], "TrapResource")
            self.assertEqual(stable["instances"][flow]["bound_parent"], "TrapResource")
            self.assertFalse(stable["instances"][flow]["alive"])
            self.assertEqual(stable["instances"][reference]["target"], flow)
            self.assertEqual(
                stable["instances"][reference]["target_generation"],
                stable["instances"][flow]["generation"],
            )
            model_data = read_json(model)
            scenario.write_text(
                json.dumps(
                    {
                        "schema": SNAPSHOT_SCHEMA,
                        "version": SNAPSHOT_VERSION,
                        "producer": PRODUCER,
                        "source": str(spec),
                        "model_fingerprint": model_data["model_fingerprint"],
                        "snapshot": stable,
                    }
                ),
                encoding="utf-8",
            )
            self.assertEqual(
                derive_main(
                    [
                        str(model), "--signal", "Harness.InspectAggregate",
                        "--scenario", str(scenario), "-o", str(aggregate),
                    ]
                ),
                0,
            )
            aggregate_data = read_json(aggregate)
            self.assertEqual(aggregate_data["verdict"], "complete", aggregate_data["signals"])
            self.assertEqual(
                derive_main(
                    [
                        str(model), "--signal", "Harness.InspectStale",
                        "--scenario", str(scenario), "-o", str(stale),
                    ]
                ),
                0,
            )
            stale_data = read_json(stale)
            self.assertEqual(stale_data["verdict"], "failed", stale_data["signals"])
            self.assertEqual(stale_data["signals"][1]["outcome"], "rejected")
            self.assertIn(
                "trap_flow_ref_generation_valid",
                stale_data["signals"][1]["reason"],
            )

    def test_indexed_owned_rejects_bad_publish_and_rolls_back_atomically(self) -> None:
        prefix = """
            type LogicId { }
            type CPU {
                initial_state: State::Base;
                state State::Base {
                    transitions {
                        on Transition::Preset -> State::Prepared {
                            depends_on { cpu_allowed(); }
                        }
                    }
                }
                state State::Prepared { }
            }
            type CpuGroupObject {
                owned { indexed cpus[key: LogicId]: CPU; }
                initial_state: State::Base;
                state State::Base {
                    actions { on Action::Publish { BODY } }
                }
            }
            object CpuGroup: CpuGroupObject { }
        """
        cases = {
            "duplicate": "drives { declare self.cpus[0] of CPU; declare self.cpus[0] of CPU; }",
            "wrong-key": "drives { declare self.cpus[\"bad\"] of CPU; }",
            "child-failure": (
                "drives { declare self.cpus[0] of CPU; "
                "self.cpus[0].Transition::Preset; }"
            ),
        }
        for name, body in cases.items():
            with self.subTest(name=name):
                derivation, checked, _ = self.run_source(
                    prefix.replace("BODY", body),
                    "CpuGroup.Publish",
                    max_depth="all",
                    max_breadth="all",
                )
                self.assertEqual(checked["verdict"], "failed")
                stable = derivation["last_stable_snapshot"]
                self.assertNotIn("CpuGroup.cpus[0]", stable["instances"])
                self.assertNotIn("CpuGroup.cpus[0]", stable["references"])
                self.assertEqual(stable["states"]["CpuGroup"], "Base")
                self.assertTrue(
                    any(
                        event["kind"] == "indexed_transaction_rolled_back"
                        for event in derivation["events"]
                    )
                )

        unauthorized = prefix.replace(
            "BODY", "drives { declare CpuGroup.cpus[0] of CPU; }"
        ) + """
            system Intruder {
                initial_state: State::Base;
                state State::Base {
                    actions {
                        on Action::Publish {
                            drives { declare CpuGroup.cpus[0] of CPU; }
                        }
                    }
                }
            }
        """
        derivation, checked, _ = self.run_source(unauthorized, "Intruder.Publish")
        self.assertEqual(checked["verdict"], "failed")
        self.assertIn("only by their owner handler", derivation["signals"][0]["reason"])

        missing = prefix.replace(
            "BODY", "drives { self.cpus[0].Transition::Preset; }"
        )
        derivation, checked, _ = self.run_source(missing, "CpuGroup.Publish")
        self.assertEqual(checked["verdict"], "failed")
        self.assertIn("unbound system reference", derivation["signals"][0]["reason"])

    def test_cpu_local_mailbox_ipi_idle_lane_and_rejections(self) -> None:
        source = """
            type CpuRef { }
            type TaskRef { }
            type MailboxOrdinal { }

            predicate task_ref_ready<T: TaskRef>(task_ref: T) -> bool;
            predicate mailbox_owner_cpu_is<S, C: CpuRef>(scheduler: S, cpu_ref: C) -> bool;
            predicate mailbox_ordinal_fresh<O: MailboxOrdinal>(ordinal: O) -> bool;
            predicate mailbox_target_generation_valid<T: TaskRef, C: CpuRef>(
                task_ref: T,
                cpu_ref: C
            ) -> bool;
            predicate mailbox_published_release<T: TaskRef, C: CpuRef, O: MailboxOrdinal>(
                task_ref: T,
                cpu_ref: C,
                ordinal: O
            ) -> bool;
            predicate reschedule_ipi_after_release<C: CpuRef>(cpu_ref: C) -> bool;
            predicate ssip_pending_cleared<C: CpuRef>(cpu_ref: C) -> bool;
            predicate need_resched_cpu_local<C: CpuRef>(cpu_ref: C) -> bool;
            predicate duplicate_ipi_coalesced<C: CpuRef>(cpu_ref: C) -> bool;
            predicate handler_did_not_switch<C: CpuRef>(cpu_ref: C) -> bool;
            predicate mailbox_consumed_once<T: TaskRef, O: MailboxOrdinal>(
                task_ref: T,
                ordinal: O
            ) -> bool;
            predicate target_runqueue_enqueued<T: TaskRef, C: CpuRef>(
                task_ref: T,
                cpu_ref: C
            ) -> bool;
            predicate idle_switched_through_common_context<C: CpuRef>(cpu_ref: C) -> bool;
            predicate mailbox_not_consumed<C: CpuRef>(cpu_ref: C) -> bool;
            predicate idle_wfi_recheck_safe<C: CpuRef>(cpu_ref: C) -> bool;

            system Remote {
                initial_state: State::Base;
                state State::Base {
                    invariant {
                        task_ref_ready(WorkerRef);
                        task_ref_ready(StaleWorkerRef);
                        mailbox_owner_cpu_is(Scheduler, Cpu1Ref);
                        mailbox_ordinal_fresh(Ordinal1);
                        mailbox_target_generation_valid(WorkerRef, Cpu1Ref);
                    }
                    actions {
                        on Action::Dispatch {
                            drives {
                                Scheduler.Transition::PublishInbound(WorkerRef, Cpu1Ref, Ordinal1);
                                IpiMux.Transition::SendReschedule(Cpu1Ref);
                                IpiMux.Transition::HandleReschedule(Cpu1Ref);
                                IpiMux.Action::CoalesceReschedule(Cpu1Ref);
                                Scheduler.Transition::ConsumeInbound(WorkerRef, Cpu1Ref, Ordinal1);
                                Scheduler.Action::RunIdle(Cpu1Ref);
                            }
                        }
                        on Action::IpiBeforeMailbox {
                            drives {
                                IpiMux.Action::HandleEarly(Cpu1Ref);
                                IpiMux.Action::HandleEarly(Cpu1Ref);
                                Scheduler.Action::RunIdleWithoutMailbox(Cpu1Ref);
                            }
                        }
                        on Action::WrongTarget {
                            drives {
                                Scheduler.Transition::ConsumeInbound(WorkerRef, Cpu2Ref, Ordinal1);
                            }
                        }
                        on Action::StaleGeneration {
                            drives {
                                Scheduler.Transition::ConsumeInbound(StaleWorkerRef, Cpu1Ref, Ordinal1);
                            }
                        }
                        on Action::DuplicateConsume {
                            drives {
                                Scheduler.Transition::ConsumeInbound(WorkerRef, Cpu1Ref, Ordinal1);
                            }
                        }
                    }
                }
            }

            system Scheduler {
                initial_state: State::Base;
                state State::Base {
                    transitions {
                        on Transition::PublishInbound(
                            task_ref: TaskRef,
                            target_cpu: CpuRef,
                            ordinal: MailboxOrdinal
                        ) -> State::Ready {
                            depends_on {
                                task_ref_ready(task_ref);
                                mailbox_owner_cpu_is(self, target_cpu);
                                mailbox_ordinal_fresh(ordinal);
                                mailbox_target_generation_valid(task_ref, target_cpu);
                            }
                            ensures {
                                mailbox_published_release(task_ref, target_cpu, ordinal);
                            }
                        }
                    }
                    actions {
                        on Action::RunIdleWithoutMailbox(cpu_ref: CpuRef) {
                            depends_on { need_resched_cpu_local(cpu_ref); }
                            ensures {
                                mailbox_not_consumed(cpu_ref);
                                idle_wfi_recheck_safe(cpu_ref);
                            }
                        }
                    }
                }
                state State::Ready {
                    transitions {
                        on Transition::ConsumeInbound(
                            task_ref: TaskRef,
                            target_cpu: CpuRef,
                            ordinal: MailboxOrdinal
                        ) -> State::Online {
                            depends_on {
                                mailbox_published_release(task_ref, target_cpu, ordinal);
                                mailbox_owner_cpu_is(self, target_cpu);
                                mailbox_ordinal_fresh(ordinal);
                                mailbox_target_generation_valid(task_ref, target_cpu);
                                need_resched_cpu_local(target_cpu);
                            }
                            ensures {
                                mailbox_consumed_once(task_ref, ordinal);
                                target_runqueue_enqueued(task_ref, target_cpu);
                            }
                        }
                    }
                }
                state State::Online {
                    actions {
                        on Action::RunIdle(cpu_ref: CpuRef) {
                            depends_on {
                                target_runqueue_enqueued(WorkerRef, cpu_ref);
                                mailbox_consumed_once(WorkerRef, Ordinal1);
                            }
                            ensures { idle_switched_through_common_context(cpu_ref); }
                        }
                    }
                }
            }

            system IpiMux {
                initial_state: State::Base;
                state State::Base {
                    transitions {
                        on Transition::SendReschedule(cpu_ref: CpuRef) -> State::Prepared {
                            depends_on {
                                mailbox_published_release(WorkerRef, cpu_ref, Ordinal1);
                            }
                            ensures { reschedule_ipi_after_release(cpu_ref); }
                        }
                    }
                    actions {
                        on Action::HandleEarly(cpu_ref: CpuRef) {
                            ensures {
                                ssip_pending_cleared(cpu_ref);
                                need_resched_cpu_local(cpu_ref);
                                duplicate_ipi_coalesced(cpu_ref);
                                handler_did_not_switch(cpu_ref);
                            }
                        }
                    }
                }
                state State::Prepared {
                    transitions {
                        on Transition::HandleReschedule(cpu_ref: CpuRef) -> State::Online {
                            depends_on { reschedule_ipi_after_release(cpu_ref); }
                            ensures {
                                ssip_pending_cleared(cpu_ref);
                                need_resched_cpu_local(cpu_ref);
                                handler_did_not_switch(cpu_ref);
                            }
                        }
                    }
                }
                state State::Online {
                    actions {
                        on Action::CoalesceReschedule(cpu_ref: CpuRef) {
                            ensures {
                                need_resched_cpu_local(cpu_ref);
                                duplicate_ipi_coalesced(cpu_ref);
                                handler_did_not_switch(cpu_ref);
                            }
                        }
                    }
                }
            }
        """

        derivation, checked, _ = self.run_source(
            source,
            "Remote.Dispatch",
            max_depth="all",
            max_breadth="all",
        )
        self.assertEqual(checked["verdict"], "complete", derivation)
        self.assertEqual(
            [(item["target"], item["name"], item["outcome"]) for item in derivation["signals"]],
            [
                ("Remote", "Dispatch", "completed"),
                ("Scheduler", "PublishInbound", "completed"),
                ("IpiMux", "SendReschedule", "completed"),
                ("IpiMux", "HandleReschedule", "completed"),
                ("IpiMux", "CoalesceReschedule", "completed"),
                ("Scheduler", "ConsumeInbound", "completed"),
                ("Scheduler", "RunIdle", "completed"),
            ],
        )
        facts = set(derivation["last_stable_snapshot"]["facts"])
        for fact in (
            "mailbox_published_release(WorkerRef,Cpu1Ref,Ordinal1)",
            "reschedule_ipi_after_release(Cpu1Ref)",
            "mailbox_consumed_once(WorkerRef,Ordinal1)",
            "target_runqueue_enqueued(WorkerRef,Cpu1Ref)",
            "duplicate_ipi_coalesced(Cpu1Ref)",
            "handler_did_not_switch(Cpu1Ref)",
            "idle_switched_through_common_context(Cpu1Ref)",
        ):
            self.assertIn(fact, facts)

        early, early_checked, _ = self.run_source(
            source,
            "Remote.IpiBeforeMailbox",
            max_depth="all",
            max_breadth="all",
        )
        self.assertEqual(early_checked["verdict"], "complete", early)
        early_facts = set(early["last_stable_snapshot"]["facts"])
        self.assertIn("mailbox_not_consumed(Cpu1Ref)", early_facts)
        self.assertIn("idle_wfi_recheck_safe(Cpu1Ref)", early_facts)

        rejection_scenarios = (
            (
                "WrongTarget",
                {
                    "states": {"Scheduler": "Ready", "IpiMux": "Online"},
                    "facts": [
                        "mailbox_published_release(WorkerRef,Cpu1Ref,Ordinal1)",
                        "need_resched_cpu_local(Cpu1Ref)",
                    ],
                },
            ),
            (
                "StaleGeneration",
                {
                    "states": {"Scheduler": "Ready", "IpiMux": "Online"},
                    "facts": [
                        "mailbox_published_release(StaleWorkerRef,Cpu1Ref,Ordinal1)",
                        "need_resched_cpu_local(Cpu1Ref)",
                    ],
                },
            ),
            (
                "DuplicateConsume",
                {
                    "states": {"Scheduler": "Online", "IpiMux": "Online"},
                    "facts": [
                        "mailbox_consumed_once(WorkerRef,Ordinal1)",
                        "target_runqueue_enqueued(WorkerRef,Cpu1Ref)",
                    ],
                },
            ),
        )
        for action, scenario in rejection_scenarios:
            rejected, rejected_checked, _ = self.run_source(
                source,
                f"Remote.{action}",
                max_depth="all",
                max_breadth="all",
                scenario=scenario,
            )
            self.assertEqual(rejected_checked["verdict"], "failed", rejected)
            self.assertEqual(rejected["signals"][-1]["outcome"], "rejected")
            self.assertNotIn(
                "mailbox_consumed_once(StaleWorkerRef,Ordinal1)",
                rejected["last_stable_snapshot"]["facts"],
            )

    def test_current_selectors_inherit_only_through_synchronous_task_flow_drives(self) -> None:
        source = """
            type LogicId { }
            type CpuRef { }
            type TaskRef { }
            type Stack { }
            enum TaskExecutionAuthority { None, Reserved, Live }
            type Task {
                associations {
                    flow: TaskFlow;
                }
                processes {
                    Action::Touch { state_effect: StateEffect::None; }
                }
            }
            type CPU {
                initial_state: State::Base;
                state State::Base {
                    transitions { on Transition::Preset -> State::Prepared { } }
                }
                state State::Prepared {
                    actions {
                        on Action::Touch(task_ref: TaskRef) {
                            depends_on { task_ref_ready(task_ref); }
                            ensures { cpu_touched(self); }
                        }
                    }
                }
            }
            type CpuGroupObject {
                owned { indexed cpus[key: LogicId]: CPU; }
                initial_state: State::Base;
                state State::Base {
                    transitions {
                        on Transition::Preset -> State::Prepared {
                            drives {
                                declare self.cpus[0] of CPU;
                                self.cpus[0].Transition::Preset;
                            }
                            ensures { cpu_ref_targets(BootCPURef, self.cpus[0]); }
                        }
                    }
                }
                state State::Prepared { }
            }
            type TaskFlow {
                parent: Task;
                associations { mutable cpu_ref: CpuRef; }
                processes {
                    Action::AssignCpuRef(cpu_ref: CpuRef) {
                        updates { self.cpu_ref = cpu_ref; }
                    }
                    Action::BindTaskStack(task: Task, stack: Stack) {
                        state_effect: StateEffect::None;
                    }
                }
            }
            external Human {
                drives {
                    CpuGroup.Transition::Preset;
                    BootInitFlow.Action::AssignCpuRef(BootCPURef);
                    BootInitFlow.Action::BindCurrent;
                }
                emits { BootInitFlow.Action::Run; }
            }
            object CpuGroup: CpuGroupObject { }
            object BootTask: Task {
                associations { flow = BootInitFlow; }
                initial_state: State::OnCpu;
                state State::OnCpu {
                    invariant {
                        task_execution_authority_is(BootTask, TaskExecutionAuthority::Live);
                        task_ref_targets(BootTaskRef, BootTask);
                        task_ref_ready(BootTaskRef);
                    }
                }
            }
            object BootInitFlow: TaskFlow {
                parent: BootTask;
                initial_state: State::Base;
                state State::Base {
                    invariant {
                        task_flow_parent_is(BootInitFlow, BootTask);
                        task_flow_owner_is(BootInitFlow, BootTask);
                    }
                    actions {
                        on Action::BindCurrent {
                            drives { CurrentTask.Action::BindTaskStack(BootTask, BootTask.stack); }
                        }
                        on Action::Run {
                            drives { SyncChild.Action::UseCurrent; }
                            emits { Async.Action::UseCurrent; }
                        }
                    }
                }
            }
            system SyncChild {
                parent: BootInitFlow;
                initial_state: State::Base;
                state State::Base {
                    actions {
                        on Action::UseCurrent {
                            drives { CurrentCPU.Action::Touch(CurrentTaskRef); }
                        }
                    }
                }
            }
            system Async {
                initial_state: State::Base;
                state State::Base {
                    actions {
                        on Action::UseCurrent { drives { CurrentTask.Action::Touch; } }
                    }
                }
            }
        """
        derivation, checked, _ = self.run_source(
            source, None, max_depth="all", max_breadth="all"
        )
        self.assertEqual(checked["verdict"], "failed")
        touched = [item for item in derivation["signals"] if item["name"] == "Touch"]
        self.assertEqual(
            len(touched),
            1,
            [(item["id"], item["target"], item["name"], item.get("reason")) for item in derivation["signals"]],
        )
        self.assertEqual(touched[0]["target"], "CpuGroup.cpus[0]")
        self.assertEqual(
            touched[0]["selector_resolutions"],
            [
                {
                    "selector": "CurrentCPU",
                    "source_cpu_ref": "BootCPURef",
                    "source_flow": "BootInitFlow",
                    "target": "CpuGroup.cpus[0]",
                },
                {
                    "selector": "CurrentTask",
                    "source_cpu": "CpuGroup.cpus[0]",
                    "source_flow": "BootInitFlow",
                    "source_task_ref": "BootTaskRef",
                    "target": "BootTask",
                },
            ],
        )
        async_signal = next(
            item for item in derivation["signals"] if item["target"] == "Async"
        )
        self.assertEqual(async_signal["outcome"], "failed")
        self.assertIn("CurrentTask requires an effective TaskFlow", async_signal["reason"])

    def test_sender_flow_context_property_resolves_async_scheduler_current_task(self) -> None:
        source = """
            type CpuRef { }
            type TaskRef { }
            type Stack { }
            enum TaskExecutionAuthority { Live }
            type CPU {
                initial_state: State::Base;
                state State::Base { }
            }
            type Task {
                associations { flow: TaskFlow; }
                processes {
                    Action::Touch {
                        state_effect: StateEffect::None;
                        ensures { task_touched(self); }
                    }
                }
            }
            type TaskFlow {
                parent: Task;
                associations { cpu_ref: CpuRef; }
                processes {
                    Action::Send {
                        state_effect: StateEffect::None;
                        emits { Scheduler.Action::Schedule; }
                    }
                }
            }
            type SchedulerType {
                initial_state: State::Base;
                processes {
                    Action::Schedule {
                        state_effect: StateEffect::None;
                        sender_flow_context: true;
                        drives { CurrentTask.Action::Touch; }
                    }
                }
                state State::Base { }
            }
            external Human {
                drives { BootFlow.Action::Send; }
                emits { Async.Action::Noop; }
            }
            system Async {
                initial_state: State::Base;
                state State::Base { actions { on Action::Noop { } } }
            }
            object CPU0: CPU { }
            object BootTask: Task {
                associations { flow = BootFlow; }
                initial_state: State::OnCpu;
                state State::OnCpu {
                    invariant {
                        task_execution_authority_is(BootTask, TaskExecutionAuthority::Live);
                        task_ref_targets(BootTaskRef, BootTask);
                        task_ref_ready(BootTaskRef);
                    }
                }
            }
            object BootFlow: TaskFlow {
                parent: BootTask;
                associations { cpu_ref = BootCPURef; }
                initial_state: State::Base;
                state State::Base {
                    invariant {
                        cpu_ref_targets(BootCPURef, CPU0);
                        task_flow_parent_is(BootFlow, BootTask);
                        task_flow_owner_is(BootFlow, BootTask);
                    }
                }
            }
            object Scheduler: SchedulerType { }
        """
        scenario = {
            "contextual_bindings": {
                "current_task": {
                    "CPU0": {
                        "task": "BootTask",
                        "task_ref": "BootTaskRef",
                        "source_flow": "BootFlow",
                        "source_cpu_ref": "BootCPURef",
                        "address_view": "CanonicalTaskAddress",
                        "revision": 1,
                    }
                },
                "current_stack": {
                    "CPU0": {
                        "task": "BootTask",
                        "stack": "BootTask.stack",
                        "source_flow": "BootFlow",
                        "source_cpu_ref": "BootCPURef",
                        "address_view": "CanonicalTaskAddress",
                        "revision": 1,
                    }
                },
            }
        }
        derivation, checked, trace = self.run_source(
            source,
            None,
            max_depth="all",
            max_breadth="all",
            scenario=scenario,
        )
        self.assertEqual(
            checked["verdict"],
            "complete",
            {
                "signals": [(item["id"], item["target"], item["name"], item.get("reason")) for item in derivation["signals"]],
                "trace": trace,
            },
        )
        schedules = [
            item for item in derivation["signals"]
            if item["target"] == "Scheduler" and item["name"] == "Schedule"
        ]
        self.assertEqual(
            len(schedules),
            1,
            [(item["target"], item["name"], item.get("reason")) for item in derivation["signals"]],
        )
        schedule = schedules[0]
        self.assertEqual(schedule["source"], "BootFlow")
        touch = next(item for item in derivation["signals"] if item["name"] == "Touch")
        self.assertEqual(touch["target"], "BootTask")
        self.assertEqual(touch["selector_resolutions"][0]["source_flow"], "BootFlow")

    def test_current_task_rejects_inconsistent_execution_contexts(self) -> None:
        def source(
            *,
            task_name: str = "BootTask",
            flow_name: str = "BootInitFlow",
            state: str = "OnCpu",
            authority: str = "Live",
            fixed_flow: str | None = None,
            parent: str | None = None,
            owner: str | None = None,
            ready_ref: bool = True,
            second_ref: bool = False,
        ) -> str:
            fixed_flow = fixed_flow or flow_name
            parent = parent or task_name
            owner = owner or task_name
            ready_fact = f"task_ref_ready({task_name}Ref);" if ready_ref else ""
            second_facts = (
                f"task_ref_targets(SecondRef, {task_name}); task_ref_ready(SecondRef);"
                if second_ref
                else ""
            )
            return f"""
                enum TaskExecutionAuthority {{ None, Reserved, Live }}
                type CpuRef {{ }}
                type TaskRef {{ }}
                type Stack {{ }}
                type CPU {{
                    initial_state: State::Base;
                    state State::Base {{
                        invariant {{ cpu_ref_targets(BootCPURef, CPU0); }}
                    }}
                }}
                type Task {{
                    associations {{
                        flow: TaskFlow;
                    }}
                    processes {{
                        Action::Touch(task_ref: TaskRef) {{
                            state_effect: StateEffect::None;
                            depends_on {{ task_ref_targets(task_ref, self); }}
                            ensures {{ current_task_touched(self, task_ref); }}
                        }}
                    }}
                }}
                type TaskFlow {{
                    parent: Task;
                    associations {{ cpu_ref: CpuRef; }}
                    processes {{
                        Action::BindTaskStack(task: Task, stack: Stack) {{
                            state_effect: StateEffect::None;
                        }}
                        Action::ConfirmStack(stack: Stack) {{
                            state_effect: StateEffect::None;
                        }}
                    }}
                }}
                external Human {{
                    drives {{ {flow_name}.Action::Run; }}
                    emits {{ Async.Action::Noop; }}
                }}
                system Async {{
                    initial_state: State::Base;
                    state State::Base {{
                        actions {{ on Action::Noop {{ }} }}
                    }}
                }}
                object CPU0: CPU {{ }}
                object {task_name}: Task {{
                    associations {{ flow = {fixed_flow}; }}
                    initial_state: State::{state};
                    state State::{state} {{
                        invariant {{
                            task_execution_authority_is({task_name}, TaskExecutionAuthority::{authority});
                            task_ref_targets({task_name}Ref, {task_name});
                            {ready_fact}
                            {second_facts}
                        }}
                    }}
                }}
                object OtherTask: Task {{
                    associations {{ flow = OtherFlow; }}
                    initial_state: State::Online;
                    state State::Online {{ }}
                }}
                object {flow_name}: TaskFlow {{
                    parent: {parent};
                    associations {{ cpu_ref = BootCPURef; }}
                    initial_state: State::Base;
                    state State::Base {{
                        invariant {{
                            task_flow_parent_is({flow_name}, {parent});
                            task_flow_owner_is({flow_name}, {owner});
                        }}
                        actions {{
                            on Action::Run {{
                                drives {{
                                    CurrentTask.Action::BindTaskStack({task_name}, {task_name}.stack);
                                    self.Action::ConfirmStack(CurrentStack);
                                    CurrentTask.Action::Touch(CurrentTaskRef);
                                }}
                            }}
                        }}
                    }}
                }}
                object OtherFlow: TaskFlow {{
                    parent: OtherTask;
                    associations {{ cpu_ref = BootCPURef; }}
                    initial_state: State::Base;
                    state State::Base {{
                        invariant {{
                            task_flow_parent_is(OtherFlow, OtherTask);
                            task_flow_owner_is(OtherFlow, OtherTask);
                        }}
                    }}
                }}
            """

        derivation, checked, _ = self.run_source(
            source(), None, max_depth="all", max_breadth="all"
        )
        self.assertEqual(
            checked["verdict"],
            "complete",
            [(item["id"], item["target"], item["name"], item.get("reason")) for item in derivation["signals"]],
        )
        touched = next(item for item in derivation["signals"] if item["name"] == "Touch")
        self.assertEqual(touched["target"], "BootTask")
        self.assertEqual(
            touched["selector_resolutions"],
            [{
                "selector": "CurrentTask",
                "source_cpu": "CPU0",
                "source_flow": "BootInitFlow",
                "source_task_ref": "BootTaskRef",
                "target": "BootTask",
            }],
        )
        stack_confirmation = next(
            item for item in derivation["signals"] if item["name"] == "ConfirmStack"
        )
        self.assertEqual(
            stack_confirmation["selector_resolutions"],
            [
                {
                    "selector": "CurrentTask",
                    "source_cpu": "CPU0",
                    "source_flow": "BootInitFlow",
                    "source_task_ref": "BootTaskRef",
                    "target": "BootTask",
                },
                {
                    "selector": "CurrentStack",
                    "source_cpu": "CPU0",
                    "source_flow": "BootInitFlow",
                    "source_task": "BootTask",
                    "target": "BootTask.stack",
                }
            ],
        )

        cases = [
            (source(owner="OtherTask"), "parent/owner does not uniquely match"),
            (source(fixed_flow="OtherFlow"), "parent/owner does not uniquely match the fixed child"),
            (source(state="Online"), "is not OnCpu"),
            (source(authority="Reserved"), "is not Live"),
            (source(ready_ref=False), "exactly one live TaskRef"),
            (source(second_ref=True), "exactly one live TaskRef"),
        ]
        for invalid_source, expected in cases:
            with self.subTest(expected=expected):
                derivation, checked, _ = self.run_source(
                    invalid_source, None, max_depth="all", max_breadth="all"
                )
                self.assertEqual(checked["verdict"], "failed")
                failed = next(
                    item
                    for item in reversed(derivation["signals"])
                    if item["outcome"] == "failed"
                )
                self.assertIn(expected, failed["reason"])

    def test_current_task_binding_is_cpu_local_atomic_and_required_for_selection(self) -> None:
        prebind_source = """
            type CpuRef { }
            type TaskRef { }
            type Stack { }
            enum TaskExecutionAuthority { None, Reserved, Live }
            type CPU {
                initial_state: State::Prepared;
                state State::Prepared {
                    actions {
                        on Action::Touch { state_effect: StateEffect::None; }
                    }
                }
            }
            type Task {
                associations { flow: TaskFlow; }
                processes {
                    Action::Touch { state_effect: StateEffect::None; }
                }
            }
            type TaskFlow {
                parent: Task;
                associations { cpu_ref: CpuRef; }
                processes {
                    Action::ConfirmStack(stack: Stack) { state_effect: StateEffect::None; }
                }
            }
            external Human {
                drives {
                    Flow.Action::ProbeCpu;
                    Flow.Action::ProbeTask;
                }
                emits { Async.Action::Noop; }
            }
            system Async {
                initial_state: State::Base;
                state State::Base {
                    actions { on Action::Noop { } }
                }
            }
            object CPU0: CPU { }
            object BootTask: Task {
                associations { flow = Flow; }
                initial_state: State::OnCpu;
                state State::OnCpu {
                    invariant {
                        task_execution_authority_is(BootTask, TaskExecutionAuthority::Live);
                        task_ref_targets(BootTaskRef, BootTask);
                        task_ref_ready(BootTaskRef);
                    }
                }
            }
            object Flow: TaskFlow {
                parent: BootTask;
                associations { cpu_ref = BootCPURef; }
                initial_state: State::Base;
                state State::Base {
                    invariant {
                        cpu_ref_targets(BootCPURef, CPU0);
                        task_flow_parent_is(Flow, BootTask);
                        task_flow_owner_is(Flow, BootTask);
                    }
                    actions {
                        on Action::ProbeCpu { drives { CurrentCPU.Action::Touch; } }
                        on Action::ProbeTask { drives { CurrentTask.Action::Touch; } }
                        on Action::ProbeStack { drives { self.Action::ConfirmStack(CurrentStack); } }
                    }
                }
            }
        """
        derivation, checked, _ = self.run_source(
            prebind_source, None, max_depth="all", max_breadth="all"
        )
        self.assertEqual(checked["verdict"], "failed")
        cpu_touch = next((
            item
            for item in derivation["signals"]
            if item["name"] == "Touch" and item["target"] == "CPU0"
        ), None)
        self.assertIsNotNone(
            cpu_touch,
            [(item["target"], item["name"], item["outcome"], item.get("reason")) for item in derivation["signals"]],
        )
        self.assertEqual(cpu_touch["outcome"], "completed")
        self.assertEqual(
            cpu_touch["selector_resolutions"][0]["source_flow"], "Flow"
        )
        failed_probe = next(
            item
            for item in derivation["signals"]
            if item["name"] == "ProbeTask"
        )
        self.assertIn("has no bound Task", failed_probe["reason"])
        self.assertEqual(failed_probe["before_snapshot"], failed_probe["after_snapshot"])
        self.assertEqual(
            failed_probe["after_snapshot"]["contextual_bindings"], {}
        )

        prebind_stack_source = prebind_source.replace(
            "Flow.Action::ProbeTask;", "Flow.Action::ProbeStack;"
        )
        stack_derivation, stack_checked, _ = self.run_source(
            prebind_stack_source, None, max_depth="all", max_breadth="all"
        )
        self.assertEqual(stack_checked["verdict"], "failed")
        failed_stack_probe = next(
            item for item in stack_derivation["signals"]
            if item["name"] == "ProbeStack"
        )
        self.assertIn("has no bound stack", failed_stack_probe["reason"])
        self.assertEqual(
            failed_stack_probe["before_snapshot"],
            failed_stack_probe["after_snapshot"],
        )
        self.assertEqual(
            failed_stack_probe["after_snapshot"]["contextual_bindings"], {}
        )

        def rebind_source(*, other_cpu_ref: str) -> str:
            return f"""
                type CpuRef {{ }}
                type TaskRef {{ }}
                type Stack {{ }}
                enum TaskExecutionAuthority {{ None, Live }}
                enum TaskBreakpointState {{ Valid }}
                type CPU {{
                    initial_state: State::Base;
                    state State::Base {{ }}
                }}
                type Task {{
                    associations {{ flow: TaskFlow; }}
                }}
                type TaskFlow {{
                    parent: Task;
                    associations {{ cpu_ref: CpuRef; }}
                    processes {{
                        Action::BindTaskStack(task: Task, stack: Stack) {{ state_effect: StateEffect::None; }}
                        Action::BindTask(task: Task, dispatch_flow: TaskFlow) {{ state_effect: StateEffect::None; }}
                    }}
                }}
                external Human {{
                    drives {{
                        BootInitFlow.Action::BindBoot;
                        BootInitFlow.Action::BindOther;
                    }}
                    emits {{ Async.Action::Noop; }}
                }}
                system Async {{
                    initial_state: State::Base;
                    state State::Base {{
                        actions {{ on Action::Noop {{ }} }}
                    }}
                }}
                object CPU0: CPU {{ }}
                object CPU1: CPU {{ }}
                object BootTask: Task {{
                    associations {{ flow = BootInitFlow; }}
                    initial_state: State::OnCpu;
                    state State::OnCpu {{
                        invariant {{
                            task_execution_authority_is(BootTask, TaskExecutionAuthority::Live);
                            task_ref_targets(BootTaskRef, BootTask);
                            task_ref_ready(BootTaskRef);
                        }}
                    }}
                }}
                object OtherTask: Task {{
                    associations {{ flow = OtherFlow; }}
                    initial_state: State::Online;
                    state State::Online {{
                        invariant {{
                            task_execution_authority_is(OtherTask, TaskExecutionAuthority::None);
                            task_breakpoint_state_is(OtherTask, TaskBreakpointState::Valid);
                            task_ref_targets(OtherTaskRef, OtherTask);
                            task_ref_ready(OtherTaskRef);
                        }}
                    }}
                }}
                object BootInitFlow: TaskFlow {{
                    parent: BootTask;
                    associations {{ cpu_ref = BootCPURef; }}
                    initial_state: State::Base;
                    state State::Base {{
                        invariant {{
                            cpu_ref_targets(BootCPURef, CPU0);
                            cpu_ref_targets(ApCPURef, CPU1);
                            task_flow_parent_is(BootInitFlow, BootTask);
                            task_flow_owner_is(BootInitFlow, BootTask);
                        }}
                        actions {{
                            on Action::BindBoot {{
                                drives {{ CurrentTask.Action::BindTaskStack(BootTask, BootTask.stack); }}
                            }}
                            on Action::BindOther {{
                                drives {{ CurrentTask.Action::BindTask(OtherTask, OtherFlow); }}
                            }}
                        }}
                    }}
                }}
                object OtherFlow: TaskFlow {{
                    parent: OtherTask;
                    associations {{ cpu_ref = {other_cpu_ref}; }}
                    initial_state: State::Online;
                    state State::Online {{
                        invariant {{
                            task_flow_parent_is(OtherFlow, OtherTask);
                            task_flow_owner_is(OtherFlow, OtherTask);
                        }}
                    }}
                }}
            """

        preserved_cpu1 = {
            "task": "OtherTask",
            "task_ref": "OtherTaskRef",
            "source_flow": "OtherFlow",
            "source_cpu_ref": "ApCPURef",
            "address_view": "CanonicalTaskAddress",
            "revision": 7,
        }
        preserved_cpu1_stack = {
            "task": "OtherTask",
            "stack": "OtherTask.stack",
            "source_flow": "OtherFlow",
            "source_cpu_ref": "ApCPURef",
            "address_view": "CanonicalTaskAddress",
            "revision": 7,
        }
        scenario = {
            "contextual_bindings": {
                "current_task": {"CPU1": preserved_cpu1},
                "current_stack": {"CPU1": preserved_cpu1_stack},
            }
        }
        for other_cpu_ref, expected in (
            ("BootCPURef", "outside scheduler switch commit"),
            ("ApCPURef", "does not belong to CPU CPU0"),
        ):
            with self.subTest(other_cpu_ref=other_cpu_ref):
                derivation, checked, _ = self.run_source(
                    rebind_source(other_cpu_ref=other_cpu_ref),
                    None,
                    max_depth="all",
                    max_breadth="all",
                    scenario=scenario,
                )
                self.assertEqual(checked["verdict"], "failed")
                failed_bind = next(
                    item
                    for item in reversed(derivation["signals"])
                    if item["name"] == "BindTask" and item["outcome"] == "failed"
                )
                self.assertIn(expected, failed_bind["reason"])
                self.assertEqual(
                    failed_bind["before_snapshot"], failed_bind["after_snapshot"]
                )
                bindings = failed_bind["after_snapshot"]["contextual_bindings"][
                    "current_task"
                ]
                self.assertEqual(bindings["CPU0"]["task"], "BootTask")
                self.assertEqual(bindings["CPU1"], preserved_cpu1)
                stack_bindings = failed_bind["after_snapshot"]["contextual_bindings"][
                    "current_stack"
                ]
                self.assertEqual(stack_bindings["CPU0"]["stack"], "BootTask.stack")
                self.assertEqual(stack_bindings["CPU1"], preserved_cpu1_stack)
                states = failed_bind["after_snapshot"]["states"]
                instances = failed_bind["after_snapshot"]["instances"]
                self.assertFalse(
                    any(
                        forbidden in name
                        for name in (*states, *instances)
                        for forbidden in (
                            "CurrentTaskSlot",
                            "CurrentStackSlot",
                            "BindStack",
                        )
                    )
                )

        scheduler_source = rebind_source(other_cpu_ref="BootCPURef").replace(
            "drives { CurrentTask.Action::BindTask(OtherTask, OtherFlow); }",
            "drives { Scheduler.Action::SwitchTo; }",
            1,
        )
        scheduler_source = scheduler_source.replace(
            "type TaskFlow {",
            """
                type SchedulerObject {
                    initial_state: State::Base;
                    state State::Base {
                        actions {
                            on Action::SwitchTo {
                                drives { CurrentTask.Action::BindTask(OtherTask, OtherFlow); }
                            }
                        }
                    }
                }
                type TaskFlow {""",
            1,
        ).replace(
            "object CPU0: CPU { }",
            "object Scheduler: SchedulerObject { }\n                object CPU0: CPU { }",
            1,
        )
        switched, switched_check, _ = self.run_source(
            scheduler_source,
            None,
            max_depth="all",
            max_breadth="all",
            scenario=scenario,
        )
        self.assertEqual(
            switched_check["verdict"],
            "complete",
            [(item["id"], item["target"], item["name"], item.get("reason")) for item in switched["signals"]],
        )
        bind_next = next(
            item for item in switched["signals"] if item["name"] == "BindTask"
        )
        self.assertEqual(
            bind_next["after_snapshot"]["contextual_bindings"]["current_task"]["CPU0"]["task"],
            "OtherTask",
        )
        self.assertEqual(
            bind_next["after_snapshot"]["contextual_bindings"]["current_stack"]["CPU0"]["stack"],
            "OtherTask.stack",
        )
        switch_commit = next(
            item for item in switched["signals"] if item["name"] == "SwitchTo"
        )
        committed_bindings = switch_commit["after_snapshot"]["contextual_bindings"]
        self.assertEqual(committed_bindings["current_task"]["CPU0"]["task"], "OtherTask")
        self.assertEqual(committed_bindings["current_task"]["CPU0"]["revision"], 2)
        self.assertEqual(committed_bindings["current_stack"]["CPU0"]["stack"], "OtherTask.stack")
        self.assertEqual(committed_bindings["current_stack"]["CPU0"]["revision"], 2)
        self.assertEqual(committed_bindings["current_task"]["CPU1"], preserved_cpu1)
        self.assertEqual(committed_bindings["current_stack"]["CPU1"], preserved_cpu1_stack)

    def test_boot_task_stack_refresh_is_same_pair_and_atomic(self) -> None:
        def source(stack_argument: str) -> str:
            return f"""
                type CpuRef {{ }}
                type TaskRef {{ }}
                type Stack {{ }}
                enum TaskExecutionAuthority {{ Live }}
                type CPU {{
                    initial_state: State::Base;
                    state State::Base {{ }}
                }}
                type Task {{
                    associations {{ flow: TaskFlow; }}
                }}
                type TaskFlow {{
                    parent: Task;
                    associations {{ cpu_ref: CpuRef; }}
                    processes {{
                        Action::BindTaskStack(task: Task, stack: Stack) {{
                            state_effect: StateEffect::None;
                        }}
                        Action::RefreshTaskStack(task: Task, stack: Stack) {{
                            state_effect: StateEffect::None;
                        }}
                    }}
                }}
                external Human {{
                    drives {{
                        BootInitFlow.Action::BindFirst;
                        BootInitFlow.Action::Refresh;
                    }}
                    emits {{ Async.Action::Noop; }}
                }}
                system Async {{
                    initial_state: State::Base;
                    state State::Base {{ actions {{ on Action::Noop {{ }} }} }}
                }}
                object CPU0: CPU {{ }}
                object BootTask: Task {{
                    associations {{ flow = BootInitFlow; }}
                    initial_state: State::OnCpu;
                    state State::OnCpu {{
                        invariant {{
                            task_execution_authority_is(BootTask, TaskExecutionAuthority::Live);
                            task_ref_targets(BootTaskRef, BootTask);
                            task_ref_ready(BootTaskRef);
                        }}
                    }}
                }}
                object BootInitFlow: TaskFlow {{
                    parent: BootTask;
                    associations {{ cpu_ref = BootCPURef; }}
                    initial_state: State::Base;
                    state State::Base {{
                        invariant {{
                            cpu_ref_targets(BootCPURef, CPU0);
                            task_flow_parent_is(BootInitFlow, BootTask);
                            task_flow_owner_is(BootInitFlow, BootTask);
                        }}
                        actions {{
                            on Action::BindFirst {{
                                drives {{
                                    CurrentTask.Action::BindTaskStack(BootTask, BootTask.stack);
                                }}
                            }}
                            on Action::Refresh {{
                                drives {{
                                    CurrentTask.Action::RefreshTaskStack(BootTask, {stack_argument});
                                }}
                            }}
                        }}
                    }}
                }}
            """

        successful, successful_check, _ = self.run_source(
            source("BootTask.stack"), None, max_depth="all", max_breadth="all"
        )
        self.assertEqual(
            successful_check["verdict"],
            "complete",
            [(item["id"], item["name"], item["outcome"], item.get("reason")) for item in successful["signals"]],
        )
        refreshed = next(
            item for item in successful["signals"]
            if item["name"] == "RefreshTaskStack"
        )
        pair = refreshed["after_snapshot"]["contextual_bindings"]
        self.assertEqual(pair["current_task"]["CPU0"]["revision"], 2)
        self.assertEqual(pair["current_stack"]["CPU0"]["revision"], 2)
        self.assertEqual(pair["current_stack"]["CPU0"]["stack"], "BootTask.stack")

        rejected, rejected_check, _ = self.run_source(
            source("BootTask.other_stack"), None, max_depth="all", max_breadth="all"
        )
        self.assertEqual(rejected_check["verdict"], "failed")
        failed_refresh = next(
            item for item in rejected["signals"]
            if item["name"] == "RefreshTaskStack"
        )
        self.assertIn("matching BootTask/stack pair", failed_refresh["reason"])
        self.assertEqual(
            failed_refresh["before_snapshot"], failed_refresh["after_snapshot"]
        )
        preserved = failed_refresh["after_snapshot"]["contextual_bindings"]
        self.assertEqual(preserved["current_task"]["CPU0"]["revision"], 1)
        self.assertEqual(preserved["current_stack"]["CPU0"]["revision"], 1)
        self.assertEqual(preserved["current_stack"]["CPU0"]["stack"], "BootTask.stack")

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
            empty_scenario = root / "empty-scenario.json"
            empty_scenario.write_text("{}\n", encoding="utf-8")
            for spelling in ("Startup", "Preset"):
                work = root / f"shortcut-{spelling}"
                result = subprocess.run(
                    [
                        str(shortcut),
                        "-f",
                        str(spec),
                        "-t",
                        f"Root.{spelling}",
                        "-s",
                        str(empty_scenario),
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
            "boundaries: deferred=0 trimmed=0 occurrences=0 obligations=0\n"
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

    def test_reached_snapshot_is_v10_with_boundary_provenance_and_resumes(self) -> None:
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

            empty_scenario = root / "empty-scenario.json"
            empty_scenario.write_text("{}\n", encoding="utf-8")
            snapshot = root / "snapshot.json"
            first_work = root / "first-work"
            first = subprocess.run(
                [
                    str(shortcut),
                    "-t",
                    "Root.Start",
                    "-f",
                    str(PIPELINE),
                    "-s",
                    str(empty_scenario),
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
                    "-s",
                    str(empty_scenario),
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
                    "-s",
                    str(empty_scenario),
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

    def test_pyveri_default_scenario_missing_override_and_path_safety(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            missing_work = root / "missing-work"
            missing, missing_args, _, missing_stderr = self.run_shortcut_focused(
                [
                    "-t",
                    "Computer.Startup",
                    "--work-dir",
                    str(missing_work),
                ]
            )
            self.assertEqual(missing, 2)
            self.assertIsNone(missing_args)
            self.assertIn("canonical signal Computer.Preset", missing_stderr)
            self.assertIn(
                "tools2/scenarios/Computer.Preset.snapshot.json", missing_stderr
            )
            self.assertFalse(missing_work.exists())

            empty_scenario = root / "empty-scenario.json"
            empty_scenario.write_text("{}\n", encoding="utf-8")
            override_work = root / "override-work"
            override, override_args, _, override_stderr = self.run_shortcut_focused(
                [
                    "-f",
                    str(PIPELINE),
                    "-t",
                    "Root.Start",
                    "-s",
                    str(empty_scenario),
                    "--work-dir",
                    str(override_work),
                ]
            )
            self.assertEqual(override, 0, override_stderr)
            self.assertEqual(
                override_args,
                [
                    str(PIPELINE),
                    "--source",
                    "Human",
                    "--max-depth",
                    "all",
                    "--max-breadth",
                    "all",
                    "--signal",
                    "Root.Start",
                    "--scenario",
                    str(empty_scenario),
                    "--work-dir",
                    str(override_work),
                ],
            )

            malformed = TOOLS2 / "scenarios" / "Malformed.Start.snapshot.json"
            self.addCleanup(malformed.unlink, missing_ok=True)
            malformed_cases = (
                ("missing boundary provenance", {}),
                (
                    "boundary signal does not match",
                    {
                        "provenance": {
                            "boundary": {
                                "normalized_signal": "Other.Start",
                                "source": "Harness",
                            }
                        }
                    },
                ),
                (
                    "boundary source is missing or invalid",
                    {
                        "provenance": {
                            "boundary": {
                                "normalized_signal": "Malformed.Start",
                                "source": "",
                            }
                        }
                    },
                ),
            )
            for expected, extra in malformed_cases:
                with self.subTest(default_snapshot=expected):
                    value = {
                        "schema": SNAPSHOT_SCHEMA,
                        "version": SNAPSHOT_VERSION,
                        "producer": PRODUCER,
                        "model_fingerprint": "sha256:not-reached",
                        "snapshot": {},
                        **extra,
                    }
                    malformed.write_text(json.dumps(value), encoding="utf-8")
                    malformed_work = root / expected.replace(" ", "-")
                    result, result_args, _, result_stderr = self.run_shortcut_focused(
                        [
                            "-f",
                            str(PIPELINE),
                            "-t",
                            "Malformed.Start",
                            "--work-dir",
                            str(malformed_work),
                        ]
                    )
                    self.assertEqual(result, 2)
                    self.assertIsNone(result_args)
                    self.assertIn(expected, result_stderr)
                    self.assertFalse(malformed_work.exists())

            outside = root / "outside.snapshot.json"
            outside.write_text("{}\n", encoding="utf-8")
            symlink = TOOLS2 / "scenarios" / "Escape.Preset.snapshot.json"
            symlink.symlink_to(outside)
            self.addCleanup(symlink.unlink, missing_ok=True)
            for signal in ("../outside.Preset", str(outside.with_suffix(".Preset")), "Escape.Preset"):
                with self.subTest(signal=signal):
                    unsafe, unsafe_args, _, unsafe_stderr = self.run_shortcut_focused(
                        ["-f", str(PIPELINE), "-t", signal]
                    )
                    self.assertEqual(unsafe, 2)
                    self.assertIsNone(unsafe_args)
                    self.assertTrue(
                        "unsafe default scenario" in unsafe_stderr
                        or "argument -t/--trigger" in unsafe_stderr,
                        unsafe_stderr,
                    )



class MainModelIntegrationTests(_ShortcutTestSupport, unittest.TestCase):
    """Main-model integration coverage with one immutable prepared Model per run."""

    @classmethod
    def setUpClass(cls) -> None:
        super().setUpClass()
        cls._fixture_temporary = tempfile.TemporaryDirectory(
            prefix="lkm-tools2-main-model-tests-"
        )
        cls.fixture_root = Path(cls._fixture_temporary.name)
        cls.fixture_ast = cls.fixture_root / "ast.json"
        cls.fixture_model = cls.fixture_root / "model.json"
        cls.fixture_cases = cls.fixture_root / "cases"
        cls.fixture_cases.mkdir()
        cls.fixture_evidence: list[dict] = []
        cls._prepared_run_count = 0

        started = time.monotonic()
        if parse_main(
            [str(ROOT / "spec" / "model" / "main.spec"), "-o", str(cls.fixture_ast)]
        ) != 0:
            raise RuntimeError("failed to prepare main-model AST fixture")
        parse_seconds = time.monotonic() - started

        started = time.monotonic()
        if model_main([str(cls.fixture_ast), "-o", str(cls.fixture_model)]) != 0:
            raise RuntimeError("failed to prepare main-model Model fixture")
        model_seconds = time.monotonic() - started

        cls.prepared_model_document = read_json(cls.fixture_model)
        cls.fixture_model_fingerprint = cls.prepared_model_document["model_fingerprint"]
        cls.fixture_source_hash = hashlib.sha256(
            (ROOT / "spec" / "model" / "main.spec").read_bytes()
        ).hexdigest()
        cls.fixture_ast_hash = hashlib.sha256(cls.fixture_ast.read_bytes()).hexdigest()
        cls.fixture_model_hash = hashlib.sha256(cls.fixture_model.read_bytes()).hexdigest()
        cls.fixture_document_hash = hashlib.sha256(
            json.dumps(
                cls.prepared_model_document,
                ensure_ascii=False,
                sort_keys=True,
                separators=(",", ":"),
            ).encode()
        ).hexdigest()
        cls.fixture_ast.chmod(0o444)
        cls.fixture_model.chmod(0o444)
        cls.fixture_evidence.append(
            {
                "case": "prepared-main-model",
                "cache_hit": False,
                "source": cls.prepared_model_document["source"],
                "source_sha256": cls.fixture_source_hash,
                "model_fingerprint": cls.prepared_model_document["model_fingerprint"],
                "phases_seconds": {
                    "parse": round(parse_seconds, 6),
                    "model": round(model_seconds, 6),
                },
                "artifact_bytes": {
                    "ast": cls.fixture_ast.stat().st_size,
                    "model": cls.fixture_model.stat().st_size,
                },
            }
        )
        cls._assert_fixture_integrity(check_document=True)

    @classmethod
    def tearDownClass(cls) -> None:
        try:
            cls._assert_fixture_integrity(check_document=True)
            print(
                "tools2-main-model-test-evidence: "
                + json.dumps(cls.fixture_evidence, sort_keys=True),
                file=sys.stderr,
            )
        finally:
            cls._fixture_temporary.cleanup()
            super().tearDownClass()

    @classmethod
    def _assert_fixture_integrity(cls, *, check_document: bool = False) -> None:
        if check_document and (
            hashlib.sha256(
                (ROOT / "spec" / "model" / "main.spec").read_bytes()
            ).hexdigest()
            != cls.fixture_source_hash
        ):
            raise AssertionError("shared main-model source changed")
        if hashlib.sha256(cls.fixture_ast.read_bytes()).hexdigest() != cls.fixture_ast_hash:
            raise AssertionError("shared main-model AST fixture changed")
        if hashlib.sha256(cls.fixture_model.read_bytes()).hexdigest() != cls.fixture_model_hash:
            raise AssertionError("shared main-model Model fixture changed")
        if (
            hashlib.sha256(
                json.dumps(
                    cls.prepared_model_document,
                    ensure_ascii=False,
                    sort_keys=True,
                    separators=(",", ":"),
                ).encode()
            ).hexdigest()
            != cls.fixture_document_hash
        ):
            raise AssertionError("shared in-memory main-model fixture changed")
        if (
            cls.prepared_model_document["model_fingerprint"]
            != cls.fixture_model_fingerprint
        ):
            raise AssertionError("shared main-model fingerprint changed")

    def setUp(self) -> None:
        self._assert_fixture_integrity(check_document=True)

    def tearDown(self) -> None:
        self._assert_fixture_integrity(check_document=True)

    def record_cli_pipeline(self, label: str, seconds: float, work: Path) -> None:
        self.fixture_evidence.append(
            {
                "case": label,
                "cache_hit": False,
                "model_fingerprint": read_json(work / "model.json")["model_fingerprint"],
                "phases_seconds": {"complete_cli": round(seconds, 6)},
                "artifact_bytes": {
                    name.removesuffix(".json").removesuffix(".txt"): (work / name).stat().st_size
                    for name in (
                        "ast.json",
                        "model.json",
                        "derive.json",
                        "check.json",
                        "view.json",
                        "trace.txt",
                    )
                },
            }
        )

    def run_prepared(
        self,
        label: str,
        *,
        signal: str | None = None,
        source: str = "Human",
        until: str | None = None,
        scenario: str | Path | None = None,
        max_depth: int | None = None,
        max_breadth: int | None = None,
        include_view: bool = False,
    ) -> tuple[dict, dict, dict | None, Path]:
        self._assert_fixture_integrity()
        type(self)._prepared_run_count += 1
        case_root = self.fixture_cases / f"{self._prepared_run_count:02d}-{label}"
        case_root.mkdir()

        started = time.monotonic()
        derived = derive(
            self.prepared_model_document,
            signal=signal,
            source=source,
            until=until,
            scenario=scenario,
            max_depth=max_depth,
            max_breadth=max_breadth,
        )
        derive_seconds = time.monotonic() - started
        derivation = {
            "schema": DERIVE_SCHEMA,
            "version": DERIVE_VERSION,
            "producer": PRODUCER,
            "source": self.prepared_model_document["source"],
            **derived,
        }

        started = time.monotonic()
        checked_result = check_derivation(derivation)
        checked = {
            "schema": CHECK_SCHEMA,
            "version": CHECK_VERSION,
            "producer": PRODUCER,
            "source": derivation["source"],
            **checked_result,
        }
        check_seconds = time.monotonic() - started

        view = None
        view_seconds = None
        if include_view:
            started = time.monotonic()
            view = {
                "schema": VIEW_SCHEMA,
                "version": VIEW_VERSION,
                "producer": PRODUCER,
                "source": derivation["source"],
                **build_view(derivation),
            }
            view_seconds = time.monotonic() - started
        summary_path = case_root / "summary.json"
        write_json(
            summary_path,
            {
                "model_fingerprint": derivation["model_fingerprint"],
                "verdict": derivation["verdict"],
                "allowed": checked["allowed"],
                "signals": len(derivation["signals"]),
                "events": len(derivation["events"]),
                "boundary_inventory": len(derivation["boundary_inventory"]),
                "obligations": len(derivation["obligations"]),
                "view_signals": None if view is None else len(view["signals"]),
            },
        )
        self.fixture_evidence.append(
            {
                "case": label,
                "cache_hit": True,
                "model_fingerprint": derivation["model_fingerprint"],
                "phases_seconds": {
                    "derive": round(derive_seconds, 6),
                    "check": round(check_seconds, 6),
                    "view": None if view_seconds is None else round(view_seconds, 6),
                },
                "artifact_size": {
                    "summary_bytes": summary_path.stat().st_size,
                    "signals": len(derivation["signals"]),
                    "events": len(derivation["events"]),
                    "boundary_inventory": len(derivation["boundary_inventory"]),
                    "obligations": len(derivation["obligations"]),
                    "view_signals": None if view is None else len(view["signals"]),
                },
            }
        )
        self.assertEqual(
            derivation["model_fingerprint"],
            self.prepared_model_document["model_fingerprint"],
        )
        self._assert_fixture_integrity()
        return derivation, checked, view, case_root

    def write_snapshot(self, path: Path, derivation: dict) -> None:
        write_json(
            path,
            {
                "schema": SNAPSHOT_SCHEMA,
                "version": SNAPSHOT_VERSION,
                "producer": PRODUCER,
                "source": derivation["source"],
                "model_fingerprint": derivation["model_fingerprint"],
                "snapshot": derivation["last_stable_snapshot"],
                "provenance": {
                    "verdict": derivation["verdict"],
                    "root_request": derivation["root_request"],
                    "until_request": derivation["until_request"],
                    "boundary": derivation["boundary"],
                },
            },
        )

    def test_main_model_default_request_reaches_kernel_presend_and_snapshot_resumes(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            shortcut = TOOLS2 / "bin" / "pyveri"
            work = root / "kernel-boundary"
            snapshot = root / "kernel-boundary.snapshot.json"
            cli_started = time.monotonic()
            reached = subprocess.run(
                [
                    str(shortcut),
                    "-u",
                    "Kernel.Enable",
                    "--work-dir",
                    str(work),
                    "--snapshot-out",
                    str(snapshot),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(reached.returncode, 0, reached.stderr)
            self.record_cli_pipeline(
                "canonical-presend-cli", time.monotonic() - cli_started, work
            )
            derivation = read_json(work / "derive.json")
            self.assertEqual(derivation["verdict"], "reached")
            self.assertEqual(
                (derivation["root_request"]["source"], derivation["root_request"]["target"], derivation["root_request"]["signal"]),
                ("Human", "Computer", "Preset"),
            )
            self.assertEqual(derivation["boundary"]["normalized_signal"], "Kernel.Enable")
            self.assertEqual(
                [
                    (
                        item["id"],
                        item["source"],
                        item["target"],
                        item["name"],
                        item["delivery"],
                        item["cause_id"],
                    )
                    for item in derivation["signals"]
                ],
                [
                    ("sig-0001", "Human", "Computer", "Preset", "drives", None),
                    ("sig-0002", "Computer", "Riscv64Platform", "Preset", "drives", "sig-0001"),
                    ("sig-0003", "Computer", "OpenSBI", "Preset", "drives", "sig-0001"),
                    ("sig-0004", "Computer", "Kernel", "Preset", "drives", "sig-0001"),
                    ("sig-0005", "Human", "Computer", "Setup", "drives", None),
                    ("sig-0006", "Computer", "Riscv64Platform", "Setup", "drives", "sig-0005"),
                    ("sig-0007", "Computer", "OpenSBI", "Setup", "drives", "sig-0005"),
                    ("sig-0008", "Computer", "Kernel", "Setup", "drives", "sig-0005"),
                    ("sig-0009", "Kernel", "Config", "Enable", "drives", "sig-0008"),
                    ("sig-0010", "Kernel", "Lds", "Enable", "drives", "sig-0008"),
                    ("sig-0011", "Human", "Computer", "Enable", "emits", None),
                    ("sig-0012", "Computer", "Riscv64Platform", "Enable", "emits", "sig-0011"),
                    ("sig-0013", "Riscv64Platform", "OpenSBI", "Enable", "emits", "sig-0012"),
                    ("sig-0014", "OpenSBI", "CpuGroup", "Preset", "drives", "sig-0013"),
                    ("sig-0015", "CpuGroup", "CpuGroup.cpus[0]", "Preset", "drives", "sig-0014"),
                ],
            )
            self.assertEqual(
                [
                    (
                        item["handler"]["id"],
                        item["before_snapshot"]["states"][item["target"]],
                        item["after_snapshot"]["states"][item["target"]],
                        item["outcome"],
                    )
                    for item in derivation["signals"]
                ],
                [
                    ("Computer.Transition::Preset@Base", "Base", "Prepared", "completed"),
                    ("Riscv64Platform.Transition::Preset@Base", "Base", "Prepared", "completed"),
                    ("OpenSBI.Transition::Preset@Base", "Base", "Prepared", "completed"),
                    ("Kernel.Transition::Preset@Base", "Base", "Prepared", "completed"),
                    ("Computer.Transition::Setup@Prepared", "Prepared", "Ready", "completed"),
                    ("Riscv64Platform.Transition::Setup@Prepared", "Prepared", "Ready", "completed"),
                    ("OpenSBI.Transition::Setup@Prepared", "Prepared", "Ready", "completed"),
                    ("Kernel.Transition::Setup@Prepared", "Prepared", "Ready", "completed"),
                    ("Config.Transition::Enable@Ready", "Ready", "Online", "completed"),
                    ("Lds.Transition::Enable@Ready", "Ready", "Online", "completed"),
                    ("Computer.Transition::Enable@Ready", "Ready", "Online", "completed"),
                    ("Riscv64Platform.Transition::Enable@Ready", "Ready", "Online", "completed"),
                    ("OpenSBI.Transition::Enable@Ready", "Ready", "Online", "completed"),
                    ("CpuGroup.Transition::Preset@Base", "Base", "Prepared", "completed"),
                    ("CpuGroup.cpus[0].Transition::Preset@Base", "Base", "Prepared", "completed"),
                ],
            )
            computer_signals = [
                item for item in derivation["signals"] if item["target"] == "Computer"
            ]
            self.assertEqual(
                [
                    (item["source"], item["name"], item["delivery"], item["cause_id"])
                    for item in computer_signals
                ],
                [
                    ("Human", "Preset", "drives", None),
                    ("Human", "Setup", "drives", None),
                    ("Human", "Enable", "emits", None),
                ],
            )
            async_ids = {"sig-0011", "sig-0012", "sig-0013"}
            self.assertEqual(
                [
                    (
                        event["kind"],
                        event.get("child_id") or event.get("signal_id"),
                        event.get("fifo_position"),
                        event.get("remaining"),
                    )
                    for event in derivation["events"]
                    if (
                        event["kind"] == "emits_enqueued"
                        and event.get("child_id") in async_ids
                    )
                    or (
                        event["kind"] == "emits_dequeued"
                        and event.get("signal_id") in async_ids
                    )
                ],
                [
                    ("emits_enqueued", "sig-0011", 1, None),
                    ("emits_dequeued", "sig-0011", None, 0),
                    ("emits_enqueued", "sig-0012", 1, None),
                    ("emits_dequeued", "sig-0012", None, 0),
                    ("emits_enqueued", "sig-0013", 1, None),
                    ("emits_dequeued", "sig-0013", None, 0),
                ],
            )
            self.assertLess(
                next(i for i, item in enumerate(derivation["signals"]) if item["target"] == "Config"),
                next(i for i, item in enumerate(derivation["signals"]) if item["target"] == "Lds"),
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
                        "BootArgs",
                        "Config",
                        "Lds",
                        "Computer",
                        "Riscv64Platform",
                        "BootCpuRegisters",
                        "OpenSBI",
                        "Kernel",
                        "KernelImage",
                        "CpuGroup",
                        "CpuGroup.cpus[0]",
                    )
                },
                {
                    "BootArgs": "Online",
                    "Config": "Online",
                    "Lds": "Online",
                    "Computer": "Online",
                    "Riscv64Platform": "Online",
                    "BootCpuRegisters": "Online",
                    "OpenSBI": "Online",
                    "Kernel": "Ready",
                    "KernelImage": "Base",
                    "CpuGroup": "Prepared",
                    "CpuGroup.cpus[0]": "Prepared",
                },
            )
            self.assertEqual(
                derivation["boundary"]["snapshot"]["instances"]["CpuGroup.cpus[0]"]["key"],
                0,
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
            boundary_facts = set(derivation["boundary"]["snapshot"]["facts"])
            self.assertIn("assert:BootCpuRegisters.satp == 0", boundary_facts)
            self.assertIn("linux_riscv64_kernel_boot_spec_available", boundary_facts)
            self.assertIn("linux_riscv64_kernel_boot_spec_adopted", boundary_facts)
            self.assertIn("linux_riscv64_kernel_a0_hartid_required", boundary_facts)
            self.assertIn("linux_riscv64_kernel_a1_dtb_pa_required", boundary_facts)
            self.assertIn("linux_riscv64_kernel_satp_zero_required", boundary_facts)
            self.assertIn(
                "linux_riscv64_kernel_pmd_aligned_load_required(Config.pmd_size)",
                boundary_facts,
            )
            self.assertIn(
                "kernel_elf_linked_from_config_and_lds(Config,Lds)", boundary_facts
            )
            self.assertIn(
                "kernel_boot_artifact_constructed_from_elf(Config,Lds)", boundary_facts
            )
            self.assertIn("kernel_boot_artifact_constructed", boundary_facts)
            self.assertIn(
                "kernel_image_loaded_for_handoff_at(OpenSBI.kernel_load_pa)",
                boundary_facts,
            )
            self.assertIn(
                "kernel_image_load_pmd_aligned(OpenSBI.kernel_load_pa,Config.pmd_size)",
                boundary_facts,
            )
            self.assertIn("assert:OpenSBI.kernel_load_pa != 0", boundary_facts)
            kernel_setup = next(
                item
                for item in derivation["signals"]
                if item["target"] == "Kernel" and item["name"] == "Setup"
            )
            kernel_ready_facts = set(kernel_setup["after_snapshot"]["facts"])
            self.assertIn("kernel_boot_artifact_constructed", kernel_ready_facts)
            self.assertFalse(
                any(
                    fact.startswith("kernel_image_loaded_for_handoff_at(")
                    or fact.startswith("kernel_image_load_pmd_aligned(")
                    for fact in kernel_ready_facts
                )
            )
            self.assertIn("ordered_booting_enabled", boundary_facts)
            self.assertIn("primary_hart_only_at_kernel_entry", boundary_facts)
            self.assertIn("task_concurrency_closed", boundary_facts)
            self.assertNotIn("interrupt_concurrency_closed", boundary_facts)
            self.assertNotIn("context_is(SystemExclusive)", boundary_facts)
            self.assertNotIn("primary_hart_sie_clear_at_kernel_entry", boundary_facts)
            self.assertEqual(boundary_states["LinuxRiscv64KernelBootSpec"], "Online")
            self.assertEqual(boundary_states["BootInitFlow"], "Base")
            self.assertEqual(boundary_states["KernelInitFlow"], "Base")
            self.assertEqual(boundary_states["KernelImage"], "Base")
            self.assertFalse(
                any(
                    item["target"] == "Kernel" and item["name"] == "Enable"
                    for item in derivation["signals"]
                )
            )
            self.assertFalse(
                any(
                    event["kind"] != "until_signal_reached"
                    and event.get("target") == "Kernel"
                    and event.get("signal") == "Enable"
                    for event in derivation["events"]
                )
            )
            saved = read_json(snapshot)
            self.assertEqual(saved["snapshot"], derivation["boundary"]["snapshot"])
            self.assertEqual(snapshot.read_bytes(), KERNEL_ENABLE_SCENARIO.read_bytes())
            self.assertEqual(saved["schema"], SNAPSHOT_SCHEMA)
            self.assertEqual(saved["version"], SNAPSHOT_VERSION)
            self.assertEqual(saved["producer"], PRODUCER)
            self.assertEqual(saved["source"], "spec/model/main.spec")
            self.assertEqual(saved["model_fingerprint"], derivation["model_fingerprint"])
            model_data = self.prepared_model_document
            view_data = read_json(work / "view.json")
            systems = model_data["model"]["systems"]
            self.assertEqual(
                {
                    name: systems[name]["parent"]
                    for name in (
                        "KernelAddrSpace", "Vm", "Soc", "CpuGroup", "KernelImage",
                        "FixMap", "LinearMap", "UserSpaceReserve", "PhysicalDirect",
                        "TrampolineVm", "EarlyVm", "SwapperVm",
                    )
                },
                {
                    "KernelAddrSpace": "Kernel", "Vm": "Kernel", "Soc": "Kernel",
                    "CpuGroup": "Kernel", "KernelImage": "KernelAddrSpace",
                    "FixMap": "KernelAddrSpace", "LinearMap": "KernelAddrSpace",
                    "UserSpaceReserve": "KernelAddrSpace", "PhysicalDirect": "Vm",
                    "TrampolineVm": "Vm", "EarlyVm": "Vm", "SwapperVm": "Vm",
                },
            )
            bind_body = systems["BootInitFlow"]["handlers_by_name"]["BindTask"][0]["body"]
            bind_guards = next(item["entries"] for item in bind_body if item["kind"] == "depends_on")
            self.assertTrue(
                any(
                    item.get("name") == "current_task_bind_scheduler_commit_boundary_valid"
                    for item in bind_guards
                )
            )
            self.assertFalse(
                any("stack" in item.get("text", "").lower() for item in bind_guards)
            )
            self.assertFalse(
                any(
                    item.get("name") == "cpu_translation_controller_matches_live_satp_for_ref"
                    for item in bind_guards
                )
            )
            bind_stack_body = systems["BootInitFlow"]["handlers_by_name"]["BindTaskStack"][0]["body"]
            bind_stack_guards = next(
                item["entries"] for item in bind_stack_body if item["kind"] == "depends_on"
            )
            bind_stack_ensures = next(
                item["entries"] for item in bind_stack_body if item["kind"] == "ensures"
            )
            self.assertTrue(
                any(
                    item.get("name") == "boot_task_bind_task_stack_boundary_valid"
                    for item in bind_stack_ensures
                )
            )
            self.assertTrue(
                any(
                    item.get("kind") == "reference_condition"
                    and item.get("reference") == "task.flow"
                    for item in bind_stack_guards
                )
            )
            refresh_body = systems["BootInitFlow"]["handlers_by_name"]["RefreshTaskStack"][0]["body"]
            refresh_guards = next(
                item["entries"] for item in refresh_body if item["kind"] == "depends_on"
            )
            refresh_ensures = next(
                item["entries"] for item in refresh_body if item["kind"] == "ensures"
            )
            self.assertTrue(
                any(
                    item.get("name") == "boot_task_refresh_task_stack_boundary_valid"
                    for item in refresh_ensures
                )
            )
            self.assertTrue(
                any(
                    item.get("kind") == "reference_condition"
                    and item.get("reference") == "task.flow"
                    for item in refresh_guards
                )
            )
            self.assertEqual(
                {
                    derivation["model_fingerprint"],
                    model_data["model_fingerprint"],
                    view_data["model_fingerprint"],
                },
                {saved["model_fingerprint"]},
            )
            with mock.patch.dict(os.environ, {"VERBOSE": "0"}):
                compact_text = render_text(view_data)
            with mock.patch.dict(os.environ, {"VERBOSE": "1"}):
                verbose_text = render_text(view_data)
            self.assertTrue(compact_text.startswith("verdict: reached\nboundaries: "))
            self.assertIn(
                "boundary: OpenSBI -- Enable --> Kernel (before send)\n",
                compact_text,
            )
            self.assertIn(
                "Riscv64Platform -- Enable --> OpenSBI[Ready:Online]",
                compact_text,
            )
            self.assertIn("Signal derivation: Human -> Computer.Preset", verbose_text)
            self.assertIn(
                "reached boundary: OpenSBI -> Kernel.Enable [emits]", verbose_text
            )
            self.assertIn(
                "sig-0013 [emits] Riscv64Platform -> OpenSBI.Enable", verbose_text
            )
            self.assertEqual(saved["provenance"]["verdict"], "reached")
            self.assertEqual(saved["provenance"]["boundary"], derivation["boundary"])
            self.assertEqual(
                saved["provenance"]["boundary"]["call_span"]["source_file"],
                "spec/model/systems/opensbi.spec",
            )
            self.assertIn("task_ref_ready(BootTaskRef)", saved["snapshot"]["facts"])
            self.assertIn(
                "task_ref_targets(BootTaskRef,BootTask)", saved["snapshot"]["facts"]
            )
            normal_data, normal_checked, _, _ = self.run_prepared("initial-closure")
            self.assertEqual(normal_data["verdict"], "complete")
            self.assertEqual(normal_checked["verdict"], "complete")
            self.assertEqual(normal_checked["exit_code"], 0)
            self.assertFalse(
                any(
                    item["outcome"] in {"rejected", "failed"}
                    for item in normal_data["signals"]
                )
            )
            self.assertIsNone(normal_data["until_request"])
            self.assertIsNone(normal_data["boundary"])
            signals = normal_data["signals"]
            self.assertEqual(
                [
                    (item["target"], item["payload"][0]["value"])
                    for item in signals
                    if item["name"] == "ActivateOnCpu"
                ],
                [
                    ("PhysicalDirect", "BootCPURef"),
                    ("TrampolineVm", "BootCPURef"),
                    ("EarlyVm", "BootCPURef"),
                    ("SwapperVm", "BootCPURef"),
                    ("PhysicalDirect", "ApCPURef"),
                    ("TrampolineVm", "ApCPURef"),
                    ("SwapperVm", "ApCPURef"),
                ],
            )

            def signal_index(source: str, target: str, name: str) -> int:
                return next(
                    index
                    for index, item in enumerate(signals)
                    if (item["source"], item["target"], item["name"])
                    == (source, target, name)
                )

            kernel_enable_index = signal_index("OpenSBI", "Kernel", "Enable")
            accept_index = signal_index("Kernel", "Kernel", "AcceptEnable")
            boot_started_index = signal_index("Kernel", "BootInitFlow", "Preset")
            boot_ready_index = signal_index("BootInitFlow", "BootInitFlow", "Setup")
            boot_online_index = signal_index("BootInitFlow", "BootInitFlow", "Enable")
            first_schedule_index = signal_index(
                "BootInitFlow", "Cpu0Scheduler", "Schedule"
            )
            kernel_init_restore_index = signal_index(
                "Cpu0Scheduler", "KernelInitTask", "RestoreCoreContext"
            )
            kernel_init_finish_index = signal_index(
                "Cpu0Scheduler", "Cpu0Scheduler", "FinishTaskSwitch"
            )
            kernel_init_dispatch_index = signal_index(
                "Cpu0Scheduler", "KernelInitTask", "Dispatch"
            )
            kernel_init_started_index = signal_index(
                "BootInitRestInitPhase", "KernelInitFlow", "Preset"
            )
            kernel_init_ready_index = signal_index(
                "BootInitRestInitPhase", "KernelInitFlow", "Setup"
            )
            kernel_init_online_index = signal_index(
                "BootInitRestInitPhase", "KernelInitFlow", "Enable"
            )
            kernel_init_flow_enter_index = signal_index(
                "Cpu0Scheduler", "KernelInitFlow", "Enter"
            )
            handoff_prepare_online_index = signal_index(
                "PayloadHandoffPreparePhase",
                "PayloadHandoffPreparePhase",
                "Enable",
            )
            payload_commit_index = signal_index(
                "Kernel", "KernelInitFlow", "CommitPayloadHandoff"
            )
            self.assertEqual(
                [
                    kernel_enable_index,
                    accept_index,
                    boot_started_index,
                    boot_ready_index,
                    kernel_init_started_index,
                    kernel_init_ready_index,
                    kernel_init_online_index,
                    boot_online_index,
                    first_schedule_index,
                    kernel_init_restore_index,
                    kernel_init_finish_index,
                    kernel_init_dispatch_index,
                    kernel_init_flow_enter_index,
                    handoff_prepare_online_index,
                    payload_commit_index,
                ],
                sorted(
                    [
                        kernel_enable_index,
                        accept_index,
                        boot_started_index,
                        boot_ready_index,
                        kernel_init_started_index,
                        kernel_init_ready_index,
                        kernel_init_online_index,
                        boot_online_index,
                        first_schedule_index,
                        kernel_init_restore_index,
                        kernel_init_finish_index,
                        kernel_init_dispatch_index,
                        kernel_init_flow_enter_index,
                        handoff_prepare_online_index,
                        payload_commit_index,
                    ]
                ),
            )
            self.assertEqual(
                (
                    signals[kernel_init_dispatch_index]["delivery"],
                    signals[kernel_init_flow_enter_index]["delivery"],
                ),
                ("drives", "drives"),
            )
            self.assertFalse(
                any(
                    (item["source"], item["target"], item["name"])
                    == ("KernelInitTask", "KernelInitFlow", "Preset")
                    for item in signals
                )
            )
            kernel_enable_signal = signals[kernel_enable_index]
            self.assertEqual(
                (
                    kernel_enable_signal["before_snapshot"]["states"]["Kernel"],
                    kernel_enable_signal["after_snapshot"]["states"]["Kernel"],
                ),
                ("Ready", "Online"),
            )
            for item in signals[accept_index:payload_commit_index]:
                self.assertEqual(item["before_snapshot"]["states"]["Kernel"], "Ready")
                self.assertEqual(item["after_snapshot"]["states"]["Kernel"], "Ready")
            self.assertEqual(
                signals[payload_commit_index]["before_snapshot"]["states"]["Kernel"],
                "Online",
            )
            self.assertEqual(
                sum(
                    (item["source"], item["target"], item["name"])
                    == ("Kernel", "KernelInitFlow", "CommitPayloadHandoff")
                    for item in signals
                ),
                1,
            )
            fixmap = next(
                item
                for item in normal_data["signals"]
                if item["target"] == "FixMap" and item["name"] == "Preset"
            )
            self.assertEqual(fixmap["outcome"], "completed")
            normal_states = normal_data["last_stable_snapshot"]["states"]
            self.assertEqual(normal_states["BootArgs"], "Online")
            self.assertEqual(normal_states["Config"], "Online")
            self.assertEqual(normal_states["Lds"], "Online")
            self.assertEqual(normal_states["Computer"], "Online")
            self.assertEqual(normal_states["Riscv64Platform"], "Online")
            self.assertEqual(normal_states["BootCpuRegisters"], "Online")
            self.assertEqual(normal_states["OpenSBI"], "Online")
            self.assertEqual(normal_states["Kernel"], "Online")
            self.assertEqual(normal_states["KernelInitFlow"], "Online")
            self.assertEqual(normal_states["KernelInitUserAppRuntime"], "Online")
            self.assertEqual(
                {
                    name: normal_states[name]
                    for name in (
                        "KernelAddrSpace", "Vm", "PhysicalDirect", "TrampolineVm",
                        "EarlyVm", "SwapperVm",
                    )
                },
                {
                    "KernelAddrSpace": "Online", "Vm": "Online",
                    "PhysicalDirect": "Ready", "TrampolineVm": "Ready",
                    "EarlyVm": "Ready", "SwapperVm": "Ready",
                },
            )
            normal_facts = set(normal_data["last_stable_snapshot"]["facts"])
            for fact in (
                "cpu_active_translation_controller_for_ref_is(BootCPURef,TranslationControllerKind::SwapperVm)",
                "cpu_active_translation_controller_for_ref_is(ApCPURef,TranslationControllerKind::SwapperVm)",
                "translation_handoff_recorded(BootCPURef,TranslationControllerKind::TrampolineVm,TranslationControllerKind::EarlyVm,\"satp_of(EarlyVm.pg_dir, Config.satp_mode)\")",
                "translation_handoff_to_swapper_recorded_from_active_controller(BootCPURef,\"satp_of(SwapperVm.pg_dir, Config.satp_mode)\")",
                "translation_handoff_to_swapper_recorded_from_active_controller(ApCPURef,\"satp_of(SwapperVm.pg_dir, Config.satp_mode)\")",
            ):
                self.assertIn(fact, normal_facts)
            self.assertTrue(
                any(
                    fact.startswith("kernel_application_environment_ready(Kernel,")
                    for fact in normal_data["last_stable_snapshot"]["facts"]
                )
            )
            lifetime_lifecycle_counts = {
                (target, transition): sum(
                    item["target"] == target and item["name"] == transition
                    for item in normal_data["signals"]
                )
                for target in (
                    "BootInitFlow",
                    "KernelInitFlow",
                    "ApIdleFlow",
                    "KernelInitUserAppRuntime",
                )
                for transition in ("Setup", "Enable")
            }
            self.assertEqual(
                lifetime_lifecycle_counts,
                {
                    (target, transition): 1
                    for target in (
                        "BootInitFlow",
                        "KernelInitFlow",
                        "ApIdleFlow",
                        "KernelInitUserAppRuntime",
                    )
                    for transition in ("Setup", "Enable")
                },
            )

            completed_snapshot = root / "kernel-online.snapshot.json"
            resumed_data, resumed_checked, _, _ = self.run_prepared(
                "canonical-snapshot-closure",
                signal="Kernel.Enable",
                source="OpenSBI",
                scenario=snapshot,
            )
            self.write_snapshot(completed_snapshot, resumed_data)
            self.assertEqual(resumed_data["verdict"], "complete")
            self.assertEqual(resumed_checked["verdict"], "complete")
            self.assertEqual(resumed_checked["exit_code"], 0)
            self.assertTrue(resumed_checked["allowed"])
            self.assertEqual(resumed_data["summary"]["inventory_deferred"], 135)
            self.assertEqual(resumed_data["summary"]["inventory_trimmed"], 57)
            self.assertEqual(resumed_data["summary"]["unresolved_obligations"], 0)
            self.assertEqual(len(resumed_data["boundary_inventory"]), 192)
            occurrence_by_boundary = {
                item["boundary_id"]: item
                for item in resumed_data["boundary_occurrences"]
            }
            self.assertTrue(
                {"smp_bringup.001", "smp_bringup.002", "smp_bringup.003"}
                <= set(occurrence_by_boundary)
            )
            self.assertEqual(occurrence_by_boundary["page_alloc.001"]["location"], "state")
            self.assertEqual(occurrence_by_boundary["page_alloc.001"]["state"], "Ready")
            self.assertFalse(
                any(
                    item["outcome"] in {"rejected", "failed"}
                    for item in resumed_data["signals"]
                )
            )
            self.assertEqual(resumed_data["initial_snapshot"], saved["snapshot"])
            self.assertEqual(
                (resumed_data["root_request"]["target"], resumed_data["root_request"]["signal"]),
                ("Kernel", "Enable"),
            )
            self.assertIsNone(resumed_data["until_request"])

            def semantic_signal_sequence(items: list[dict]) -> list[tuple]:
                return [
                    (
                        item["source"],
                        item["target"],
                        item["name"],
                        "entry" if index == 0 else item["delivery"],
                        item["handler"]["kind"],
                        item["handler"]["id"],
                        item["outcome"],
                    )
                    for index, item in enumerate(items)
                ]

            self.assertEqual(
                semantic_signal_sequence(normal_data["signals"][15:]),
                semantic_signal_sequence(resumed_data["signals"]),
            )
            for field in (
                "states",
                "facts",
                "references",
                "instances",
            ):
                self.assertEqual(
                    normal_data["last_stable_snapshot"][field],
                    resumed_data["last_stable_snapshot"][field],
                    field,
                )
            self.assertEqual(
                normal_data["boundary_inventory"], resumed_data["boundary_inventory"]
            )
            self.assertEqual(normal_data["obligations"], resumed_data["obligations"])
            self.assertEqual(
                (normal_data["verdict"], normal_checked["verdict"]),
                (resumed_data["verdict"], resumed_checked["verdict"]),
            )
            interrupt_preset = next(
                item
                for item in resumed_data["signals"]
                if (item["source"], item["target"], item["name"])
                == ("BootInitFlow", "CpuGroup.cpus[0].trap.interrupt", "Preset")
            )
            interrupt_facts = set(interrupt_preset["after_snapshot"]["facts"])
            self.assertIn(
                "interrupt_class_gates_closed(CpuGroup.cpus[0].trap.interrupt)",
                interrupt_facts,
            )
            self.assertIn(
                "interrupt_pending_clear_write_completed(CpuGroup.cpus[0].trap.interrupt)",
                interrupt_facts,
            )
            self.assertIn(
                "interrupt_class_gates_closed_before_pending_clear_write_completed(CpuGroup.cpus[0].trap.interrupt)",
                interrupt_facts,
            )
            self.assertNotIn("interrupt_concurrency_closed", interrupt_facts)
            self.assertNotIn(
                "interrupt_total_gate_closed(CpuGroup.cpus[0].trap.interrupt)",
                interrupt_facts,
            )
            self.assertNotIn(
                "interrupt_fallback_ready(CpuGroup.cpus[0].trap.interrupt)",
                interrupt_facts,
            )

            bypass_data, bypass_checked, _, _ = self.run_prepared(
                "bypass-kernel-enable",
                signal="BootInitFlow.Preset",
                scenario=snapshot,
            )
            self.assertEqual(bypass_checked["exit_code"], 1)
            self.assertEqual(bypass_data["signals"][0]["outcome"], "rejected")
            self.assertIn("kernel_enable_accepted", bypass_data["signals"][0]["reason"])

            duplicate_data, duplicate_checked, _, _ = self.run_prepared(
                "duplicate-kernel-enable",
                signal="Kernel.Enable",
                scenario=completed_snapshot,
            )
            self.assertEqual(duplicate_checked["exit_code"], 1)
            self.assertEqual(duplicate_data["signals"][0]["outcome"], "rejected")

            default_work = root / "kernel-default-enable"
            default_result, default_args, _, default_stderr = self.run_shortcut_focused(
                [
                    "-t",
                    "Kernel.Enable",
                    "-u",
                    "BootInitFlow.Preset",
                    "--work-dir",
                    str(default_work),
                ]
            )
            self.assertEqual(default_result, 0, default_stderr)
            self.assertEqual(
                default_args,
                [
                    str(ROOT / "spec" / "model" / "main.spec"),
                    "--source",
                    "OpenSBI",
                    "--max-depth",
                    "all",
                    "--max-breadth",
                    "all",
                    "--signal",
                    "Kernel.Enable",
                    "--scenario",
                    str(KERNEL_ENABLE_SCENARIO.resolve()),
                    "--until",
                    "BootInitFlow.Preset",
                    "--work-dir",
                    str(default_work),
                ],
            )
            default_data, default_checked, _, _ = self.run_prepared(
                "canonical-kernel-auto-source",
                signal="Kernel.Enable",
                source="OpenSBI",
                until="BootInitFlow.Preset",
                scenario=KERNEL_ENABLE_SCENARIO,
            )
            self.assertEqual(default_checked["exit_code"], 0)
            self.assertEqual(default_data["initial_snapshot"], saved["snapshot"])
            self.assertEqual(default_data["root_request"]["source"], "OpenSBI")
            self.assertEqual(default_data["signals"][0]["name"], "Enable")
            self.assertEqual(default_data["verdict"], "reached")

            explicit_source_work = root / "kernel-explicit-source"
            explicit_source, explicit_args, _, explicit_stderr = self.run_shortcut_focused(
                [
                    "-t",
                    "Kernel.Enable",
                    "-u",
                    "BootInitFlow.Preset",
                    "--source",
                    "Human",
                    "--work-dir",
                    str(explicit_source_work),
                ]
            )
            self.assertEqual(explicit_source, 0, explicit_stderr)
            self.assertEqual(
                explicit_args,
                [
                    str(ROOT / "spec" / "model" / "main.spec"),
                    "--source",
                    "Human",
                    "--max-depth",
                    "all",
                    "--max-breadth",
                    "all",
                    "--signal",
                    "Kernel.Enable",
                    "--scenario",
                    str(KERNEL_ENABLE_SCENARIO.resolve()),
                    "--until",
                    "BootInitFlow.Preset",
                    "--work-dir",
                    str(explicit_source_work),
                ],
            )
            explicit_data, explicit_checked, _, _ = self.run_prepared(
                "canonical-kernel-explicit-source",
                signal="Kernel.Enable",
                source="Human",
                until="BootInitFlow.Preset",
                scenario=KERNEL_ENABLE_SCENARIO,
            )
            self.assertEqual(explicit_checked["exit_code"], 0)
            self.assertEqual(explicit_data["root_request"]["source"], "Human")

            stale = subprocess.run(
                [str(shortcut), "-f", str(PIPELINE), "-t", "Kernel.Enable"],
                cwd=root,
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(stale.returncode, 2)
            self.assertIn("snapshot model fingerprint does not match", stale.stderr)

            direct_data, direct_checked, _, _ = self.run_prepared(
                "wrong-model-initial-state", signal="Kernel.Enable", include_view=True
            )
            self.assertEqual(direct_checked["exit_code"], 1)
            self.assertNotEqual(direct_data["initial_snapshot"], saved["snapshot"])
            self.assertEqual(direct_data["signals"][0]["outcome"], "rejected")

    def test_main_model_real_opensbi_handoff_stops_before_boot_init_preset(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            snapshot = root / "boot-init-boundary.snapshot.json"
            derivation, checked, view_data, _ = self.run_prepared(
                "opensbi-handoff",
                until="BootInitFlow.Preset",
                include_view=True,
            )
            self.assertEqual(checked["exit_code"], 0)
            self.assertIsNotNone(view_data)
            self.write_snapshot(snapshot, derivation)
            self.assertEqual(derivation["verdict"], "reached")
            self.assertEqual(
                (
                    derivation["root_request"]["source"],
                    derivation["root_request"]["target"],
                    derivation["root_request"]["signal"],
                ),
                ("Human", "Computer", "Preset"),
            )
            boundary = derivation["boundary"]
            self.assertEqual(
                (
                    boundary["normalized_signal"],
                    boundary["source"],
                    boundary["target"],
                    boundary["send_position"]["delivery"],
                    boundary["send_position"]["cause_id"],
                ),
                ("BootInitFlow.Preset", "Kernel", "BootInitFlow", "drives", "sig-0016"),
            )

            self.assertEqual(len(derivation["signals"]), 19)
            kernel_enable, accept_enable, assign_cpu, physical_activation = derivation["signals"][-4:]
            self.assertEqual(
                (
                    kernel_enable["id"],
                    kernel_enable["source"],
                    kernel_enable["target"],
                    kernel_enable["name"],
                    kernel_enable["delivery"],
                    kernel_enable["cause_id"],
                    kernel_enable["handler"]["id"],
                    kernel_enable["outcome"],
                    kernel_enable["reason"],
                ),
                (
                    "sig-0016",
                    "OpenSBI",
                    "Kernel",
                    "Enable",
                    "emits",
                    "sig-0013",
                    "Kernel.Transition::Enable@Ready",
                    "stopped",
                    "until_signal_reached",
                ),
            )
            self.assertEqual(
                (
                    accept_enable["id"],
                    accept_enable["source"],
                    accept_enable["target"],
                    accept_enable["name"],
                    accept_enable["delivery"],
                    accept_enable["cause_id"],
                    accept_enable["handler"]["id"],
                    accept_enable["handler"]["kind"],
                    accept_enable["outcome"],
                ),
                (
                    "sig-0017",
                    "Kernel",
                    "Kernel",
                    "AcceptEnable",
                    "drives",
                    "sig-0016",
                    "Kernel.Action::AcceptEnable@Ready",
                    "Action",
                    "completed",
                ),
            )
            self.assertEqual(
                (
                    assign_cpu["id"],
                    assign_cpu["source"],
                    assign_cpu["target"],
                    assign_cpu["name"],
                    assign_cpu["delivery"],
                    assign_cpu["cause_id"],
                    assign_cpu["handler"]["kind"],
                    assign_cpu["outcome"],
                ),
                (
                    "sig-0018",
                    "Kernel",
                    "BootInitFlow",
                    "AssignCpuRef",
                    "drives",
                    "sig-0016",
                    "Action",
                    "completed",
                ),
            )
            self.assertEqual(
                (
                    physical_activation["id"],
                    physical_activation["source"],
                    physical_activation["target"],
                    physical_activation["name"],
                    physical_activation["delivery"],
                    physical_activation["cause_id"],
                    physical_activation["handler"]["id"],
                    physical_activation["handler"]["kind"],
                    physical_activation["outcome"],
                ),
                (
                    "sig-0019",
                    "Kernel",
                    "PhysicalDirect",
                    "ActivateOnCpu",
                    "drives",
                    "sig-0016",
                    "PhysicalDirect.Action::ActivateOnCpu@Ready",
                    "Action",
                    "completed",
                ),
            )
            self.assertEqual(
                (
                    kernel_enable["before_snapshot"]["states"]["Kernel"],
                    kernel_enable["after_snapshot"]["states"]["Kernel"],
                    accept_enable["before_snapshot"]["states"]["Kernel"],
                    accept_enable["after_snapshot"]["states"]["Kernel"],
                ),
                ("Ready", "Ready", "Ready", "Ready"),
            )
            self.assertEqual(boundary["snapshot"], physical_activation["after_snapshot"])
            self.assertEqual(boundary["snapshot"], kernel_enable["after_snapshot"])
            boundary_states = boundary["snapshot"]["states"]
            self.assertEqual(
                {
                    name: boundary_states[name]
                    for name in (
                        "Computer",
                        "Riscv64Platform",
                        "OpenSBI",
                        "Kernel",
                        "BootInitFlow",
                        "CpuGroup",
                        "CpuGroup.cpus[0]",
                    )
                },
                {
                    "Computer": "Online",
                    "Riscv64Platform": "Online",
                    "OpenSBI": "Online",
                    "Kernel": "Ready",
                    "BootInitFlow": "Base",
                    "CpuGroup": "Prepared",
                    "CpuGroup.cpus[0]": "Prepared",
                },
            )
            boundary_facts = set(boundary["snapshot"]["facts"])
            for fact in (
                "computer_assembled_from(Riscv64Platform,OpenSBI,Kernel)",
                "assert:BootCpuRegisters.a0 == BootArgs.boot_hartid",
                "assert:BootCpuRegisters.a1 == BootArgs.dtb_pa",
                "assert:BootCpuRegisters.satp == 0",
                "kernel_image_loaded_for_handoff_at(OpenSBI.kernel_load_pa)",
                "kernel_image_load_pmd_aligned(OpenSBI.kernel_load_pa,Config.pmd_size)",
                "ordered_booting_enabled",
                "primary_hart_only_at_kernel_entry",
                "kernel_enable_accepted(Kernel)",
                "task_flow_cpu_ref_is(BootInitFlow,BootCPURef)",
                "cpu_active_translation_controller_for_ref_is(BootCPURef,TranslationControllerKind::PhysicalDirect)",
                "translation_initial_activation_recorded(BootCPURef,TranslationControllerKind::PhysicalDirect,0)",
            ):
                self.assertIn(fact, boundary_facts)
            self.assertFalse(
                any(
                    item["target"] == "BootInitFlow" and item["name"] == "Preset"
                    for item in derivation["signals"]
                )
            )
            self.assertFalse(
                any(
                    event["kind"] == "signal_sent"
                    and event.get("target") == "BootInitFlow"
                    and event.get("signal") == "Preset"
                    for event in derivation["events"]
                )
            )

            kernel_conditions = {
                event["expression"]: event["result"]
                for event in derivation["events"]
                if event["kind"] == "condition_checked"
                and event.get("signal_id") == "sig-0016"
            }
            for expression in (
                "Riscv64Platform.state == State::Online",
                "OpenSBI.state == State::Online",
                "BootCpuRegisters.a0 == BootArgs.boot_hartid",
                "BootCpuRegisters.a1 == BootArgs.dtb_pa",
                "BootCpuRegisters.satp == 0",
                "kernel_image_loaded_for_handoff_at(OpenSBI.kernel_load_pa)",
                "BootInitFlow.state == State::Base",
            ):
                self.assertIs(kernel_conditions[expression], True)

            def event_sequence(kind: str, signal_id: str | None = None) -> int:
                return next(
                    event["sequence"]
                    for event in derivation["events"]
                    if event["kind"] == kind
                    and (signal_id is None or event.get("signal_id") == signal_id)
                )

            self.assertLess(
                event_sequence("signal_received", "sig-0016"),
                event_sequence("response_started", "sig-0016"),
            )
            self.assertLess(
                event_sequence("response_started", "sig-0016"),
                event_sequence("signal_sent", "sig-0017"),
            )
            self.assertLess(
                event_sequence("response_completed", "sig-0018"),
                event_sequence("until_signal_reached"),
            )
            self.assertLess(
                event_sequence("until_signal_reached"),
                event_sequence("response_stopped", "sig-0016"),
            )
            self.assertFalse(
                any(
                    event["kind"] == "response_completed"
                    and event.get("signal_id") == "sig-0016"
                    for event in derivation["events"]
                )
            )

            saved = read_json(snapshot)
            self.assertEqual(saved["snapshot"], boundary["snapshot"])
            self.assertEqual(saved["provenance"]["boundary"], boundary)
            model_data = self.prepared_model_document
            assert view_data is not None
            self.assertEqual(
                {
                    derivation["model_fingerprint"],
                    model_data["model_fingerprint"],
                    view_data["model_fingerprint"],
                    saved["model_fingerprint"],
                },
                {derivation["model_fingerprint"]},
            )
            with mock.patch.dict(os.environ, {"VERBOSE": "0"}):
                compact_text = render_text(view_data)
            with mock.patch.dict(os.environ, {"VERBOSE": "1"}):
                verbose_text = render_text(view_data)
            self.assertIn(
                "boundary: Kernel -- Startup --> BootInitFlow (before send)",
                compact_text,
            )
            self.assertIn(
                "OpenSBI -- Enable --> Kernel[Ready:Ready] !! stopped: until_signal_reached",
                compact_text,
            )
            self.assertIn("Kernel -- AcceptEnable --> Kernel", compact_text)
            self.assertIn(
                "reached boundary: Kernel -> BootInitFlow.Preset [drives]", verbose_text
            )
            self.assertIn(
                "sig-0016 [emits] OpenSBI -> Kernel.Enable", verbose_text
            )
            self.assertIn(
                "sig-0017 [drives] Kernel -> Kernel.AcceptEnable", verbose_text
            )
            self.assertIn(
                "sig-0018 [drives] Kernel -> BootInitFlow.AssignCpuRef", verbose_text
            )
            self.assertIn(
                "sig-0019 [drives] Kernel -> PhysicalDirect.ActivateOnCpu", verbose_text
            )

    def test_main_model_boot_init_preset_reaches_setup_boundary_and_snapshot_resumes(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            shortcut = TOOLS2 / "bin" / "pyveri"
            snapshot = root / "BootInitFlow.Setup.snapshot.json"
            derivation, checked, view, _ = self.run_prepared(
                "boot-init-setup-boundary",
                until="BootInitFlow.Setup",
                include_view=True,
            )
            self.assertEqual(checked["exit_code"], 0)
            self.assertIsNotNone(view)
            self.write_snapshot(snapshot, derivation)
            self.assertEqual(derivation["verdict"], "reached")
            self.assertEqual(
                derivation["summary"],
                {
                    "boundary_occurrences": 2,
                    "completed": 51,
                    "failed": 0,
                    "inventory_deferred": 135,
                    "inventory_trimmed": 57,
                    "pending": 0,
                    "rejected": 0,
                    "signals": 52,
                    "stopped": 1,
                    "truncated": 0,
                    "unresolved_obligations": 0,
                    "yielded": 0,
                },
            )
            self.assertEqual(
                (
                    derivation["root_request"]["source"],
                    derivation["root_request"]["target"],
                    derivation["root_request"]["signal"],
                ),
                ("Human", "Computer", "Preset"),
            )

            expected = [
                (16, "OpenSBI", "Kernel", "Enable", "emits", 13, "Transition", "Ready", "Ready", "stopped"),
                (17, "Kernel", "Kernel", "AcceptEnable", "drives", 16, "Action", "Ready", "Ready", "completed"),
                (18, "Kernel", "BootInitFlow", "AssignCpuRef", "drives", 16, "Action", "Base", "Base", "completed"),
                (19, "Kernel", "PhysicalDirect", "ActivateOnCpu", "drives", 16, "Action", "Ready", "Ready", "completed"),
                (20, "Kernel", "BootInitFlow", "Preset", "drives", 16, "Transition", "Base", "Prepared", "completed"),
                (21, "BootInitFlow", "CpuGroup.cpus[0].trap.interrupt", "Preset", "drives", 20, "Transition", "Base", "Prepared", "completed"),
                (22, "BootInitFlow", "KernelImage", "Preset", "drives", 20, "Transition", "Base", "Prepared", "completed"),
                (23, "BootInitFlow", "CpuGroup.cpus[0]", "DisableFpuVectorExecution", "drives", 20, "Action", "Prepared", "Prepared", "completed"),
                (24, "BootInitFlow", "KernelImage", "Setup", "drives", 20, "Transition", "Prepared", "Ready", "completed"),
                (25, "BootInitFlow", "BootInitFlow", "RecordBootCpuHartid", "drives", 20, "Action", "Base", "Base", "completed"),
                (26, "BootInitFlow", "BootInitFlow", "BindTaskStack", "drives", 20, "Action", "Base", "Base", "completed"),
                (27, "BootInitFlow", "CpuGroup.cpus[0]", "Setup", "drives", 20, "Transition", "Prepared", "Ready", "completed"),
                (28, "BootInitFlow", "CpuGroup.cpus[0].trap.interrupt", "Setup", "drives", 20, "Transition", "Prepared", "Ready", "completed"),
                (29, "BootInitFlow", "CpuGroup.cpus[0].trap", "Preset", "drives", 20, "Transition", "Base", "Prepared", "completed"),
                (30, "BootInitFlow", "Vm", "Preset", "drives", 20, "Transition", "Base", "Prepared", "completed"),
                (31, "Vm", "KernelAddrSpace", "Preset", "drives", 30, "Transition", "Base", "Prepared", "completed"),
                (32, "KernelAddrSpace", "LinearMap", "Preset", "drives", 31, "Transition", "Base", "Ready", "completed"),
                (33, "KernelAddrSpace", "UserSpaceReserve", "Preset", "drives", 31, "Transition", "Base", "Ready", "completed"),
                (34, "Vm", "TrampolineVm", "Setup", "drives", 30, "Transition", "Base", "Ready", "completed"),
                (35, "Vm", "EarlyVm", "Preset", "drives", 30, "Transition", "Base", "Prepared", "completed"),
                (36, "EarlyVm", "RawDtb", "Preset", "drives", 35, "Transition", "Base", "Prepared", "completed"),
                (37, "EarlyVm", "RawDtb", "Setup", "drives", 35, "Transition", "Prepared", "Ready", "completed"),
                (38, "EarlyVm", "FixMap", "Preset", "drives", 35, "Transition", "Base", "Ready", "completed"),
                (39, "Vm", "KernelAddrSpace", "Setup", "drives", 30, "Transition", "Prepared", "Ready", "completed"),
                (40, "Vm", "EarlyVm", "Setup", "drives", 30, "Transition", "Prepared", "Ready", "completed"),
                (41, "BootInitFlow", "Vm", "Setup", "drives", 20, "Transition", "Prepared", "Ready", "completed"),
                (42, "Vm", "TrampolineVm", "ActivateOnCpu", "drives", 41, "Action", "Ready", "Ready", "completed"),
                (43, "Vm", "EarlyVm", "ActivateOnCpu", "drives", 41, "Action", "Ready", "Ready", "completed"),
                (44, "Vm", "KernelImage", "Enable", "drives", 41, "Transition", "Ready", "Online", "completed"),
                (45, "BootInitFlow", "CpuGroup.cpus[0].trap", "Setup", "drives", 20, "Transition", "Prepared", "Ready", "completed"),
                (46, "CpuGroup.cpus[0].trap", "CpuGroup.cpus[0].trap.exception", "Preset", "drives", 45, "Transition", "Base", "Prepared", "completed"),
                (47, "CpuGroup.cpus[0].trap.exception", "CpuGroup.cpus[0].trap.exception.page_fault", "Preset", "drives", 46, "Transition", "Base", "Prepared", "completed"),
                (48, "CpuGroup.cpus[0].trap.exception", "CpuGroup.cpus[0].trap.exception.syscall", "Preset", "drives", 46, "Transition", "Base", "Prepared", "completed"),
                (49, "CpuGroup.cpus[0].trap.exception", "CpuGroup.cpus[0].trap.exception.breakpoint", "Preset", "drives", 46, "Transition", "Base", "Prepared", "completed"),
                (50, "CpuGroup.cpus[0].trap.exception", "CpuGroup.cpus[0].trap.exception.unexpected", "Preset", "drives", 46, "Transition", "Base", "Prepared", "completed"),
                (51, "BootInitFlow", "BootInitFlow", "RefreshTaskStack", "drives", 20, "Action", "Base", "Base", "completed"),
                (52, "BootInitFlow", "Soc", "Preset", "drives", 20, "Transition", "Base", "Prepared", "completed"),
            ]
            actual = []
            for item in derivation["signals"][15:]:
                number = int(item["id"].removeprefix("sig-"))
                cause = int(item["cause_id"].removeprefix("sig-"))
                actual.append(
                    (
                        number,
                        item["source"],
                        item["target"],
                        item["name"],
                        item["delivery"],
                        cause,
                        item["handler"]["kind"],
                        item["before_snapshot"]["states"][item["target"]],
                        item["after_snapshot"]["states"][item["target"]],
                        item["outcome"],
                    )
                )
                handler_member = "Action" if number in {17, 18, 19, 23, 25, 26, 42, 43, 51} else "Transition"
                handler_state = (
                    "process"
                    if number in {18, 23, 25, 26, 51}
                    else item["before_snapshot"]["states"][item["target"]]
                )
                self.assertEqual(
                    item["handler"]["id"],
                    f"{item['target']}.{handler_member}::{item['name']}@{handler_state}",
                )
            self.assertEqual(actual, expected)

            raw_dtb_preset = derivation["signals"][35]
            raw_dtb_setup = derivation["signals"][36]
            self.assertEqual(raw_dtb_preset["id"], "sig-0036")
            self.assertEqual(raw_dtb_setup["id"], "sig-0037")
            self.assertNotIn(
                "valid_dtb_magic(RawDtb.header)",
                raw_dtb_preset["after_snapshot"]["facts"],
            )
            self.assertIn(
                "valid_dtb_magic(RawDtb.header)",
                raw_dtb_setup["after_snapshot"]["facts"],
            )

            vm_preset = derivation["signals"][29]
            vm_setup = derivation["signals"][40]
            self.assertEqual(
                vm_preset["handler"]["description"],
                "准备早期布局与页表 controller，但保持当前 CPU 继续由 PhysicalDirect 承载。",
            )
            self.assertEqual(
                vm_setup["handler"]["description"],
                "依次把 BootCPU 从 PhysicalDirect 交给 TrampolineVm 和 EarlyVm，并恢复保护入口。",
            )
            for fact in (
                "cpu_active_translation_controller_for_ref_is(BootCPURef,TranslationControllerKind::PhysicalDirect)",
                "translation_live_satp_for_ref_is(BootCPURef,0)",
            ):
                self.assertIn(fact, vm_preset["after_snapshot"]["facts"])
            for fact in (
                "cpu_active_translation_controller_for_ref_is(BootCPURef,TranslationControllerKind::EarlyVm)",
                'translation_live_satp_for_ref_is(BootCPURef,"satp_of(EarlyVm.pg_dir, Config.satp_mode)")',
                "vm_transition_stvec_released_to_trap(BootCpuRegisters.stvec,CpuGroup.cpus[0].trap)",
            ):
                self.assertIn(fact, vm_setup["after_snapshot"]["facts"])

            trap_preset = derivation["signals"][28]
            self.assertEqual(trap_preset["id"], "sig-0029")
            self.assertEqual(
                trap_preset["handler"]["description"],
                "为所属 CPU 建立临时保护入口，用于处理意外事件并支持测试和缺陷定位。",
            )
            self.assertIn(
                "trap_flow_type_is_common_interrupt_exception_entry(CpuGroup.cpus[0].trap)",
                trap_preset["before_snapshot"]["facts"],
            )
            for fact in (
                "trap_temporary_protection_entry_ready(CpuGroup.cpus[0].trap,CpuGroup.cpus[0])",
                "trap_temporary_protection_handles_unexpected_events(CpuGroup.cpus[0].trap)",
                "trap_temporary_protection_supports_testing_and_defect_localization(CpuGroup.cpus[0].trap)",
            ):
                self.assertNotIn(fact, trap_preset["before_snapshot"]["facts"])
                self.assertIn(fact, trap_preset["after_snapshot"]["facts"])
            self.assertFalse(
                any(item["cause_id"] == "sig-0029" for item in derivation["signals"])
            )

            trap_setup = derivation["signals"][44]
            exception_preset = derivation["signals"][45]
            self.assertEqual(trap_setup["id"], "sig-0045")
            self.assertEqual(exception_preset["id"], "sig-0046")
            self.assertEqual(exception_preset["cause_id"], "sig-0045")
            self.assertEqual(
                trap_setup["handler"]["description"],
                "把所属 CPU 的异常/中断响应入口重置为正式的 TrapFlowType 响应流入口。",
            )
            for fact in (
                "trap_response_entry_reset_to_formal_trap_flow(CpuGroup.cpus[0].trap,CpuGroup.cpus[0])",
                "trap_formal_entry_creates_fresh_trap_flow(CpuGroup.cpus[0].trap)",
                "exception_initial_fallbacks_prepared(CpuGroup.cpus[0].trap.exception)",
            ):
                self.assertNotIn(fact, trap_setup["before_snapshot"]["facts"])
                self.assertIn(fact, trap_setup["after_snapshot"]["facts"])
            self.assertEqual(derivation["signals"][15]["reason"], "until_signal_reached")
            self.assertEqual(
                derivation["signals"][22]["selector_resolutions"],
                [
                    {
                        "selector": "CurrentCPU",
                        "source_cpu_ref": "BootCPURef",
                        "source_flow": "BootInitFlow",
                        "target": "CpuGroup.cpus[0]",
                    }
                ],
            )
            self.assertEqual(
                " ".join(derivation["signals"][22]["handler"]["description"].split()),
                "关闭 BootCPU 的浮点运算和向量运算能力。内核态默认禁止使用这些能力，只在明确受控的执行区间内才允许临时打开，随即关闭。用户态根据任务需要和系统策略打开。",
            )
            self.assertEqual(
                " ".join(derivation["signals"][23]["handler"]["description"].split()),
                "为内核映像的BSS段清零，让落到该段的全局变量初值为零。清零完成后，BSS 作为普通可写内存使用。",
            )
            self.assertEqual(
                " ".join(derivation["signals"][24]["handler"]["description"].split()),
                "把内核启动时的第一个参数作为BootCPU的hartid记录下来，以备后续使用。",
            )
            self.assertEqual(
                derivation["signals"][26]["selector_resolutions"],
                [
                    {
                        "selector": "CurrentCPU",
                        "source_cpu_ref": "BootCPURef",
                        "source_flow": "BootInitFlow",
                        "target": "CpuGroup.cpus[0]",
                    }
                ],
            )
            first_binding = derivation["signals"][25]["after_snapshot"][
                "contextual_bindings"
            ]["current_task"]["CpuGroup.cpus[0]"]
            self.assertEqual(
                first_binding,
                {
                    "address_view": "TranslationControllerKind::PhysicalDirect",
                    "context_epoch": 0,
                    "dispatch_ordinal": 0,
                    "revision": 1,
                    "source_cpu_ref": "BootCPURef",
                    "source_flow": "BootInitFlow",
                    "task": "BootTask",
                    "task_ref": "BootTaskRef",
                },
            )
            self.assertIn(
                "首次原子建立", derivation["signals"][25]["handler"]["description"]
            )
            first_stack_binding = derivation["signals"][25]["after_snapshot"][
                "contextual_bindings"
            ]["current_stack"]["CpuGroup.cpus[0]"]
            self.assertEqual(first_stack_binding["stack"], "BootTask.stack")
            self.assertEqual(first_stack_binding["address_view"], "TranslationControllerKind::PhysicalDirect")
            self.assertEqual(first_stack_binding["revision"], 1)
            second_binding = derivation["signals"][50]["after_snapshot"][
                "contextual_bindings"
            ]["current_task"]["CpuGroup.cpus[0]"]
            self.assertEqual(second_binding["task"], "BootTask")
            self.assertEqual(second_binding["address_view"], "TranslationControllerKind::EarlyVm")
            self.assertEqual(second_binding["revision"], 2)
            self.assertIn(
                "原子刷新虚拟 tp/sp",
                derivation["signals"][50]["handler"]["description"],
            )
            second_stack_binding = derivation["signals"][50]["after_snapshot"][
                "contextual_bindings"
            ]["current_stack"]["CpuGroup.cpus[0]"]
            self.assertEqual(second_stack_binding["stack"], "BootTask.stack")
            self.assertEqual(second_stack_binding["address_view"], "TranslationControllerKind::EarlyVm")
            self.assertEqual(second_stack_binding["revision"], 2)
            self.assertTrue(
                all("selector_resolution" not in item for item in derivation["signals"])
            )

            boundary = derivation["boundary"]
            self.assertEqual(
                {
                    key: boundary[key]
                    for key in ("kind", "normalized_signal", "source", "target", "signal")
                },
                {
                    "kind": "before_signal_send",
                    "normalized_signal": "BootInitFlow.Setup",
                    "source": "BootInitFlow",
                    "target": "BootInitFlow",
                    "signal": "Setup",
                },
            )
            self.assertEqual(
                (
                    boundary["send_position"]["delivery"],
                    boundary["send_position"]["cause_id"],
                    boundary["send_position"]["fifo_position"],
                ),
                ("emits", "sig-0020", 1),
            )
            self.assertEqual(
                boundary["call_span"],
                {
                    "end_column": 1,
                    "end_line": 166,
                    "source_file": "spec/model/flows/boot_init_flow/main.spec",
                    "start_column": 1,
                    "start_line": 165,
                },
            )
            self.assertEqual(boundary["snapshot"], derivation["signals"][19]["after_snapshot"])
            states = boundary["snapshot"]["states"]
            self.assertEqual(
                {
                    name: states[name]
                    for name in (
                        "Kernel", "BootInitFlow", "CpuGroup.cpus[0].trap.interrupt", "KernelAddrSpace", "KernelImage",
                        "LinearMap", "UserSpaceReserve",
                        "CpuGroup.cpus[0]",
                        "CpuGroup",
                        "CpuGroup.cpus[0].trap", "CpuGroup.cpus[0].trap.exception", "Vm",
                        "TrampolineVm", "EarlyVm", "RawDtb", "FixMap", "Soc",
                        "CpuGroup.cpus[0].trap.exception.page_fault",
                        "CpuGroup.cpus[0].trap.exception.syscall",
                        "CpuGroup.cpus[0].trap.exception.breakpoint",
                        "CpuGroup.cpus[0].trap.exception.unexpected",
                    )
                },
                {
                    "Kernel": "Ready", "BootInitFlow": "Prepared",
                    "CpuGroup.cpus[0].trap.interrupt": "Ready", "KernelAddrSpace": "Ready",
                    "KernelImage": "Online", "LinearMap": "Ready", "UserSpaceReserve": "Ready",
                    "CpuGroup.cpus[0]": "Ready",
                    "CpuGroup": "Prepared",
                    "CpuGroup.cpus[0].trap": "Ready",
                    "CpuGroup.cpus[0].trap.exception": "Prepared", "Vm": "Ready",
                    "TrampolineVm": "Ready", "EarlyVm": "Ready",
                    "RawDtb": "Ready", "FixMap": "Ready", "Soc": "Prepared",
                    "CpuGroup.cpus[0].trap.exception.page_fault": "Prepared",
                    "CpuGroup.cpus[0].trap.exception.syscall": "Prepared",
                    "CpuGroup.cpus[0].trap.exception.breakpoint": "Prepared",
                    "CpuGroup.cpus[0].trap.exception.unexpected": "Prepared",
                },
            )
            self.assertNotIn("BootCurrentCPU", states)
            self.assertNotIn("BootCpuCurrentTask", states)
            self.assertFalse(any("CurrentTaskSlot" in name for name in states))
            self.assertFalse(any("CurrentStackSlot" in name for name in states))
            self.assertFalse(any(name == "Stack" for name in states))
            self.assertEqual(
                boundary["snapshot"]["instances"]["CpuGroup.cpus[0]"]["parent"],
                "CpuGroup",
            )
            resident_parents = {
                name: boundary["snapshot"]["instances"][name]["parent"]
                for name in (
                    "CpuGroup.cpus[0].trap",
                    "CpuGroup.cpus[0].trap.interrupt",
                    "CpuGroup.cpus[0].trap.exception",
                    "CpuGroup.cpus[0].trap.exception.page_fault",
                    "CpuGroup.cpus[0].trap.exception.syscall",
                    "CpuGroup.cpus[0].trap.exception.breakpoint",
                    "CpuGroup.cpus[0].trap.exception.unexpected",
                )
            }
            self.assertEqual(
                resident_parents,
                {
                    "CpuGroup.cpus[0].trap": "CpuGroup.cpus[0]",
                    "CpuGroup.cpus[0].trap.interrupt": "CpuGroup.cpus[0].trap",
                    "CpuGroup.cpus[0].trap.exception": "CpuGroup.cpus[0].trap",
                    "CpuGroup.cpus[0].trap.exception.page_fault":
                        "CpuGroup.cpus[0].trap.exception",
                    "CpuGroup.cpus[0].trap.exception.syscall":
                        "CpuGroup.cpus[0].trap.exception",
                    "CpuGroup.cpus[0].trap.exception.breakpoint":
                        "CpuGroup.cpus[0].trap.exception",
                    "CpuGroup.cpus[0].trap.exception.unexpected":
                        "CpuGroup.cpus[0].trap.exception",
                },
            )
            facts = set(boundary["snapshot"]["facts"])
            for fact in (
                "kernel_enable_accepted(Kernel)",
                "task_flow_started(BootInitFlow)",
                "interrupt_concurrency_closed",
                "interrupt_class_gates_closed(CpuGroup.cpus[0].trap.interrupt)",
                "interrupt_pending_clear_write_completed(CpuGroup.cpus[0].trap.interrupt)",
                "interrupt_class_gates_closed_before_pending_clear_write_completed(CpuGroup.cpus[0].trap.interrupt)",
                "gp_relative_addressing_ready(KernelImage)",
                "kernel_image_bss_zeroing_completed(KernelImage)",
                "kernel_image_bss_ordinary_writable(KernelImage)",
                "boot_cpu_hartid_recorded_for_later_use(BootCpuRegisters.a0)",
                "cpu_fpu_execution_disabled(CpuGroup.cpus[0])",
                "cpu_vector_execution_disabled(CpuGroup.cpus[0])",
                "cpu_kernel_fpu_vector_default_disabled(CpuGroup.cpus[0])",
                "cpu_kernel_fpu_vector_temporary_enable_requires_controlled_scope(CpuGroup.cpus[0])",
                "cpu_kernel_fpu_vector_disabled_after_controlled_scope(CpuGroup.cpus[0])",
                "cpu_user_fpu_vector_enable_follows_task_need_and_system_policy(CpuGroup.cpus[0])",
                "boot_task_stack_current_binding_established(CpuGroup.cpus[0],BootTask,BootTask.stack)",
                "boot_task_stack_current_binding_refreshed_for_active_controller(CpuGroup.cpus[0],BootTask,BootTask.stack)",
                "current_stack_binding_committed(CpuGroup.cpus[0],BootTask,BootTask.stack)",
                "current_stack_binding_address_view_is(CpuGroup.cpus[0],BootTask.stack,TranslationControllerKind::EarlyVm)",
                "current_stack_binding_revision_is(CpuGroup.cpus[0],2)",
                "current_stack_pointer_matches_active_controller(CpuGroup.cpus[0],BootTask,BootTask.stack)",
                "assert:BootCpuRegisters.satp == satp_of(EarlyVm.pg_dir, Config.satp_mode)",
                "boot_task_current_binding_established(CpuGroup.cpus[0],BootTask)",
                "boot_task_current_binding_refreshed_for_active_controller(CpuGroup.cpus[0],BootTask)",
                "boot_task_preemption_is_static_initial_property(BootTask)",
                "current_task_binding_committed(CpuGroup.cpus[0],BootTask,BootInitFlow)",
                "current_task_binding_ref_is(CpuGroup.cpus[0],BootTaskRef)",
                "current_task_binding_address_view_is(CpuGroup.cpus[0],BootTask,TranslationControllerKind::EarlyVm)",
                "current_task_binding_revision_is(CpuGroup.cpus[0],2)",
                "cpu_active_translation_controller_for_ref_is(BootCPURef,TranslationControllerKind::EarlyVm)",
                "trap_flow_type_is_common_interrupt_exception_entry(CpuGroup.cpus[0].trap)",
                "trap_common_entry_routes_by_event_class(CpuGroup.cpus[0].trap,CpuGroup.cpus[0].trap.interrupt,CpuGroup.cpus[0].trap.exception)",
                "trap_temporary_protection_entry_ready(CpuGroup.cpus[0].trap,CpuGroup.cpus[0])",
                "trap_temporary_protection_handles_unexpected_events(CpuGroup.cpus[0].trap)",
                "trap_temporary_protection_supports_testing_and_defect_localization(CpuGroup.cpus[0].trap)",
                "trap_formal_entry_ready(CpuGroup.cpus[0].trap)",
                "early_vm_translation_sync_complete(EarlyVm,BootCPURef)",
                "translation_controller_retired_for_cpu(TrampolineVm,BootCPURef)",
                "kernel_addr_space_regions_disjoint(KernelImage.virt_range,FixMap,LinearMap,UserSpaceReserve)",
                "valid_dtb_header(RawDtb.header)",
                "valid_dtb_magic(RawDtb.header)",
                "assert:header.total_size >= size_of::<DtbHeader>()",
                "dtb_range_addition_safe(BootArgs.dtb_pa,header.total_size)",
                "fits_in_fixmap_slot(RawDtb.range,Config.fixmap.fdt,Config.page_size)",
                "firmware_dtb_range_accessible_from_handoff_contract(RawDtb.range)",
                "raw_dtb_nodes_unparsed(RawDtb)",
                "fixmap_slot_mapping_ready(EarlyVm.pg_dir,FixMap.fdt_slot)",
                "soc_preset_semantics_deferred(Soc)",
            ):
                self.assertIn(fact, facts)
            self.assertFalse(any(fact.startswith("cpu_hartid_ready(") for fact in facts))
            self.assertFalse(
                any(
                    item["target"] == "BootInitFlow" and item["name"] == "Setup"
                    for item in derivation["signals"]
                )
            )
            self.assertNotIn("memory_zeroed(segments.bss.range)", facts)
            self.assertNotIn(
                "trap_early_fatal_prelude_ready(CpuGroup.cpus[0].trap)", facts
            )
            self.assertNotIn(
                "trap_prelude_creates_no_flow_occurrence(CpuGroup.cpus[0].trap)",
                facts,
            )

            context_events = [
                event
                for event in derivation["events"]
                if event["kind"] in {"context_entered", "context_exited"}
            ]
            self.assertEqual(
                [
                    (event["kind"], event["signal_id"], event["context"], event["stack"])
                    for event in context_events
                ],
                [
                    ("context_entered", "sig-0020", "SingleTaskContext", ["SingleTaskContext"]),
                    ("context_exited", "sig-0020", "SingleTaskContext", ["SingleTaskContext"]),
                ],
            )

            def sequence(kind: str, signal_id: str | None = None) -> int:
                return next(
                    event["sequence"]
                    for event in derivation["events"]
                    if event["kind"] == kind
                    and (signal_id is None or event.get("signal_id") == signal_id)
                )

            self.assertLess(context_events[0]["sequence"], sequence("signal_sent", "sig-0021"))
            self.assertLess(sequence("response_completed", "sig-0052"), context_events[1]["sequence"])
            self.assertLess(context_events[1]["sequence"], sequence("response_completed", "sig-0020"))
            self.assertLess(sequence("response_completed", "sig-0020"), sequence("until_signal_reached"))
            self.assertLess(sequence("until_signal_reached"), sequence("response_stopped", "sig-0016"))
            self.assertEqual(
                [
                    event["identity"]
                    for event in derivation["events"]
                    if event["kind"] == "indexed_instance_declared"
                ],
                ["CpuGroup.cpus[0]"],
            )
            self.assertFalse(
                any(event["kind"] == "indexed_transaction_rolled_back" for event in derivation["events"])
            )

            saved = read_json(snapshot)
            model = self.prepared_model_document
            assert view is not None
            boot_init_setup_inventory = {
                item["id"]: item["status"]
                for item in model["model"]["boundary_inventory"]
                if item["id"].startswith("boot_init_setup.")
            }
            self.assertEqual(
                boot_init_setup_inventory,
                {
                    "boot_init_setup.001": "deferred",
                    "boot_init_setup.002": "trimmed",
                    "boot_init_setup.003": "trimmed",
                    "boot_init_setup.004": "trimmed",
                    "boot_init_setup.005": "trimmed",
                    "boot_init_setup.006": "trimmed",
                    "boot_init_setup.007": "trimmed",
                    "boot_init_setup.008": "trimmed",
                    "boot_init_setup.009": "trimmed",
                    "boot_init_setup.010": "trimmed",
                    "boot_init_setup.011": "trimmed",
                    "boot_init_setup.012": "trimmed",
                    "boot_init_setup.013": "trimmed",
                    "boot_init_setup.014": "deferred",
                    "boot_init_setup.015": "deferred",
                    "boot_init_setup.016": "deferred",
                    "boot_init_setup.017": "deferred",
                },
            )
            self.assertEqual(saved["snapshot"], boundary["snapshot"])
            self.assertEqual(saved["provenance"]["boundary"], boundary)
            self.assertEqual(snapshot.read_bytes(), BOOT_INIT_SETUP_SCENARIO.read_bytes())
            self.assertEqual(
                hashlib.sha256(snapshot.read_bytes()).hexdigest(),
                "86cb8bc5d4a3d608c2f4a6d88f738c3b6a894687a54e0c5ed0bd5968083b4adf",
            )
            self.assertEqual(
                {
                    derivation["model_fingerprint"], model["model_fingerprint"],
                    view["model_fingerprint"], saved["model_fingerprint"],
                },
                {"sha256:3fc93c292767cc261a217f9e397bbfa2d4df610608173cccfb3dd734fae5ebc8"},
            )
            with mock.patch.dict(os.environ, {"VERBOSE": "0"}):
                compact_text = render_text(view)
            with mock.patch.dict(os.environ, {"VERBOSE": "1"}):
                verbose_text = render_text(view)
            self.assertTrue(compact_text.startswith("verdict: reached\nboundaries: "))
            self.assertIn(
                "boundary: BootInitFlow -- Setup --> BootInitFlow (before send)\n",
                compact_text,
            )
            self.assertIn(
                "OpenSBI -- Enable --> Kernel[Ready:Ready] !! stopped: until_signal_reached",
                compact_text,
            )
            self.assertIn("Kernel -- Startup --> BootInitFlow[Base:Prepared]", compact_text)
            self.assertIn("BootInitFlow -- Startup --> Soc[Base:Prepared]", compact_text)
            self.assertIn("Signal derivation: Human -> Computer.Preset", verbose_text)
            self.assertIn("sig-0020 [drives] Kernel -> BootInitFlow.Preset", verbose_text)
            self.assertIn("handler: BootInitFlow.Transition::Preset@Base", verbose_text)
            self.assertIn(
                "sig-0023 [drives] BootInitFlow -> CpuGroup.cpus[0].DisableFpuVectorExecution",
                verbose_text,
            )
            self.assertIn(
                "sig-0025 [drives] BootInitFlow -> BootInitFlow.RecordBootCpuHartid",
                verbose_text,
            )
            self.assertIn("sig-0052 [drives] BootInitFlow -> Soc.Preset", verbose_text)
            self.assertIn(
                "reached boundary: BootInitFlow -> BootInitFlow.Setup [emits]",
                verbose_text,
            )

            default_work = root / "default-setup"
            default, default_args, _, default_stderr = self.run_shortcut_focused(
                [
                    "-t", "BootInitFlow.Setup", "--max-depth", "0",
                    "--work-dir", str(default_work),
                ],
                downstream_exit=1,
            )
            self.assertEqual(default, 1, default_stderr)
            self.assertEqual(
                default_args,
                [
                    str(ROOT / "spec" / "model" / "main.spec"),
                    "--source",
                    "BootInitFlow",
                    "--max-depth",
                    "0",
                    "--max-breadth",
                    "all",
                    "--signal",
                    "BootInitFlow.Setup",
                    "--scenario",
                    str(BOOT_INIT_SETUP_SCENARIO.resolve()),
                    "--work-dir",
                    str(default_work),
                ],
            )
            default_data, default_checked, _, _ = self.run_prepared(
                "canonical-setup-bounded",
                signal="BootInitFlow.Setup",
                source="BootInitFlow",
                scenario=BOOT_INIT_SETUP_SCENARIO,
                max_depth=0,
            )
            self.assertEqual(default_checked["exit_code"], 1)
            self.assertEqual(default_data["initial_snapshot"], saved["snapshot"])
            self.assertEqual(
                (
                    default_data["signals"][0]["target"],
                    default_data["signals"][0]["name"],
                    default_data["signals"][0]["outcome"],
                ),
                ("BootInitFlow", "Setup", "failed"),
            )
            self.assertTrue(
                any(item["outcome"] == "truncated" for item in default_data["signals"])
            )

            duplicate_data, duplicate_checked, _, _ = self.run_prepared(
                "duplicate-boot-init-preset",
                signal="BootInitFlow.Preset",
                scenario=BOOT_INIT_SETUP_SCENARIO,
            )
            self.assertEqual(duplicate_checked["exit_code"], 1)
            self.assertEqual(duplicate_data["signals"][0]["outcome"], "rejected")

            stale = subprocess.run(
                [str(shortcut), "-f", str(PIPELINE), "-t", "BootInitFlow.Setup"],
                cwd=root,
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(stale.returncode, 2)
            self.assertIn("snapshot model fingerprint does not match", stale.stderr)

            missing, missing_args, _, missing_stderr = self.run_shortcut_focused(
                ["-t", "BootInitFlow.Enable"]
            )
            self.assertEqual(missing, 2)
            self.assertIsNone(missing_args)
            self.assertIn(
                "tools2/scenarios/BootInitFlow.Enable.snapshot.json", missing_stderr
            )

    def test_main_model_boot_init_setup_stops_at_setup_arch_return(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            snapshot = root / "CorePreparePhase.Preset.snapshot.json"
            derivation, checked, view, _ = self.run_prepared(
                "setup-arch-return",
                signal="BootInitFlow.Setup",
                source="BootInitFlow",
                until="CorePreparePhase.Preset",
                scenario=BOOT_INIT_SETUP_SCENARIO,
                include_view=True,
            )
            self.assertEqual(checked["exit_code"], 0)
            self.assertIsNotNone(view)
            self.write_snapshot(snapshot, derivation)
            self.assertEqual(derivation["verdict"], "reached")
            self.assertEqual(
                derivation["summary"],
                {
                    "boundary_occurrences": 2,
                    "completed": 61,
                    "failed": 0,
                    "inventory_deferred": 135,
                    "inventory_trimmed": 57,
                    "pending": 0,
                    "rejected": 0,
                    "signals": 62,
                    "stopped": 1,
                    "truncated": 0,
                    "unresolved_obligations": 0,
                    "yielded": 0,
                },
            )
            self.assertEqual(
                (
                    derivation["root_request"]["source"],
                    derivation["root_request"]["target"],
                    derivation["root_request"]["signal"],
                ),
                ("BootInitFlow", "BootInitFlow", "Setup"),
            )
            boundary = derivation["boundary"]
            self.assertEqual(
                {
                    key: boundary[key]
                    for key in ("kind", "normalized_signal", "source", "target", "signal")
                },
                {
                    "kind": "before_signal_send",
                    "normalized_signal": "CorePreparePhase.Preset",
                    "source": "BootInitFlow",
                    "target": "CorePreparePhase",
                    "signal": "Preset",
                },
            )
            self.assertEqual(
                (
                    boundary["send_position"]["delivery"],
                    boundary["send_position"]["cause_id"],
                ),
                ("drives", "sig-0001"),
            )

            direct = [
                (item["id"], item["source"], item["target"], item["name"],
                 item["delivery"], item["cause_id"], item["handler"]["id"],
                 item["before_snapshot"]["states"][item["target"]],
                 item["after_snapshot"]["states"][item["target"]])
                for item in derivation["signals"]
                if item.get("cause_id") == "sig-0001" and item["outcome"] == "completed"
            ]
            self.assertEqual(
                direct,
                [
                    ("sig-0002", "BootInitFlow", "BootTask", "EnableStackGuard", "drives", "sig-0001", "BootTask.Action::EnableStackGuard@process", "OnCpu", "OnCpu"),
                    ("sig-0003", "BootInitFlow", "EarlyDtb", "Preset", "drives", "sig-0001", "EarlyDtb.Transition::Preset@Base", "Base", "Prepared"),
                    ("sig-0008", "BootInitFlow", "CpuGroup.cpus[0]", "Enable", "drives", "sig-0001", "CpuGroup.cpus[0].Transition::Enable@Ready", "Ready", "Online"),
                    ("sig-0009", "BootInitFlow", "PrintkBuffer", "Preset", "drives", "sig-0001", "PrintkBuffer.Transition::Preset@Base", "Base", "Prepared"),
                    ("sig-0010", "BootInitFlow", "EarlyDtb", "Setup", "drives", "sig-0001", "EarlyDtb.Transition::Setup@Prepared", "Prepared", "Ready"),
                    ("sig-0014", "BootInitFlow", "InitMM", "Setup", "drives", "sig-0001", "InitMM.Transition::Setup@Base", "Base", "Ready"),
                    ("sig-0015", "BootInitFlow", "EarlyIoremap", "Setup", "drives", "sig-0001", "EarlyIoremap.Transition::Setup@Base", "Base", "Ready"),
                    ("sig-0016", "BootInitFlow", "SBI", "Setup", "drives", "sig-0001", "SBI.Transition::Setup@Base", "Base", "Ready"),
                    ("sig-0017", "BootInitFlow", "Params", "Preset", "drives", "sig-0001", "Params.Transition::Preset@Base", "Base", "Prepared"),
                    ("sig-0025", "BootInitFlow", "MemBlock", "Setup", "drives", "sig-0001", "MemBlock.Transition::Setup@Prepared", "Prepared", "Ready"),
                    ("sig-0026", "BootInitFlow", "Vm", "Enable", "drives", "sig-0001", "Vm.Transition::Enable@Ready", "Ready", "Online"),
                    ("sig-0030", "BootInitFlow", "MemBlock", "Enable", "drives", "sig-0001", "MemBlock.Transition::Enable@Ready", "Ready", "Online"),
                    ("sig-0031", "BootInitFlow", "EarlyDtb", "Cleanup", "drives", "sig-0001", "EarlyDtb.Transition::Cleanup@Ready", "Ready", "Destroyed"),
                    ("sig-0032", "BootInitFlow", "DeviceTree", "Setup", "drives", "sig-0001", "DeviceTree.Transition::Setup@Base", "Base", "Ready"),
                    ("sig-0033", "BootInitFlow", "Zones", "Setup", "drives", "sig-0001", "Zones.Transition::Setup@Base", "Base", "Ready"),
                    ("sig-0034", "BootInitFlow", "PageMetadataMap", "Setup", "drives", "sig-0001", "PageMetadataMap.Transition::Setup@Base", "Base", "Ready"),
                    ("sig-0035", "BootInitFlow", "ResourceLock", "Preset", "drives", "sig-0001", "ResourceLock.Transition::Preset@Base", "Base", "Prepared"),
                    ("sig-0036", "BootInitFlow", "ResourceLock", "Setup", "drives", "sig-0001", "ResourceLock.Transition::Setup@Prepared", "Prepared", "Ready"),
                    ("sig-0037", "BootInitFlow", "ResourceTree", "Setup", "drives", "sig-0001", "ResourceTree.Transition::Setup@Base", "Base", "Ready"),
                    ("sig-0038", "BootInitFlow", "CpuGroup", "Setup", "drives", "sig-0001", "CpuGroup.Transition::Setup@Prepared", "Prepared", "Ready"),
                    ("sig-0060", "BootInitFlow", "CacheBlockInfo", "Setup", "drives", "sig-0001", "CacheBlockInfo.Transition::Setup@Base", "Base", "Ready"),
                    ("sig-0061", "BootInitFlow", "CpuCapabilities", "Setup", "drives", "sig-0001", "CpuCapabilities.Transition::Setup@Base", "Base", "Ready"),
                    ("sig-0062", "BootInitFlow", "DmaCachePolicy", "Setup", "drives", "sig-0001", "DmaCachePolicy.Transition::Setup@Base", "Base", "Ready"),
                ],
            )

            saved = read_json(snapshot)
            self.assertEqual(saved["snapshot"], boundary["snapshot"])
            states = saved["snapshot"]["states"]
            self.assertEqual(states["BootInitFlow"], "Prepared")
            self.assertEqual(states["CorePreparePhase"], "Base")
            for name in (
                "DeviceTree", "Zones", "PageMetadataMap", "ResourceLock",
                "ResourceTree", "CpuGroup", "CacheBlockInfo", "CpuCapabilities",
                "DmaCachePolicy",
            ):
                self.assertEqual(states[name], "Ready")
            self.assertEqual(states["EarlyDtb"], "Destroyed")
            self.assertEqual(states["MemBlock"], "Online")
            self.assertEqual(states["Vm"], "Online")
            self.assertEqual(states["SwapperVm"], "Ready")
            self.assertEqual(states["KernelAddrSpace"], "Online")
            self.assertEqual(states["PerCpuStorage"], "Base")
            self.assertEqual(states["CommandLine"], "Prepared")
            self.assertEqual(states["Params"], "Prepared")
            self.assertEqual(states["BootParam"], "Base")
            self.assertEqual(states["PayloadParam"], "Base")
            self.assertEqual(states["CpuGroup.cpus[0].trap.exception"], "Prepared")
            facts = set(saved["snapshot"]["facts"])
            for fact in (
                "task_stack_guard_ready(BootTask,BootTask.stack)",
                "task_stack_range_valid(BootTask,BootTask.stack)",
                "memblock_resize_allowed(MemBlock)",
                "swapper_vm_translation_sync_complete(SwapperVm,BootCPURef)",
                "device_tree_unflattened_from_raw_dtb(DeviceTree,RawDtb)",
                "zones_ready(Zones,MemBlock)",
                "page_metadata_map_ready(PageMetadataMap,Zones)",
                "resource_tree_ready(ResourceTree,MemBlock)",
                "cache_block_info_ready(CacheBlockInfo,DeviceTree,CpuGroup)",
                "cpu_capabilities_ready(CpuCapabilities,DeviceTree,CpuGroup)",
                "dma_cache_policy_ready(DmaCachePolicy,CpuCapabilities,CacheBlockInfo)",
            ):
                self.assertIn(fact, facts)

            assert view is not None
            rendered_text = render_text(view)
            rendered_html = render_html(
                build_animation(self.prepared_model_document, view)
            )
            serialized = json.dumps(derivation, sort_keys=True)
            self.assertNotIn("EntrySuccessorPhase", serialized)
            self.assertNotIn("EntrySuccessorPhase", rendered_text)
            self.assertNotIn("EntrySuccessorPhase", rendered_html)

    def test_main_model_schedule_presend_has_unique_cpu_schedulers(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            snapshot = root / "Cpu0Scheduler.Schedule.snapshot.json"
            derivation, checked, view, _ = self.run_prepared(
                "cpu0-scheduler-schedule-boundary",
                until="Cpu0Scheduler.Schedule",
                include_view=True,
            )
            self.assertEqual(checked["exit_code"], 0)
            self.assertIsNotNone(view)
            self.write_snapshot(snapshot, derivation)
            self.assertEqual(derivation["verdict"], "reached")
            self.assertEqual(
                derivation["summary"],
                {
                    "boundary_occurrences": 120,
                    "completed": 342,
                    "failed": 0,
                    "inventory_deferred": 135,
                    "inventory_trimmed": 57,
                    "pending": 0,
                    "rejected": 0,
                    "signals": 344,
                    "stopped": 2,
                    "truncated": 0,
                    "unresolved_obligations": 0,
                    "yielded": 0,
                },
            )
            boundary = derivation["boundary"]
            self.assertEqual(
                (
                    boundary["kind"],
                    boundary["normalized_signal"],
                    boundary["source"],
                    boundary["target"],
                    boundary["send_position"]["delivery"],
                ),
                (
                    "before_signal_send",
                    "Cpu0Scheduler.Schedule",
                    "BootInitFlow",
                    "Cpu0Scheduler",
                    "yields",
                ),
            )
            states = boundary["snapshot"]["states"]
            self.assertEqual(
                (states["BootInitFlow"], states["BootTask"]),
                ("Online", "OnCpu"),
            )
            references = boundary["snapshot"]["references"]
            for index in range(8):
                scheduler = f"Cpu{index}Scheduler"
                self.assertEqual(
                    references[f"CpuGroup.cpus[{index}].scheduler"], scheduler
                )
                self.assertEqual(
                    states[scheduler], "Online" if index == 0 else "Ready"
                )
                self.assertNotIn(f"CpuGroup.cpus[{index}].scheduler", states)
            self.assertFalse(
                any(
                    item["target"] == "Cpu0Scheduler" and item["name"] == "Schedule"
                    for item in derivation["signals"]
                )
            )
            self.assertEqual(
                (
                    derivation["signals"][-1]["source"],
                    derivation["signals"][-1]["target"],
                    derivation["signals"][-1]["name"],
                    derivation["signals"][-1]["outcome"],
                ),
                ("BootInitFlow", "BootInitFlow", "RequestSchedule", "stopped"),
            )
            self.assertEqual(
                snapshot.read_bytes(), CPU0_SCHEDULER_SCHEDULE_SCENARIO.read_bytes()
            )
            self.assertEqual(
                hashlib.sha256(snapshot.read_bytes()).hexdigest(),
                "fdffe2f8888418b5d59e7afc6a404f5a57a7678719ebed8ae86e5bc7dee4c9ab",
            )
            model = self.prepared_model_document
            assert view is not None
            saved = read_json(snapshot)
            self.assertEqual(
                {
                    derivation["model_fingerprint"],
                    model["model_fingerprint"],
                    view["model_fingerprint"],
                    saved["model_fingerprint"],
                },
                {"sha256:3fc93c292767cc261a217f9e397bbfa2d4df610608173cccfb3dd734fae5ebc8"},
            )

    def test_main_model_all_cpu_schedulers_expose_ap_mailbox_ipi_idle_protocol(self) -> None:
        model = self.prepared_model_document["model"]

        def facts(handler: dict, section: str) -> set[str]:
            return {
                entry["name"]
                for item in handler["body"]
                if item["kind"] == section
                for entry in item["entries"]
                if entry["kind"] == "fact"
            }

        required_handlers = {
            "PublishInbound",
            "ConsumeInbound",
            "MarkNeedResched",
            "RunIdle",
        }
        for index in range(8):
            scheduler = model["systems"][f"Cpu{index}Scheduler"]
            handlers = scheduler["handlers_by_name"]
            self.assertTrue(required_handlers.issubset(handlers))

            publish = handlers["PublishInbound"][0]
            self.assertTrue(
                {
                    "scheduler_mailbox_message_published_release",
                    "scheduler_reschedule_ipi_requested_after_release",
                    "scheduler_mailbox_rejects_stale_wrong_target_or_duplicate",
                }.issubset(facts(publish, "ensures"))
            )

            consume = handlers["ConsumeInbound"][0]
            self.assertTrue(
                {
                    "scheduler_mailbox_message_published_release",
                    "scheduler_mailbox_message_target_and_generation_valid",
                    "scheduler_mailbox_ordinal_fresh",
                }.issubset(facts(consume, "depends_on"))
            )
            self.assertTrue(
                {
                    "scheduler_mailbox_message_consumed_once",
                    "scheduler_mailbox_rejects_stale_wrong_target_or_duplicate",
                }.issubset(facts(consume, "ensures"))
            )

            run_idle = handlers["RunIdle"][0]
            self.assertTrue(
                {
                    "scheduler_idle_sleep_recheck_complete",
                    "scheduler_idle_wfi_only_without_visible_work",
                    "scheduler_idle_uses_common_switch_protocol",
                }.issubset(facts(run_idle, "ensures"))
            )

        interrupt_process = next(
            process
            for process in model["types"]["InterruptType"]["effective_processes"]
            if process["name"] == "HandleRescheduleIpi"
        )
        self.assertTrue(
            {
                "interrupt_ssip_pending_cleared",
                "interrupt_need_resched_recorded_cpu_local",
                "interrupt_reschedule_ipi_does_not_switch_in_handler",
                "interrupt_duplicate_reschedule_ipi_coalesced",
            }.issubset(facts(interrupt_process, "ensures"))
        )

    def test_main_model_trap_overlay_and_page_fault_resume_contract(self) -> None:
        model = self.prepared_model_document["model"]

        def facts(handler: dict, section: str = "ensures") -> set[str]:
            return {
                entry["name"]
                for item in handler["body"]
                if item["kind"] == section
                for entry in item["entries"]
                if entry["kind"] == "fact"
            }

        def state_handler(type_name: str, state_name: str, name: str) -> dict:
            state = model["types"][type_name]["states"][state_name]
            return next(handler for handler in state["handlers"] if handler["name"] == name)

        def process_handler(type_name: str, name: str) -> dict:
            return next(
                handler
                for handler in model["types"][type_name]["effective_processes"]
                if handler["name"] == name
            )

        self.assertTrue(
            {
                "trap_runtime_lease_cpu_local",
                "trap_runtime_lease_has_narrow_task_access",
                "trap_exception_table_access_read_only",
                "trap_observation_per_cpu_stable",
            }.issubset(facts(state_handler("TrapType", "Prepared", "Setup")))
        )
        self.assertTrue(
            {
                "trap_return_token_created_once",
                "trap_formal_occurrence_never_bypassed",
            }.issubset(facts(state_handler("TrapFlowType", "Ready", "Enable")))
        )
        self.assertTrue(
            {
                "interrupt_reschedule_ipi_uses_formal_overlay",
                "interrupt_reschedule_ipi_does_not_consume_mailbox",
                "interrupt_reschedule_ipi_does_not_switch_in_handler",
                "interrupt_duplicate_reschedule_ipi_coalesced",
            }.issubset(facts(process_handler("InterruptType", "HandleRescheduleIpi")))
        )
        self.assertTrue(
            {
                "page_fault_flow_user_recovery_can_schedule",
                "page_fault_flow_kernel_task_context_fixup_can_schedule",
                "page_fault_flow_atomic_fixup_or_fatal",
                "page_fault_flow_nested_hardirq_or_irqoff_never_schedules",
                "page_fault_flow_missing_or_stale_fixup_is_terminal",
            }.issubset(
                facts(state_handler("PageFaultExceptionFlowType", "Prepared", "Setup"))
            )
        )
        self.assertIn(
            "page_fault_flow_root_leaf_revalidated_after_schedule",
            facts(state_handler("PageFaultExceptionFlowType", "Ready", "Enable")),
        )
        self.assertTrue(
            {
                "task_flow_optional_root_preflight_valid",
                "task_flow_active_trap_leaf_resumed_exactly_once_or_absent",
                "task_flow_enter_does_not_select_machine_coordinate",
            }.issubset(facts(process_handler("TaskFlow", "Enter")))
        )
        self.assertTrue(
            {
                "scheduler_optional_root_trap_preflight_valid",
                "scheduler_active_trap_leaf_generation_valid",
                "scheduler_trap_leaf_context_epoch_matches",
            }.issubset(facts(process_handler("Scheduler", "PreflightNextDispatch")))
        )

    def test_main_model_boot_init_entry_stops_at_first_missing_guard(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)

            def scenario_without(fact: str, name: str) -> Path:
                scenario = deepcopy(read_json(KERNEL_ENABLE_SCENARIO))
                scenario["snapshot"]["facts"].remove(fact)
                path = root / f"{name}.snapshot.json"
                path.write_text(json.dumps(scenario, indent=2, sort_keys=True) + "\n", encoding="utf-8")
                return path

            def derive_case(name: str, scenario: Path) -> dict:
                data, checked, _, _ = self.run_prepared(
                    name,
                    signal="Kernel.Enable",
                    scenario=scenario,
                )
                self.assertEqual(checked["exit_code"], 1)
                self.assertEqual(data["last_stable_snapshot"]["states"]["Kernel"], "Ready")
                self.assertEqual(data["last_stable_snapshot"]["states"]["BootInitFlow"], "Base")
                self.assertNotIn("task_flow_started(BootInitFlow)", data["last_stable_snapshot"]["facts"])
                self.assertFalse(
                    any(
                        item["target"] == "BootInitFlow" and item["name"] == "Setup"
                        for item in data["signals"]
                    )
                )
                return data

            bypass_data, bypass_checked, _, _ = self.run_prepared(
                "bypass-boot-init-entry",
                signal="BootInitFlow.Preset",
                scenario=KERNEL_ENABLE_SCENARIO,
            )
            self.assertEqual(bypass_checked["exit_code"], 1)
            self.assertEqual(len(bypass_data["signals"]), 1)
            self.assertEqual(bypass_data["signals"][0]["outcome"], "rejected")
            self.assertIn("kernel_enable_accepted", bypass_data["signals"][0]["reason"])

            cases = (
                (
                    "missing-task-concurrency",
                    "task_concurrency_closed",
                    ("BootInitFlow", "Preset"),
                    None,
                ),
                (
                    "missing-entry-satp",
                    "assert:BootCpuRegisters.satp == 0",
                    ("Kernel", "Enable"),
                    None,
                ),
                (
                    "missing-raw-dtb-access",
                    "firmware_dtb_blob_accessible_at_kernel_entry(BootArgs.dtb_pa)",
                    ("RawDtb", "Preset"),
                    ("RawDtb", "Setup"),
                ),
            )
            for name, fact, rejected_signal, forbidden_later in cases:
                with self.subTest(name=name):
                    data = derive_case(name, scenario_without(fact, name))
                    rejected = [
                        item for item in data["signals"] if item["outcome"] == "rejected"
                    ]
                    self.assertEqual(len(rejected), 1)
                    self.assertEqual(
                        (rejected[0]["target"], rejected[0]["name"]), rejected_signal
                    )
                    rejected_index = data["signals"].index(rejected[0])
                    self.assertTrue(
                        all(
                            item["outcome"] == "failed"
                            for item in data["signals"][rejected_index + 1 :]
                        )
                    )
                    if forbidden_later is not None:
                        self.assertFalse(
                            any(
                                (item["target"], item["name"]) == forbidden_later
                                for item in data["signals"]
                            )
                        )

    def test_main_model_enable_chain_rejects_missing_prerequisites_strictly(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            cases = [
                (
                    "Computer.Enable",
                    {
                        "states": {
                            "Riscv64Platform": "Ready",
                            "OpenSBI": "Ready",
                            "Kernel": "Ready",
                            "Computer": "Ready",
                        }
                    },
                    "computer_assembled_from(Riscv64Platform, OpenSBI, Kernel)",
                ),
                (
                    "Riscv64Platform.Enable",
                    {
                        "states": {
                            "Computer": "Online",
                            "Riscv64Platform": "Ready",
                        },
                    },
                    "riscv64_platform_system_spec_established()",
                ),
                (
                    "OpenSBI.Enable",
                    {
                        "states": {
                            "Computer": "Online",
                            "Riscv64Platform": "Online",
                            "OpenSBI": "Ready",
                            "Kernel": "Ready",
                            "Config": "Online",
                            "Lds": "Online",
                        },
                    },
                    "linux_riscv64_kernel_boot_spec_adopted()",
                ),
            ]
            for index, (signal, scenario, missing) in enumerate(cases):
                with self.subTest(signal=signal):
                    scenario_path = root / f"scenario-{index}.json"
                    scenario_path.write_text(json.dumps(scenario), encoding="utf-8")
                    data, checked, _, _ = self.run_prepared(
                        f"missing-enable-prerequisite-{index}",
                        signal=signal,
                        scenario=scenario_path,
                    )
                    self.assertEqual(checked["exit_code"], 1)
                    self.assertEqual(data["verdict"], "failed")
                    self.assertEqual(data["signals"][0]["outcome"], "rejected")
                    self.assertEqual(
                        data["signals"][0]["reason"],
                        f"condition_not_satisfied: {missing}",
                    )

            def golden_without_fact(fact: str) -> dict:
                scenario = deepcopy(read_json(KERNEL_ENABLE_SCENARIO))
                scenario["snapshot"]["facts"].remove(fact)
                scenario["provenance"]["boundary"]["snapshot"]["facts"].remove(fact)
                return scenario

            missing_abi_fact = "assert:BootCpuRegisters.satp == 0"
            missing_abi_scenario = root / "kernel-missing-satp.snapshot.json"
            missing_abi_scenario.write_text(
                json.dumps(golden_without_fact(missing_abi_fact)), encoding="utf-8"
            )
            missing_abi_data, missing_abi_checked, _, _ = self.run_prepared(
                "kernel-missing-satp",
                signal="Kernel.Enable",
                scenario=missing_abi_scenario,
            )
            self.assertEqual(missing_abi_checked["exit_code"], 1)
            self.assertEqual(missing_abi_data["signals"][0]["outcome"], "rejected")
            self.assertEqual(
                missing_abi_data["signals"][0]["reason"],
                "condition_not_satisfied: BootCpuRegisters.satp == 0",
            )
            self.assertFalse(
                any(item["target"] == "BootInitFlow" for item in missing_abi_data["signals"])
            )

            accept_fact = "kernel_enable_accept_available(Kernel)"
            missing_accept_scenario = root / "kernel-missing-accept.snapshot.json"
            missing_accept_scenario.write_text(
                json.dumps(golden_without_fact(accept_fact)), encoding="utf-8"
            )
            missing_accept_data, missing_accept_checked, _, _ = self.run_prepared(
                "kernel-missing-accept",
                signal="Kernel.Enable",
                scenario=missing_accept_scenario,
            )
            self.assertEqual(missing_accept_checked["exit_code"], 1)
            self.assertEqual(
                [
                    (item["target"], item["name"], item["outcome"])
                    for item in missing_accept_data["signals"]
                ],
                [
                    ("Kernel", "Enable", "failed"),
                    ("Kernel", "AcceptEnable", "rejected"),
                ],
            )
            self.assertEqual(
                missing_accept_data["failure"]["chain"],
                ["sig-0001", "sig-0002"],
            )
            self.assertEqual(
                missing_accept_data["last_stable_snapshot"]["states"]["Kernel"],
                "Ready",
            )
            self.assertNotIn(
                "kernel_enable_accepted(Kernel)",
                missing_accept_data["last_stable_snapshot"]["facts"],
            )
            self.assertFalse(
                any(item["target"] == "BootInitFlow" for item in missing_accept_data["signals"])
            )

            (root / "empty-scenario.json").write_text("{}\n", encoding="utf-8")
            alias_data, alias_checked, _, _ = self.run_prepared(
                "startup-alias",
                signal="Computer.Startup",
                scenario=root / "empty-scenario.json",
            )
            self.assertEqual(alias_checked["exit_code"], 0)
            self.assertEqual(alias_data["root_request"]["signal"], "Preset")
            self.assertEqual(alias_data["signals"][0]["outcome"], "completed")



if __name__ == "__main__":
    unittest.main()
