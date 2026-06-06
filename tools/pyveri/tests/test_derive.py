from __future__ import annotations

import unittest
from pathlib import Path

from pyveri.derive import DerivationStatus, derive, render_derivation_text
from pyveri.model import build_model
from pyveri.parser import parse_file, parse_text


class DerivationTests(unittest.TestCase):
    def test_derives_minimal_driven_path(self) -> None:
        result = build_model(
            parse_text(
                """
                object A: TimelineObject {
                    initial_state: State::Base;

                    state State::Base {
                        events {
                            on Event::Setup -> State::Ready {
                                drives {
                                    B.Event::Setup;
                                }
                            }
                        }
                    }

                    state State::Ready {
                        invariant {
                            B.state == State::Ready;
                        }
                    }
                }

                object B: PhaseObject {
                    initial_state: State::Base;
                    parent: A;

                    state State::Base {
                        events {
                            on Event::Setup -> State::Ready {
                                depends_on {
                                    C.state == State::Online;
                                    non_computable_predicate();
                                }
                            }
                        }
                    }

                    state State::Ready {
                    }
                }

                object C: InputObject {
                    initial_state: State::Online;

                    state State::Online {
                    }
                }
                """
            )
        )

        derivation = derive(result.model, "A.Event::Setup")

        self.assertTrue(derivation.ok)
        self.assertEqual(derivation.states["A"], "Ready")
        self.assertEqual(derivation.states["B"], "Ready")
        self.assertEqual(
            [transition.object_name for transition in derivation.transitions],
            ["B", "A"],
        )
        self.assertEqual(len(derivation.trace), 1)
        self.assertEqual(derivation.trace[0].object_name, "A")
        self.assertEqual(derivation.trace[0].children[0].object_name, "B")
        self.assertTrue(
            any(
                record.status is DerivationStatus.OBLIGATION
                and record.expression == "non_computable_predicate()"
                for record in derivation.records
            )
        )

    def test_blocks_when_dependency_state_is_missing(self) -> None:
        result = build_model(
            parse_text(
                """
                object A: TimelineObject {
                    initial_state: State::Base;

                    state State::Base {
                        events {
                            on Event::Setup -> State::Ready {
                                depends_on {
                                    B.state == State::Ready;
                                }
                            }
                        }
                    }

                    state State::Ready {
                    }
                }

                object B: PhaseObject {
                    initial_state: State::Base;

                    state State::Base {
                    }

                    state State::Ready {
                    }
                }
                """
            )
        )

        derivation = derive(result.model, "A.Event::Setup")

        self.assertFalse(derivation.ok)
        self.assertEqual(derivation.states["A"], "Base")
        self.assertTrue(
            any(
                record.status is DerivationStatus.BLOCKED
                and "B.state == State::Ready" in record.message
                and record.span is not None
                and record.span.start_line == 9
                for record in derivation.records
            )
        )


    def test_within_ensures_can_satisfy_target_invariants(self) -> None:
        result = build_model(
            parse_text(
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
                        invariant {
                            task_state_new(A);
                        }

                        events {
                            on Event::Enable -> State::Online {
                                within WakeContext {
                                    depends_on {
                                        task_state_new(A);
                                    }

                                    drives {
                                        A.Action::SetTaskState(TaskRuntimeState::Running);
                                        B.Action::EnqueueTask(task: A);
                                    }

                                    ensures {
                                        task_state_running(A);
                                        task_enqueued_on_runqueue(A, B);
                                    }
                                }

                                ensures {
                                    task_online(A);
                                }
                            }
                        }
                    }

                    state State::Online {
                        invariant {
                            task_online(A);
                            task_state_running(A);
                            task_enqueued_on_runqueue(A, B);
                        }
                    }
                }

                object B: T {
                    initial_state: State::Ready;

                    state State::Ready {
                    }
                }
                """
            )
        )

        derivation = derive(result.model, "A.Event::Enable")

        self.assertTrue(derivation.ok)
        self.assertTrue(
            any(
                record.status is DerivationStatus.PROVED
                and record.proof_provider == "within_ensures"
                and record.expression == "task_state_running(A)"
                for record in derivation.records
            )
        )
        self.assertTrue(
            any(
                record.status is DerivationStatus.PROVED
                and record.proof_class == "action_commit"
                and record.proof_provider == "within_context"
                and record.expression == "A.Action::SetTaskState(TaskRuntimeState::Running)"
                for record in derivation.records
            )
        )
        self.assertTrue(
            any(
                record.status is DerivationStatus.PROVED
                and record.proof_class == "exclusive_context_lock_event"
                and record.source_kind == "within_entered_by"
                and record.expression == "TaskPiLock.Event::LockIrqSave"
                for record in derivation.records
            )
        )
        self.assertTrue(
            any(
                record.status is DerivationStatus.PROVED
                and record.proof_class == "exclusive_context_lock_event"
                and record.source_kind == "within_exited_by"
                and record.expression == "TaskPiLock.Event::UnlockIrqRestore"
                for record in derivation.records
            )
        )

    def test_current_entry_prelude_derivation_reaches_target(self) -> None:
        spec = Path(__file__).resolve().parents[3] / "spec" / "entry-prelude-object-model.spec"
        result = build_model(parse_file(spec))

        derivation = derive(result.model)
        text = render_derivation_text(derivation)

        self.assertTrue(derivation.ok)
        self.assertIn("derive: ok", text)
        self.assertIn("target_reached: yes", text)
        self.assertIn("trace:", text)
        self.assertNotIn("categories:", text)
        self.assertNotIn("providers:", text)
        self.assertIn("proved:", text)
        self.assertIn("obligation: 0", text)
        self.assertTrue(
            any(
                record.status is DerivationStatus.PROVED
                and record.proof_provider == "event_ensures"
                and record.proof_class == "register_effect"
                for record in derivation.records
            )
        )
        self.assertIn("> StartupTimeline.Event::Setup State::Base", text)
        self.assertIn("  > PreparePhase.Event::Setup State::Base", text)
        self.assertIn("< StartupTimeline.Event::Setup State::Ready", text)
        self.assertEqual(derivation.states["StartupTimeline"], "Ready")
        self.assertEqual(derivation.states["EntrySuccessorPhase"], "Ready")
        self.assertEqual(derivation.states["CorePreparePhase"], "Ready")
        self.assertEqual(derivation.states["MmCoreInitPhase"], "Ready")
        self.assertEqual(derivation.states["PayloadPhase"], "Online")
        self.assertEqual(derivation.states["SwapperVm"], "Online")
        self.assertEqual(derivation.states["MemBlock"], "Offline")
        self.assertEqual(derivation.states["PageAllocator"], "Ready")
        self.assertFalse(derivation.blocked)


if __name__ == "__main__":
    unittest.main()
