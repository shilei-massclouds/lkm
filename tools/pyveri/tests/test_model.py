from __future__ import annotations

import unittest
from pathlib import Path

from pyveri.model import Severity, build_model
from pyveri.parser import parse_file, parse_text
from pyveri.view import (
    build_drives_view,
    build_object_view,
    build_timeline_view,
    render_dot,
    render_svg,
    render_text,
)


class ModelBuilderTests(unittest.TestCase):
    def test_build_current_model(self) -> None:
        spec = Path(__file__).resolve().parents[3] / "spec" / "model" / "main.spec"

        result = build_model(parse_file(spec))

        self.assertTrue(result.ok, [diag.format() for diag in result.errors])
        self.assertEqual(len(result.errors), 0)
        self.assertIn("StartupTimeline", result.model.objects)
        self.assertEqual(
            result.model.children["StartupTimeline"],
            [
                "PreparePhase",
                "BootPhase",
                "InterruptPhase",
                "UpMultitaskPhase",
                "SmpRuntimePhase",
                "PayloadPhase",
            ],
        )
        self.assertEqual(
            result.model.objects["BootPhase"].children,
            [
                "EntryPreludePhase",
                "EntrySuccessorPhase",
                "CorePreparePhase",
                "MmCoreInitPhase",
                "SchedInitPhase",
            ],
        )

    def test_builds_object_view(self) -> None:
        spec = Path(__file__).resolve().parents[3] / "spec" / "model" / "main.spec"

        result = build_model(parse_file(spec))
        view = build_object_view(result.model)
        text = render_text(view)
        dot = render_dot(view)

        self.assertIn("StartupTimeline: TimelineObject", text)
        self.assertIn("StartupTimeline -> PreparePhase [parent]", text)
        self.assertNotIn("drives", text)
        self.assertIn('"StartupTimeline" -> "PreparePhase"', dot)
        self.assertNotIn("drives", dot)

    def test_builds_drives_view(self) -> None:
        spec = Path(__file__).resolve().parents[3] / "spec" / "model" / "main.spec"

        result = build_model(parse_file(spec))
        view = build_drives_view(result.model)
        text = render_text(view)
        dot = render_dot(view)

        self.assertIn("StartupTimeline.Setup", text)
        self.assertIn("  -> PreparePhase.Setup", text)
        self.assertIn("BootPhase.Setup", text)
        self.assertIn("EntryPreludePhase.Setup", text)
        self.assertIn("EntrySuccessorPhase.Setup", text)
        self.assertIn("CorePreparePhase.Setup", text)
        self.assertIn("MmCoreInitPhase.Setup", text)
        self.assertIn("PayloadPhase.Setup", text)
        self.assertIn("rankdir=LR", dot)
        self.assertIn('"StartupTimeline.Setup" -> "PreparePhase.Setup"', dot)

    def test_builds_timeline_view(self) -> None:
        spec = Path(__file__).resolve().parents[3] / "spec" / "model" / "main.spec"

        result = build_model(parse_file(spec))
        view = build_timeline_view(result.model)
        text = render_text(view)
        svg = render_svg(view)

        self.assertIn("timeline view:", text)
        self.assertIn("PreparePhase: ready (State::Ready)", text)
        self.assertIn("PreparePhase: online (State::Online)", text)
        self.assertIn("  - Riscv64.State::Online", text)
        self.assertIn("  - PhysicalMemory.State::Online", text)
        self.assertIn("EntryPreludePhase: ready (State::Ready)", text)
        self.assertIn("EntrySuccessorPhase: ready (State::Ready)", text)
        self.assertIn("CorePreparePhase: ready (State::Ready)", text)
        self.assertIn("MmCoreInitPhase: ready (State::Ready)", text)
        self.assertIn("BootPhase: ready (State::Ready)", text)
        self.assertIn("PayloadPhase: ready (State::Ready)", text)
        self.assertIn("PayloadPhase: online (State::Online)", text)
        self.assertIn("  - RootStream.State::Prepared", text)
        self.assertIn("  - Soc.State::Prepared", text)
        self.assertIn("  - Vm.State::Online", text)
        self.assertIn("  - SwapperVm.State::Online", text)
        self.assertIn("  - MemBlock.State::Offline", text)
        self.assertNotIn("StartupTimeline", text)
        self.assertIn("<svg", svg)
        self.assertIn("PreparePhase", svg)
        self.assertIn("BootPhase", svg)
        self.assertIn("EntryPreludePhase", svg)
        self.assertIn("EntrySuccessorPhase", svg)
        self.assertIn("CorePreparePhase", svg)
        self.assertIn("MmCoreInitPhase", svg)
        self.assertIn("PayloadPhase", svg)
        self.assertNotIn("StartupTimeline", svg)


    def test_accepts_exclusive_context_within_action_refs(self) -> None:
        document = parse_text(
            """
            type RawSpinLock {
                processes {
                    Event::LockIrqSave {
                    }

                    Event::UnlockIrqRestore {
                    }
                }
            }

            lock TaskPiLock: RawSpinLock;

            exclusive_context WakeContext {
                lock_ref: TaskPiLock;

                obj_refs {
                    A;
                    B;
                }
            }

            object A: T {
                initial_state: State::Ready;

                state State::Ready {
                    events {
                        on Event::Enable -> State::Online {
                            within WakeContext {
                                entered_by {
                                    TaskPiLock.Event::LockIrqSave;
                                }

                                drives {
                                    A.Action::SetTaskState(TaskRuntimeState::Running);
                                    B.Action::Touch(task: A);
                                }

                                exited_by {
                                    TaskPiLock.Event::UnlockIrqRestore;
                                }
                            }
                        }
                    }
                }

                state State::Online {
                }
            }

            object B: T {
                initial_state: State::Ready;

                state State::Ready {
                }
            }
            """
        )

        result = build_model(document)

        self.assertTrue(result.ok, [diag.message for diag in result.errors])
        self.assertIn("WakeContext", result.model.exclusive_contexts)
        self.assertEqual(result.model.locks["TaskPiLock"].kind, "RawSpinLock")

    def test_accepts_resource_context_guard_action_refs(self) -> None:
        document = parse_text(
            """
            type RawSpinLock {
                processes {
                    Event::LockIrqSave {
                    }

                    Event::UnlockIrqRestore {
                    }
                }
            }

            lock TaskPiLock: RawSpinLock;

            context WakeContext: ResourceExclusiveContext {
                guard: RawSpinLockIrqSaveGuard {
                    lock_ref: TaskPiLock;

                    entered_by {
                        TaskPiLock.Event::LockIrqSave;
                    }

                    exited_by {
                        TaskPiLock.Event::UnlockIrqRestore;
                    }
                }

                obj_refs {
                    A;
                    B;
                }

                effects {
                    interruptible: false;
                    preemptible: false;
                    sleepable: false;
                    exclusive_refs: obj_refs;
                }
            }

            object A: T {
                initial_state: State::Ready;

                state State::Ready {
                    events {
                        on Event::Enable -> State::Online {
                            within WakeContext {
                                drives {
                                    A.Action::SetTaskState(TaskRuntimeState::Running);
                                    B.Action::Touch(task: A);
                                }
                            }
                        }
                    }
                }

                state State::Online {
                }
            }

            object B: T {
                initial_state: State::Ready;

                state State::Ready {
                }
            }
            """
        )

        result = build_model(document)

        self.assertTrue(result.ok, [diag.message for diag in result.errors])
        context = result.model.exclusive_contexts["WakeContext"]
        self.assertEqual(context.kind, "ResourceExclusiveContext")
        self.assertIsNotNone(context.guard)
        self.assertEqual(context.lock_ref, "TaskPiLock")
        self.assertEqual(result.model.locks["TaskPiLock"].kind, "RawSpinLock")

    def test_rejects_within_boundary_override_for_context_guard(self) -> None:
        document = parse_text(
            """
            type RawSpinLock {
                processes {
                    Event::LockIrqSave {
                    }
                }
            }

            lock TaskPiLock: RawSpinLock;

            context WakeContext: ResourceExclusiveContext {
                guard: RawSpinLockIrqSaveGuard {
                    lock_ref: TaskPiLock;

                    entered_by {
                        TaskPiLock.Event::LockIrqSave;
                    }
                }

                obj_refs {
                    A;
                }
            }

            object A: T {
                initial_state: State::Ready;

                state State::Ready {
                    events {
                        on Event::Enable -> State::Online {
                            within WakeContext {
                                entered_by {
                                    TaskPiLock.Event::LockIrqSave;
                                }
                            }
                        }
                    }
                }

                state State::Online {
                }
            }
            """
        )

        result = build_model(document)

        self.assertFalse(result.ok)
        self.assertTrue(
            any(
                "must not override context guard entered_by" in diag.message
                and diag.severity is Severity.ERROR
                for diag in result.errors
            )
        )

    def test_rejects_within_boundary_for_non_context_lock(self) -> None:
        document = parse_text(
            """
            type RawSpinLock {
                processes {
                    Event::LockIrqSave {
                    }
                }
            }

            lock TaskPiLock: RawSpinLock;
            lock OtherLock: RawSpinLock;

            exclusive_context WakeContext {
                lock_ref: TaskPiLock;

                obj_refs {
                    A;
                }
            }

            object A: T {
                initial_state: State::Ready;

                state State::Ready {
                    events {
                        on Event::Enable -> State::Online {
                            within WakeContext {
                                entered_by {
                                    OtherLock.Event::LockIrqSave;
                                }
                            }
                        }
                    }
                }

                state State::Online {
                }
            }
            """
        )

        result = build_model(document)

        self.assertFalse(result.ok)
        self.assertTrue(
            any(
                "lock event reference outside exclusive_context lock_ref" in diag.message
                and diag.severity is Severity.ERROR
                for diag in result.errors
            )
        )

    def test_rejects_action_refs_outside_exclusive_context_objects(self) -> None:
        document = parse_text(
            """
            lock TaskPiLock;

            exclusive_context WakeContext {
                lock_ref: TaskPiLock;

                obj_refs {
                    A;
                }
            }

            object A: T {
                initial_state: State::Ready;

                state State::Ready {
                    events {
                        on Event::Enable -> State::Online {
                            within WakeContext {
                                drives {
                                    B.Action::Touch(task: A);
                                }
                            }
                        }
                    }
                }

                state State::Online {
                }
            }

            object B: T {
                initial_state: State::Ready;

                state State::Ready {
                }
            }
            """
        )

        result = build_model(document)

        self.assertFalse(result.ok)
        self.assertTrue(
            any(
                "action reference outside exclusive_context obj_refs" in diag.message
                and diag.severity is Severity.ERROR
                for diag in result.errors
            )
        )

    def test_reports_unknown_drive_event(self) -> None:
        document = parse_text(
            """
            object A: T {
                initial_state: State::Base;

                state State::Base {
                    events {
                        on Event::Setup -> State::Ready {
                            drives {
                                B.Event::Missing;
                            }
                        }
                    }
                }

                state State::Ready {
                }
            }

            object B: T {
                initial_state: State::Base;

                state State::Base {
                }
            }
            """
        )

        result = build_model(document)

        self.assertFalse(result.ok)
        self.assertTrue(
            any("unknown event reference: B.Event::Missing" in diag.message for diag in result.errors)
        )

    def test_reports_unknown_state_reference(self) -> None:
        document = parse_text(
            """
            object A: T {
                initial_state: State::Base;

                state State::Base {
                    invariant {
                        B.state == State::Ready;
                    }
                }
            }

            object B: T {
                initial_state: State::Base;

                state State::Base {
                }
            }
            """
        )

        result = build_model(document)

        self.assertFalse(result.ok)
        self.assertTrue(
            any(
                "unknown state reference: B.state == State::Ready" in diag.message
                and diag.severity is Severity.ERROR
                for diag in result.errors
            )
        )

    def test_rejects_duplicate_object_event_names_across_states(self) -> None:
        document = parse_text(
            """
            object A: T {
                initial_state: State::Base;

                state State::Base {
                    events {
                        on Event::Enable -> State::Ready {
                        }
                    }
                }

                state State::Ready {
                    events {
                        on Event::Enable -> State::Online {
                        }
                    }
                }

                state State::Online {
                }
            }
            """
        )

        result = build_model(document)

        self.assertFalse(result.ok)
        self.assertTrue(
            any(
                "duplicate object event declaration: A.Event::Enable" in diag.message
                and diag.severity is Severity.ERROR
                for diag in result.errors
            )
        )

    def test_rejects_lifecycle_names_outside_controlled_sets(self) -> None:
        document = parse_text(
            """
            object A: T {
                initial_state: State::Reserved;

                state State::Reserved {
                    events {
                        on Event::Activate -> State::Done {
                        }
                    }
                }

                state State::Done {
                }
            }
            """
        )

        result = build_model(document)
        messages = [diag.message for diag in result.errors]

        self.assertFalse(result.ok)
        self.assertTrue(
            any("A.initial_state State::Reserved" in message for message in messages)
        )
        self.assertTrue(
            any("A.State::Reserved" in message for message in messages)
        )
        self.assertTrue(
            any("A.Event::Activate" in message for message in messages)
        )
        self.assertTrue(
            any("A.Event::Activate -> State::Done" in message for message in messages)
        )

    def test_accepts_disable_and_offline_lifecycle_names_and_transitions(self) -> None:
        document = parse_text(
            """
            object A: T {
                initial_state: State::Base;

                state State::Base {
                    events {
                        on Event::Setup -> State::Ready {
                        }
                    }
                }

                state State::Ready {
                    events {
                        on Event::Enable -> State::Online {
                        }
                    }
                }

                state State::Online {
                    events {
                        on Event::Disable -> State::Offline {
                        }
                    }
                }

                state State::Offline {
                    events {
                        on Event::Cleanup -> State::Destroyed {
                        }
                    }
                }

                state State::Destroyed {
                }
            }
            """
        )

        result = build_model(document)

        self.assertTrue(result.ok, [diag.message for diag in result.errors])

    def test_rejects_lifecycle_transitions_outside_controlled_table(self) -> None:
        document = parse_text(
            """
            object A: T {
                initial_state: State::Base;

                state State::Base {
                    events {
                        on Event::Enable -> State::Online {
                        }
                    }
                }

                state State::Online {
                }
            }
            """
        )

        result = build_model(document)

        self.assertFalse(result.ok)
        self.assertTrue(
            any(
                "invalid lifecycle transition: A.State::Base.Event::Enable -> State::Online"
                in diag.message
                and diag.severity is Severity.ERROR
                for diag in result.errors
            )
        )

    def test_rejects_disable_from_base_to_offline(self) -> None:
        document = parse_text(
            """
            object A: T {
                initial_state: State::Base;

                state State::Base {
                    events {
                        on Event::Disable -> State::Offline {
                        }
                    }
                }

                state State::Offline {
                }
            }
            """
        )

        result = build_model(document)

        self.assertFalse(result.ok)
        self.assertTrue(
            any(
                "invalid lifecycle transition: A.State::Base.Event::Disable -> State::Offline"
                in diag.message
                and diag.severity is Severity.ERROR
                for diag in result.errors
            )
        )


if __name__ == "__main__":
    unittest.main()
