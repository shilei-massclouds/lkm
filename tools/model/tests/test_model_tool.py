from __future__ import annotations

import contextlib
import io
import tempfile
import unittest
from pathlib import Path

from common import MODEL_SCHEMA, MODEL_VERSION, read_json
from model_tool.__main__ import main as model_main
from parse_tool.__main__ import main as parse_main
from parse_tool.parser import _read_with_includes


class ModelToolTests(unittest.TestCase):
    def setUp(self) -> None:
        self.spec = (
            Path(__file__).resolve().parents[3]
            / "spec"
            / "model"
            / "main.spec"
        )

    def test_model_writes_model_json(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            ast = Path(tmp) / "model-main.ast.json"
            model = Path(tmp) / "model-main.model.json"

            self.assertEqual(parse_main([str(self.spec), "-o", str(ast)]), 0)
            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 0)
            data = read_json(model)
            self.assertEqual(data["schema"], MODEL_SCHEMA)
            self.assertEqual(data["version"], MODEL_VERSION)
            self.assertTrue(data["summary"]["ok"])
            self.assertGreaterEqual(data["summary"]["objects"], 25)
            self.assertEqual(data["summary"]["errors"], 0)
            self.assertIn("ComputerProject", data["model"]["objects"])
            self.assertIn("OpenSBI", data["model"]["objects"])
            self.assertNotIn("OpenSbi" + "Firmware", data["model"]["objects"])

    def test_model_json_contains_stable_structured_boundary_inventory(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            ast = Path(tmp) / "model-main.ast.json"
            model = Path(tmp) / "model-main.model.json"

            self.assertEqual(parse_main([str(self.spec), "-o", str(ast)]), 0)
            self.assertEqual(model_main([str(ast), "-o", str(model)]), 0)
            data = read_json(model)
            boundaries = data["model"]["boundaries"]
            expected_user_clone = {f"user_clone.{index:03d}" for index in range(1, 17)}

            self.assertTrue(expected_user_clone.issubset(boundaries))
            self.assertEqual(data["summary"]["legacy_boundaries"], 0)
            self.assertEqual(
                data["summary"]["deferred"],
                sum(item["status"] == "deferred" for item in boundaries.values()),
            )
            self.assertEqual(
                data["summary"]["trimmed"],
                sum(item["status"] == "trimmed" for item in boundaries.values()),
            )
            clone = boundaries["user_clone.001"]
            self.assertEqual(clone["category"], "Feature")
            self.assertEqual(
                clone["owner"],
                "UserCloneDeferredBoundaries.Transition::Setup",
            )
            self.assertTrue(clone["span"]["source_file"].endswith("objects/user_boot.spec"))
            self.assertGreater(clone["span"]["source_line"], 0)

    def test_structured_boundary_validation_rejects_invalid_inventory(self) -> None:
        valid_boundary = """
            deferred demo.001 {
                category: DeferredCategory::Feature;
                summary: "Complete the demo feature.";
                evidence { demo_deferred(A); }
                close_when: "The demo feature and tests are complete.";
            }
        """
        cases = {
            "invalid-id": (
                valid_boundary.replace("demo.001", "Demo.001"),
                "invalid boundary ID: Demo.001",
            ),
            "invalid-category": (
                valid_boundary.replace(
                    "DeferredCategory::Feature", "TrimmedCategory::BuildConfig"
                ),
                "invalid deferred category on demo.001",
            ),
            "missing-summary": (
                valid_boundary.replace('summary: "Complete the demo feature.";', ""),
                "boundary demo.001 is missing summary",
            ),
            "duplicate-id": (
                valid_boundary + valid_boundary,
                "duplicate boundary ID: demo.001",
            ),
            "legacy": (
                'deferred { "legacy free text"; }',
                "legacy deferred block is forbidden",
            ),
        }

        for name, (boundary_source, expected_error) in cases.items():
            with self.subTest(name=name), tempfile.TemporaryDirectory() as tmp:
                source = f"""
                    object A: T {{
                        initial_state: State::Base;

                        state State::Base {{
                            transitions {{
                                on Transition::Setup -> State::Ready {{
                                    {boundary_source}
                                }}
                            }}
                        }}

                        state State::Ready {{
                        }}
                    }}
                """
                spec = Path(tmp) / f"{name}.spec"
                ast = Path(tmp) / f"{name}.ast.json"
                model = Path(tmp) / f"{name}.model.json"
                spec.write_text(source, encoding="utf-8")

                self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
                stderr = io.StringIO()
                with contextlib.redirect_stderr(stderr):
                    exit_code = model_main([str(ast), "-o", str(model)])

                self.assertEqual(exit_code, 1)
                self.assertIn(expected_error, stderr.getvalue())

    def test_legacy_boundary_nested_in_raw_type_process_is_forbidden(self) -> None:
        source = """
            type T {
                processes {
                    Action::Run {
                        deferred {
                            "legacy nested action text";
                        }
                    }
                }
            }

            object A: T {
                initial_state: State::Base;
                state State::Base {
                }
            }
        """
        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "nested-legacy.spec"
            ast = Path(tmp) / "nested-legacy.ast.json"
            model = Path(tmp) / "nested-legacy.model.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 1)
            self.assertIn(
                "legacy deferred/trimmed block is forbidden inside an unparsed processes block",
                stderr.getvalue(),
            )

    def test_model_json_contains_indexed_children_and_events(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            ast = Path(tmp) / "model-main.ast.json"
            model = Path(tmp) / "model-main.model.json"

            self.assertEqual(parse_main([str(self.spec), "-o", str(ast)]), 0)
            self.assertEqual(model_main([str(ast), "-o", str(model)]), 0)
            data = read_json(model)
            objects = data["model"]["objects"]
            computer = objects["ComputerProject"]
            kernel = objects["Kernel"]
            opensbi = objects["OpenSBI"]
            preset = computer["states"]["Base"]["transitions"]["Preset"]
            enable = computer["states"]["Ready"]["transitions"]["Enable"]
            event_stream = objects["EventStream"]
            event_preset = event_stream["states"]["Base"]["transitions"]["Preset"]
            completion_type = data["model"]["types"]["Completion"]

            self.assertEqual(
                computer["children"],
                [
                    "KernelProject",
                ],
            )
            self.assertEqual(
                objects["KernelProject"]["children"],
                [
                    "Kernel",
                    "OpenSBI",
                ],
            )
            self.assertEqual(opensbi["initial_state"], "Ready")
            self.assertEqual(
                opensbi["states"]["Ready"]["transitions"]["Enable"]["target_state"],
                "Online",
            )
            self.assertEqual(
                kernel["children"],
                [
                    "EntryPreludePhase",
                    "BootPhase",
                    "InterruptPhase",
                    "UpMultitaskPhase",
                    "SmpRuntimePhase",
                    "PayloadPhase",
                ],
            )
            self.assertEqual(objects["EntryPreludePhase"]["parent"], "Kernel")
            self.assertEqual(
                objects["BootPhase"]["children"],
                [
                    "EntrySuccessorPhase",
                    "CorePreparePhase",
                    "MmCoreInitPhase",
                    "SchedInitPhase",
                ],
            )
            kernel_preset = kernel["states"]["Base"]["transitions"]["Preset"]
            kernel_setup = kernel["states"]["Prepared"]["transitions"]["Setup"]
            boot_preset = objects["BootPhase"]["states"]["Base"]["transitions"][
                "Preset"
            ]
            self.assertEqual(
                kernel_preset["body_members"][1]["within"]["context"],
                "SingleTaskContext",
            )
            self.assertEqual(
                [
                    entry["text"]
                    for entry in kernel_preset["body_members"][1]["within"][
                        "drives"
                    ][0]["entries"]
                ],
                ["EntryPreludePhase.Transition::Preset"],
            )
            self.assertEqual(
                [entry["text"] for entry in kernel_setup["drives"][0]["entries"]],
                [
                    "BootPhase.Transition::Preset",
                    "InterruptPhase.Transition::Preset",
                ],
            )
            self.assertEqual(
                [entry["text"] for entry in kernel_setup["ensures"][0]["entries"]],
                [
                    "BootPhase.state == State::Online",
                    "InterruptPhase.state == State::Online",
                ],
            )
            self.assertEqual(boot_preset["drives"], [])
            self.assertEqual(
                [entry["text"] for entry in boot_preset["depends_on"][0]["entries"]],
                ["EntryPreludePhase.state == State::Online"],
            )
            self.assertEqual(preset["source_state"], "Base")
            self.assertEqual(preset["target_state"], "Prepared")
            self.assertEqual(
                [entry["text"] for entry in preset["emits"][0]["entries"]],
                ["Transition::Setup"],
            )
            self.assertIn(
                "Riscv64.stvec == phys_addr(EventStream.early_event_entry)",
                [
                    entry["text"]
                    for block in event_preset["ensures"]
                    for entry in block["entries"]
                ],
            )
            self.assertEqual(
                objects["PhysicalMemory"]["properties"]["access"],
                "Access::ReadOnly",
            )
            self.assertEqual(
                completion_type["properties"],
                {
                    "ext_state": "CompletionExtState",
                    "done": "CompletionTokenCount",
                },
            )
            self.assertEqual(
                [block["kind"] for block in completion_type["blocks"]],
                ["owned", "lifecycle", "processes"],
            )
            self.assertEqual(
                [entry["text"] for entry in enable["drives"][0]["entries"]],
                [
                    "KernelProject.Transition::Preset",
                ],
            )

    def test_model_json_preserves_entry_spans(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            ast = Path(tmp) / "model-main.ast.json"
            model = Path(tmp) / "model-main.model.json"

            self.assertEqual(parse_main([str(self.spec), "-o", str(ast)]), 0)
            self.assertEqual(model_main([str(ast), "-o", str(model)]), 0)
            data = read_json(model)
            kernel_image = data["model"]["objects"]["KernelImage"]
            enable = kernel_image["states"]["Ready"]["transitions"]["Enable"]
            entry = enable["depends_on"][0]["entries"][0]

            self.assertEqual(entry["text"], "EarlyVm.state == State::Online")
            expanded = _read_with_includes(self.spec, seen=set(), stack=[])[0].splitlines()
            line = expanded[entry["span"]["start_line"] - 1]
            self.assertIn("EarlyVm.state == State::Online", line)

    def test_invalid_ast_schema_returns_usage_error_code(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            ast = Path(tmp) / "bad.ast.json"
            model = Path(tmp) / "bad.model.json"
            ast.write_text('{"schema": "wrong", "version": 1}\n', encoding="utf-8")

            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 2)
            self.assertIn("error: invalid AST JSON", stderr.getvalue())

    def test_duplicate_object_transition_names_fail_model_stage(self) -> None:
        source = """
            object A: T {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Enable -> State::Ready {
                        }
                    }
                }

                state State::Ready {
                    transitions {
                        on Transition::Enable -> State::Online {
                        }
                    }
                }

                state State::Online {
                }
            }
        """

        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "duplicate-transition.spec"
            ast = Path(tmp) / "duplicate-transition.ast.json"
            model = Path(tmp) / "duplicate-transition.model.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 1)
            self.assertIn(
                "duplicate object transition declaration: A.Transition::Enable",
                stderr.getvalue(),
            )

    def test_emits_allows_cross_object_transition(self) -> None:
        source = """
            object A: T {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Preset -> State::Prepared {
                            emits {
                                B.Transition::Setup;
                            }
                        }
                    }
                }

                state State::Prepared {
                }
            }

            object B: T {
                initial_state: State::Base;
                state State::Base { transitions { on Transition::Setup -> State::Ready {} } }
                state State::Ready {}
            }
        """

        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "emits-cross-object.spec"
            ast = Path(tmp) / "emits-cross-object.ast.json"
            model = Path(tmp) / "emits-cross-object.model.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            self.assertEqual(model_main([str(ast), "-o", str(model)]), 0)
            data = read_json(model)
            emits = data["model"]["objects"]["A"]["states"]["Base"]["transitions"]["Preset"]["emits"]
            self.assertEqual(emits[0]["entries"][0]["text"], "B.Transition::Setup")

    def test_emits_must_target_transition_enabled_from_target_state(self) -> None:
        source = """
            object A: T {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Preset -> State::Prepared {
                            emits {
                                Transition::Enable;
                            }
                        }
                    }
                }

                state State::Prepared {
                }

                state State::Ready {
                    transitions {
                        on Transition::Enable -> State::Online {
                        }
                    }
                }

                state State::Online {
                }
            }
        """

        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "bad-emits-source-state.spec"
            ast = Path(tmp) / "bad-emits-source-state.ast.json"
            model = Path(tmp) / "bad-emits-source-state.model.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 1)
            self.assertIn(
                "emitted transition is not enabled from target state",
                stderr.getvalue(),
            )

    def test_lifecycle_names_outside_controlled_sets_fail_model_stage(self) -> None:
        source = """
            object A: T {
                initial_state: State::Reserved;

                state State::Reserved {
                    transitions {
                        on Transition::Activate -> State::Done {
                        }
                    }
                }

                state State::Done {
                }
            }
        """

        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "bad-lifecycle-name.spec"
            ast = Path(tmp) / "bad-lifecycle-name.ast.json"
            model = Path(tmp) / "bad-lifecycle-name.model.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 1)
            errors = stderr.getvalue()
            self.assertIn("A.initial_state State::Reserved", errors)
            self.assertIn("A.State::Reserved", errors)
            self.assertIn("A.Transition::Activate", errors)
            self.assertIn("A.Transition::Activate -> State::Done", errors)

    def test_disable_and_offline_lifecycle_names_and_transitions_are_allowed(self) -> None:
        source = """
            object A: T {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Setup -> State::Ready {
                        }
                    }
                }

                state State::Ready {
                    transitions {
                        on Transition::Enable -> State::Online {
                        }
                    }
                }

                state State::Online {
                    transitions {
                        on Transition::Disable -> State::Offline {
                        }
                    }
                }

                state State::Offline {
                    transitions {
                        on Transition::Cleanup -> State::Destroyed {
                        }
                    }
                }

                state State::Destroyed {
                }
            }
        """

        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "offline-lifecycle.spec"
            ast = Path(tmp) / "offline-lifecycle.ast.json"
            model = Path(tmp) / "offline-lifecycle.model.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 0, stderr.getvalue())

    def test_lifecycle_transitions_outside_controlled_table_fail_model_stage(self) -> None:
        source = """
            object A: T {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Enable -> State::Online {
                        }
                    }
                }

                state State::Online {
                }
            }
        """

        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "bad-lifecycle-transition.spec"
            ast = Path(tmp) / "bad-lifecycle-transition.ast.json"
            model = Path(tmp) / "bad-lifecycle-transition.model.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 1)
            self.assertIn(
                "invalid lifecycle transition: A.State::Base.Transition::Enable -> State::Online",
                stderr.getvalue(),
            )

    def test_disable_from_base_to_offline_fails_model_stage(self) -> None:
        source = """
            object A: T {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Disable -> State::Offline {
                        }
                    }
                }

                state State::Offline {
                }
            }
        """

        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "bad-disable-transition.spec"
            ast = Path(tmp) / "bad-disable-transition.ast.json"
            model = Path(tmp) / "bad-disable-transition.model.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 1)
            self.assertIn(
                "invalid lifecycle transition: A.State::Base.Transition::Disable -> State::Offline",
                stderr.getvalue(),
            )

    def test_single_argument_type_process_accepts_positional_drive(self) -> None:
        source = """
            type T {
                processes {
                    Transition::Touch(value: TouchValue) {
                    }
                }
            }

            object A: T {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Setup -> State::Ready {
                            drives {
                                A.Transition::Touch(TouchValue::Ready);
                            }
                        }
                    }
                }

                state State::Ready {
                }
            }
        """

        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "single-arg-position.spec"
            ast = Path(tmp) / "single-arg-position.ast.json"
            model = Path(tmp) / "single-arg-position.model.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 0, stderr.getvalue())

    def test_multi_argument_type_process_allows_positional_drive_args(self) -> None:
        source = """
            type T {
                processes {
                    Action::Copy(src: TaskRef, dst: TaskRef) {
                    }
                }
            }

            object A: T {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Setup -> State::Ready {
                            drives {
                                A.Action::Copy(SourceTaskRef, TargetTaskRef);
                            }
                        }
                    }
                }

                state State::Ready {
                }
            }
        """

        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "multi-arg-position.spec"
            ast = Path(tmp) / "multi-arg-position.ast.json"
            model = Path(tmp) / "multi-arg-position.model.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 0, stderr.getvalue())

    def test_multi_argument_type_process_rejects_wrong_positional_count(self) -> None:
        source = """
            type T {
                processes {
                    Action::Copy(src: TaskRef, dst: TaskRef) {
                    }
                }
            }

            object A: T {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Setup -> State::Ready {
                            drives {
                                A.Action::Copy(SourceTaskRef);
                            }
                        }
                    }
                }

                state State::Ready {
                }
            }
        """

        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "multi-arg-position-bad.spec"
            ast = Path(tmp) / "multi-arg-position-bad.ast.json"
            model = Path(tmp) / "multi-arg-position-bad.model.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 1)
            self.assertIn(
                "positional process argument count mismatch",
                stderr.getvalue(),
            )

    def test_context_nesting_allows_monotonic_effects(self) -> None:
        source = """
            type RawSpinLock {
                processes {
                    Transition::LockIrqSave {
                    }

                    Transition::UnlockIrqRestore {
                    }
                }
            }

            lock OuterLock: RawSpinLock;
            lock InnerLock: RawSpinLock;

            context OuterContext: ResourceExclusiveContext {
                guard {
                    lock_ref: OuterLock;

                    entered_by {
                        OuterLock.Transition::LockIrqSave;
                    }

                    exited_by {
                        OuterLock.Transition::UnlockIrqRestore;
                    }
                }

                obj_refs {
                    A;
                }
            }

            context InnerContext: ResourceExclusiveContext {
                guard {
                    lock_ref: InnerLock;

                    entered_by {
                        InnerLock.Transition::LockIrqSave;
                    }

                    exited_by {
                        InnerLock.Transition::UnlockIrqRestore;
                    }
                }

                obj_refs {
                    A;
                }
            }

            object A: T {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Setup -> State::Ready {
                            within OuterContext {
                                within InnerContext {
                                }
                            }
                        }
                    }
                }

                state State::Ready {
                }
            }
        """

        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "context-nesting-ok.spec"
            ast = Path(tmp) / "context-nesting-ok.ast.json"
            model = Path(tmp) / "context-nesting-ok.model.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 0, stderr.getvalue())

    def test_only_once_within_passes_when_reachable_once(self) -> None:
        source = """
            context GuardedContext: Context {
            }

            object ComputerProject: ProjectObject {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Preset -> State::Ready {
                            drives {
                                A.Transition::Setup;
                            }
                        }
                    }
                }

                state State::Ready {
                }
            }

            object A: T {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Setup -> State::Ready {
                            within GuardedContext only-once {
                            }
                        }
                    }
                }

                state State::Ready {
                }
            }
        """

        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "only-once-ok.spec"
            ast = Path(tmp) / "only-once-ok.ast.json"
            model = Path(tmp) / "only-once-ok.model.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 0, stderr.getvalue())
            data = read_json(model)
            setup = data["model"]["objects"]["A"]["states"]["Base"]["transitions"]["Setup"]
            self.assertTrue(setup["within"][0]["only_once"])

    def test_model_json_preserves_ordered_event_body_members(self) -> None:
        source = """
            context GuardedContext: Context {
            }

            object ComputerProject: ProjectObject {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Setup -> State::Ready {
                            drives {
                                A.Transition::Setup;
                            }
                        }
                    }
                }

                state State::Ready {
                }
            }

            object A: T {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Setup -> State::Ready {
                            drives {
                                B.Transition::Setup;
                            }

                            within GuardedContext {
                                drives {
                                    C.Transition::Setup;
                                }
                            }

                            drives {
                                D.Transition::Setup;
                            }
                        }
                    }
                }

                state State::Ready {
                }
            }

            object B: T {
                initial_state: State::Base;
                state State::Base { transitions { on Transition::Setup -> State::Ready {} } }
                state State::Ready {}
            }

            object C: T {
                initial_state: State::Base;
                state State::Base { transitions { on Transition::Setup -> State::Ready {} } }
                state State::Ready {}
            }

            object D: T {
                initial_state: State::Base;
                state State::Base { transitions { on Transition::Setup -> State::Ready {} } }
                state State::Ready {}
            }
        """

        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "ordered-body.spec"
            ast = Path(tmp) / "ordered-body.ast.json"
            model = Path(tmp) / "ordered-body.model.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 0, stderr.getvalue())
            data = read_json(model)
            setup = data["model"]["objects"]["A"]["states"]["Base"]["transitions"]["Setup"]
            self.assertEqual(
                [member["kind"] for member in setup["body_members"]],
                ["drives", "within", "drives"],
            )
            self.assertEqual(
                setup["body_members"][1]["within"]["drives"][0]["entries"][0]["text"],
                "C.Transition::Setup",
            )

    def test_only_once_within_fails_when_event_reachable_twice(self) -> None:
        source = """
            context GuardedContext: Context {
            }

            object ComputerProject: ProjectObject {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Preset -> State::Ready {
                            drives {
                                A.Transition::Setup;
                                A.Transition::Setup;
                            }
                        }
                    }
                }

                state State::Ready {
                }
            }

            object A: T {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Setup -> State::Ready {
                            within GuardedContext only-once {
                            }
                        }
                    }
                }

                state State::Ready {
                }
            }
        """

        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "only-once-bad.spec"
            ast = Path(tmp) / "only-once-bad.ast.json"
            model = Path(tmp) / "only-once-bad.model.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 1)
            self.assertIn(
                "within only-once proof failed: GuardedContext is reachable 2 times",
                stderr.getvalue(),
            )

    def test_legacy_context_effects_remain_compatible(self) -> None:
        source = """
            type RawSpinLock {
                processes {
                    Transition::LockIrqSave {
                    }

                    Transition::UnlockIrqRestore {
                    }
                }
            }

            lock ALock: RawSpinLock;

            context AContext: ResourceExclusiveContext {
                guard {
                    lock_ref: ALock;

                    entered_by {
                        ALock.Transition::LockIrqSave;
                    }

                    exited_by {
                        ALock.Transition::UnlockIrqRestore;
                    }
                }

                obj_refs {
                    A;
                }

                effects {
                    interruptible: false;
                    preemptible: false;
                    sleepable: false;
                    exclusive_refs: obj_refs;
                }
            }

            object A: T {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Setup -> State::Ready {
                            within AContext {
                            }
                        }
                    }
                }

                state State::Ready {
                }
            }
        """

        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "legacy-effects-ok.spec"
            ast = Path(tmp) / "legacy-effects-ok.ast.json"
            model = Path(tmp) / "legacy-effects-ok.model.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 0, stderr.getvalue())

    def test_resource_context_lock_ref_can_target_object(self) -> None:
        source = """
            type Mutex {
                processes {
                    Transition::Lock(current_task: TaskRef) {
                    }

                    Transition::Unlock(current_task: TaskRef) {
                    }
                }
            }

            type TaskRef {
            }

            object JumpLabelMutex: Mutex {
                initial_state: State::Base;

                state State::Base {
                }
            }

            context StaticBranchJumpLabelContext: ResourceExclusiveContext {
                guard {
                    lock_ref: JumpLabelMutex;

                    entered_by {
                        JumpLabelMutex.Transition::Lock(BootInitTaskRef);
                    }

                    exited_by {
                        JumpLabelMutex.Transition::Unlock(BootInitTaskRef);
                    }
                }

                obj_refs {
                    StaticBranch;
                }
            }

            object StaticBranch: T {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Setup -> State::Ready {
                            within StaticBranchJumpLabelContext {
                            }
                        }
                    }
                }

                state State::Ready {
                }
            }
        """

        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "object-lock-ref-context.spec"
            ast = Path(tmp) / "object-lock-ref-context.ast.json"
            model = Path(tmp) / "object-lock-ref-context.model.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 0, stderr.getvalue())

    def test_context_nesting_rejects_weaker_inner_holds(self) -> None:
        source = """
            type RawSpinLock {
                processes {
                    Transition::LockIrqSave {
                    }

                    Transition::UnlockIrqRestore {
                    }
                }
            }

            lock OuterLock: RawSpinLock;

            context OuterContext: ResourceExclusiveContext {
                guard {
                    lock_ref: OuterLock;

                    entered_by {
                        OuterLock.Transition::LockIrqSave;
                    }

                    exited_by {
                        OuterLock.Transition::UnlockIrqRestore;
                    }
                }

                obj_refs {
                    A;
                }
            }

            context InnerPreemptContext: Context {
                guard {
                    holds {
                        local_interrupts: enabled;
                    }
                }
            }

            object A: T {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Setup -> State::Ready {
                            within OuterContext {
                                within InnerPreemptContext {
                                }
                            }
                        }
                    }
                }

                state State::Ready {
                }
            }

        """

        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "context-nesting-bad.spec"
            ast = Path(tmp) / "context-nesting-bad.ast.json"
            model = Path(tmp) / "context-nesting-bad.model.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 1)
            self.assertIn(
                "invalid context nesting: inner context "
                "InnerPreemptContext weakens local_interrupts from constrained to open",
                stderr.getvalue(),
            )

    def test_local_interrupt_guard_context_is_valid(self) -> None:
        source = """
            type LocalInterruptControl {
                processes {
                    Transition::SaveAndDisable {
                    }

                    Transition::Restore {
                    }
                }
            }

            context LocalIrqContext: Context {
                guard {
                    entered_by {
                        BootCpuLocalInterrupt.Transition::SaveAndDisable;
                    }

                    exited_by {
                        BootCpuLocalInterrupt.Transition::Restore;
                    }
                }

                obj_refs {
                    BootCpuLocalInterrupt;
                    A;
                }
            }

            object BootCpuLocalInterrupt: LocalInterruptControl {
                initial_state: State::Base;

                state State::Base {
                }
            }

            object A: T {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Setup -> State::Ready {
                            within LocalIrqContext {
                            }
                        }
                    }
                }

                state State::Ready {
                }
            }
        """

        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "local-irq-context.spec"
            ast = Path(tmp) / "local-irq-context.ast.json"
            model = Path(tmp) / "local-irq-context.model.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 0, stderr.getvalue())

    def test_context_guard_rejects_unpaired_enter_boundary(self) -> None:
        source = """
            type LocalInterruptControl {
                processes {
                    Transition::Enable {
                    }
                }
            }

            context BadContext: Context {
                guard {
                    entered_by {
                        BootCpuLocalInterrupt.Transition::Enable;
                    }
                }

                obj_refs {
                    BootCpuLocalInterrupt;
                }
            }

            object BootCpuLocalInterrupt: LocalInterruptControl {
                initial_state: State::Base;

                state State::Base {
                }
            }
        """

        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "bad-context-enter-only.spec"
            ast = Path(tmp) / "bad-context-enter-only.ast.json"
            model = Path(tmp) / "bad-context-enter-only.model.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 1)
            self.assertIn(
                "context guard on BadContext must pair entered_by with exited_by",
                stderr.getvalue(),
            )

    def test_context_guard_rejects_boundary_and_holds_mix(self) -> None:
        source = """
            type PreemptionControl {
                processes {
                    Transition::Disable {
                    }

                    Transition::Enable {
                    }
                }
            }

            context BadContext: Context {
                guard {
                    entered_by {
                        BootPreemption.Transition::Disable;
                    }

                    exited_by {
                        BootPreemption.Transition::Enable;
                    }

                    holds {
                        preemption: disabled;
                    }
                }

                obj_refs {
                    BootPreemption;
                }
            }

            object BootPreemption: PreemptionControl {
                initial_state: State::Base;

                state State::Base {
                }
            }
        """

        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "bad-context-boundary-holds.spec"
            ast = Path(tmp) / "bad-context-boundary-holds.ast.json"
            model = Path(tmp) / "bad-context-boundary-holds.model.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 1)
            self.assertIn(
                "context guard on BadContext must not mix entered_by/exited_by with holds",
                stderr.getvalue(),
            )

    def test_phase_boundary_guard_without_effects_is_valid(self) -> None:
        source = """
            context SingleTaskContext: Context {
                guard {
                    holds {
                        cpu_concurrency: single_cpu;
                        task_concurrency: single_task;
                        local_interrupts: disabled;
                        preemption: disabled;
                        voluntary_switching: disabled;
                    }
                }
            }

            object A: T {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Setup -> State::Ready {
                            within SingleTaskContext {
                            }
                        }
                    }
                }

                state State::Ready {
                }
            }
        """

        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "phase-boundary-context.spec"
            ast = Path(tmp) / "phase-boundary-context.ast.json"
            model = Path(tmp) / "phase-boundary-context.model.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 0, stderr.getvalue())

    def test_plain_context_without_obj_refs_allows_drives(self) -> None:
        source = """
            context SingleTaskContext: Context {
                guard {
                    holds {
                        cpu_concurrency: single_cpu;
                        task_concurrency: single_task;
                        local_interrupts: disabled;
                        preemption: disabled;
                    }
                }
            }

            object Child: T {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Setup -> State::Ready {
                        }
                    }
                }

                state State::Ready {
                }
            }

            object Parent: T {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Setup -> State::Ready {
                            within SingleTaskContext {
                                drives {
                                    Child.Transition::Setup;
                                }
                            }
                        }
                    }
                }

                state State::Ready {
                }
            }
        """

        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "plain-context-drives.spec"
            ast = Path(tmp) / "plain-context-drives.ast.json"
            model = Path(tmp) / "plain-context-drives.model.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 0, stderr.getvalue())

    def test_guard_holds_rejects_weaker_nested_context(self) -> None:
        source = """
            context OuterContext: Context {
                guard {
                    holds {
                        voluntary_switching: disabled;
                    }
                }
            }

            context InnerContext: Context {
                guard {
                    holds {
                        voluntary_switching: enabled;
                    }
                }
            }

            object A: T {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Setup -> State::Ready {
                            within OuterContext {
                                within InnerContext {
                                }
                            }
                        }
                    }
                }

                state State::Ready {
                }
            }
        """

        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "phase-boundary-context-bad.spec"
            ast = Path(tmp) / "phase-boundary-context-bad.ast.json"
            model = Path(tmp) / "phase-boundary-context-bad.model.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                exit_code = model_main([str(ast), "-o", str(model)])

            self.assertEqual(exit_code, 1)
            self.assertIn(
                "invalid context nesting: inner context "
                "InnerContext weakens voluntary_switching from constrained to open",
                stderr.getvalue(),
            )

    def test_model_tool_does_not_import_pyveri(self) -> None:
        source_root = Path(__file__).resolve().parents[1] / "src" / "model_tool"

        for path in source_root.rglob("*.py"):
            text = path.read_text(encoding="utf-8")
            self.assertNotIn("from pyveri", text, str(path))
            self.assertNotIn("import pyveri", text, str(path))


if __name__ == "__main__":
    unittest.main()
