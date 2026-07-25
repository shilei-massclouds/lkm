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
                object A: SystemObject {
                    initial_state: State::Base;

                    state State::Base {
                        transitions {
                            on Transition::Setup -> State::Ready {
                                drives {
                                    B.Transition::Setup;
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
                        transitions {
                            on Transition::Setup -> State::Ready {
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

        derivation = derive(result.model, "A.Transition::Setup")

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

    def test_derives_cross_object_emits_in_same_chain(self) -> None:
        result = build_model(
            parse_text(
                """
                object A: SystemObject {
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

                object B: PhaseObject {
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
                """
            )
        )

        derivation = derive(result.model, "A.Transition::Preset")

        self.assertTrue(derivation.ok)
        self.assertEqual(derivation.states["A"], "Prepared")
        self.assertEqual(derivation.states["B"], "Ready")
        self.assertEqual(
            [transition.object_name for transition in derivation.transitions],
            ["A", "B"],
        )
        self.assertEqual(derivation.trace[0].children[0].edge_kind, "emits")
        self.assertEqual(derivation.trace[0].children[0].object_name, "B")

    def test_blocks_when_dependency_state_is_missing(self) -> None:
        result = build_model(
            parse_text(
                """
                object A: SystemObject {
                    initial_state: State::Base;

                    state State::Base {
                        transitions {
                            on Transition::Setup -> State::Ready {
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

        derivation = derive(result.model, "A.Transition::Setup")

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


    def test_transition_ensures_match_multiline_invariant_predicates(self) -> None:
        result = build_model(
            parse_text(
                """
                object A: PhaseObject {
                    initial_state: State::Base;

                    state State::Base {
                        transitions {
                            on Transition::Setup -> State::Ready {
                                ensures {
                                    object_ref_targets(DeviceRef::VirtioMmioPlatformDevice, VirtioMmioTransportDevice);
                                }
                            }
                        }
                    }

                    state State::Ready {
                        invariant {
                            object_ref_targets(
                                DeviceRef::VirtioMmioPlatformDevice,
                                VirtioMmioTransportDevice
                            );
                        }
                    }
                }
                """
            )
        )

        derivation = derive(result.model, "A.Transition::Setup")

        self.assertTrue(derivation.ok)
        self.assertFalse(
            any(record.status is DerivationStatus.OBLIGATION for record in derivation.records)
        )
        self.assertTrue(
            any(
                record.status is DerivationStatus.PROVED
                and record.proof_provider == "transition_ensures"
                and record.predicate == "object_ref_targets"
                and "object_ref_targets(\n" in (record.expression or "")
                for record in derivation.records
            )
        )


    def test_action_result_binding_prefers_process_result_hints(self) -> None:
        result = build_model(
            parse_text(
                """
                predicate runqueue_pick_next_task_returns<T, U, V>(runqueue_ref: T, prev_ref: U, next_ref: V) -> bool;
                predicate task_ref_targets<T, U>(task_ref: T, task: U) -> bool;

                type Task {
                }

                type TaskRef {
                    processes {
                        Action::SetCurrent(task: Task) {
                            ensures {
                                task_ref_targets(self, task);
                            }
                        }
                    }
                }

                type RunQueue {
                    processes {
                        Action::PickNextTask(prev_ref: TaskRef) -> TaskRef {
                            ensures {
                                runqueue_pick_next_task_returns(CurrentRunQueueRef, prev_ref, CurrentTaskRef);
                            }
                        }
                    }
                }

                type Scheduler {
                    processes {
                        Action::Schedule {
                            drives {
                                let next: TaskRef <- CurrentRunQueueRef.Action::PickNextTask(CurrentTaskRef);
                                CurrentTaskRef.Action::SetCurrent(task: next);
                            }

                            ensures {
                                runqueue_pick_next_task_returns(CurrentRunQueueRef, CurrentTaskRef, KernelInitTaskRef);
                            }
                        }
                    }
                }

                object A: Scheduler {
                    initial_state: State::Base;

                    state State::Base {
                        transitions {
                            on Transition::Setup -> State::Ready {
                                drives {
                                    A.Action::Schedule;
                                }
                            }
                        }
                    }

                    state State::Ready {
                    }
                }
            """
            )
        )

        derivation = derive(result.model, "A.Transition::Setup")

        self.assertTrue(derivation.ok)
        self.assertTrue(
            any(
                record.status is DerivationStatus.PROVED
                and record.proof_class == "type_process_commit"
                and record.expression == "CurrentTaskRef.Action::SetCurrent(task: KernelInitTask)"
                for record in derivation.records
            )
        )
        self.assertTrue(
            any(
                record.status is DerivationStatus.PROVED
                and record.proof_class == "type_process_ensures"
                and record.expression == "task_ref_targets(CurrentTaskRef, KernelInitTask)"
                for record in derivation.records
            )
        )


    def test_within_ensures_can_satisfy_target_invariants(self) -> None:
        result = build_model(
            parse_text(
                """
                type RawSpinLock {
                    processes {
                        Transition::LockIrqSave {
                        }

                        Transition::UnlockIrqRestore {
                        }
                    }
                }

                lock TaskPiLock: RawSpinLock;

                context WakeContext: ResourceExclusiveContext {
                    guard {
                        lock_ref: TaskPiLock;

                        entered_by {
                            TaskPiLock.Transition::LockIrqSave;
                        }

                        exited_by {
                            TaskPiLock.Transition::UnlockIrqRestore;
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

                        transitions {
                            on Transition::Enable -> State::Online {
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

        derivation = derive(result.model, "A.Transition::Enable")

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
                and record.proof_class == "context_guard_transition"
                and record.source_kind == "within_entered_by"
                and record.expression == "TaskPiLock.Transition::LockIrqSave"
                for record in derivation.records
            )
        )
        self.assertTrue(
            any(
                record.status is DerivationStatus.PROVED
                and record.proof_class == "context_guard_transition"
                and record.source_kind == "within_exited_by"
                and record.expression == "TaskPiLock.Transition::UnlockIrqRestore"
                for record in derivation.records
            )
        )

    def test_within_action_guard_boundary_records(self) -> None:
        result = build_model(
            parse_text(
                """
                type RawSpinLock {
                    processes {
                        Action::Acquire {
                        }

                        Action::Release {
                        }
                    }
                }

                lock TaskRqLock: RawSpinLock;

                context RqLockContext: ResourceExclusiveContext {
                    guard {
                        lock_ref: TaskRqLock;

                        entered_by {
                            TaskRqLock.Action::Acquire;
                        }

                        exited_by {
                            TaskRqLock.Action::Release;
                        }
                    }

                    obj_refs {
                        A;
                    }
                }

                object A: T {
                    initial_state: State::Ready;

                    state State::Ready {
                        transitions {
                            on Transition::Enable -> State::Online {
                                within RqLockContext {
                                    drives {
                                        A.Action::Touch;
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
        )

        derivation = derive(result.model, "A.Transition::Enable")

        self.assertTrue(derivation.ok)
        self.assertTrue(
            any(
                record.status is DerivationStatus.PROVED
                and record.proof_class == "context_guard_action"
                and record.source_kind == "within_entered_by"
                and record.expression == "TaskRqLock.Action::Acquire"
                for record in derivation.records
            )
        )
        self.assertTrue(
            any(
                record.status is DerivationStatus.PROVED
                and record.proof_class == "context_guard_action"
                and record.source_kind == "within_exited_by"
                and record.expression == "TaskRqLock.Action::Release"
                for record in derivation.records
            )
        )

    def test_current_model_derivation_reaches_target(self) -> None:
        spec = Path(__file__).resolve().parents[3] / "spec" / "model" / "main.spec"
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
        boot_args_invariants = {
            record.predicate
            for record in derivation.records
            if record.object_name == "BootArgs"
            and record.state_name == "Online"
            and record.proof_class == "firmware_boot_abi"
            and record.proof_provider == "firmware_boot_abi"
        }
        self.assertEqual(
            boot_args_invariants,
            {
                "attrs_accessible",
                "firmware_boot_args_defined",
                "boot_args_read_only",
            },
        )
        self.assertFalse(
            any(
                transition.object_name == "BootArgs"
                for transition in derivation.transitions
            )
        )
        self.assertTrue(
            any(
                record.status is DerivationStatus.PROVED
                and record.proof_provider == "transition_ensures"
                and record.proof_class == "register_effect"
                for record in derivation.records
            )
        )
        self.assertIn("> Computer.Transition::Preset State::Base", text)
        self.assertIn("> Riscv64Platform.Transition::Preset State::Base", text)
        self.assertIn("> OpenSBI.Transition::Preset State::Base", text)
        self.assertIn("  > Computer.Transition::Setup State::Prepared", text)
        self.assertIn("> Computer.Transition::Enable State::Ready", text)
        self.assertIn("> Riscv64Platform.Transition::Enable State::Ready", text)
        self.assertIn("> OpenSBI.Transition::Enable State::Ready", text)
        self.assertIn("> Config.Transition::Enable State::Ready", text)
        self.assertIn("> Lds.Transition::Enable State::Ready", text)
        self.assertIn("> Kernel.Transition::Preset State::Base", text)
        self.assertIn("> EntrySuccessorPhase.Transition::Setup State::Prepared", text)
        self.assertIn("< Computer.Transition::Preset State::Prepared", text)
        self.assertEqual(derivation.states["BootArgs"], "Online")
        self.assertEqual(derivation.states["Config"], "Online")
        self.assertEqual(derivation.states["Lds"], "Online")
        self.assertEqual(derivation.states["Computer"], "Online")
        self.assertEqual(derivation.states["Riscv64Platform"], "Online")
        self.assertEqual(derivation.states["BootCpuRegisters"], "Online")
        self.assertEqual(derivation.states["OpenSBI"], "Online")
        self.assertNotIn("OpenSbi" + "Firmware", derivation.states)
        self.assertEqual(derivation.states["Kernel"], "Online")
        self.assertEqual(derivation.states["EntrySuccessorPhase"], "Online")
        self.assertEqual(derivation.states["CorePreparePhase"], "Online")
        self.assertEqual(derivation.states["MmCoreInitPhase"], "Online")
        self.assertEqual(derivation.states["PayloadPreparePhase"], "Base")
        self.assertEqual(derivation.states["PayloadHandoffPreparePhase"], "Base")
        self.assertEqual(derivation.states["SwapperVm"], "Online")
        self.assertEqual(derivation.states["MemBlock"], "Offline")
        self.assertEqual(derivation.states["PageAllocator"], "Ready")
        self.assertFalse(derivation.blocked)


if __name__ == "__main__":
    unittest.main()
