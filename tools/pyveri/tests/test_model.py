from __future__ import annotations

import unittest
from pathlib import Path

from pyveri.derive import derive
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
        self.assertIn("Computer", result.model.objects)
        self.assertNotIn("OpenSbi" + "Firmware", result.model.objects)
        self.assertEqual(result.model.objects["Config"].initial_state, "Ready")
        self.assertEqual(result.model.objects["Lds"].initial_state, "Ready")
        self.assertEqual(result.model.objects["Computer"].initial_state, "Base")
        self.assertEqual(result.model.objects["Riscv64Platform"].initial_state, "Base")
        self.assertEqual(result.model.objects["OpenSBI"].initial_state, "Base")
        self.assertEqual(
            result.model.children["Computer"],
            [
                "Riscv64Platform",
                "OpenSBI",
                "Kernel",
            ],
        )
        self.assertEqual(result.model.children["Riscv64Platform"], ["Riscv64"])
        self.assertIn("Config", result.model.children["Kernel"])
        self.assertIn("Lds", result.model.children["Kernel"])
        self.assertEqual(result.model.objects["Kernel"].parent, "Computer")
        self.assertEqual(result.model.objects["Computer"].parent, None)
        self.assertEqual(result.model.objects["Riscv64"].attrs, {})
        self.assertEqual(
            result.model.objects["BootCpuRegisters"].parent, "CpuGroup.cpus[0]"
        )
        self.assertNotIn("BootCPU", result.model.objects)
        self.assertNotIn("BootCurrentCPU", result.model.objects)
        self.assertEqual(result.model.objects["BootCpuRegisters"].initial_state, "Online")
        self.assertEqual(
            list(result.model.objects["BootCpuRegisters"].attrs),
            ["a0", "a1", "sp", "tp", "gp", "sstatus", "sie", "sip", "stvec", "sscratch", "satp"],
        )
        boot_args = result.model.objects["BootArgs"]
        self.assertEqual(boot_args.initial_state, "Online")
        self.assertEqual(boot_args.parent, "OpenSBI")
        self.assertEqual(boot_args.decl.properties["source"], "firmware::boot_abi")
        self.assertEqual(
            boot_args.attrs,
            {"boot_hartid": "HartId", "dtb_pa": "PhysAddr<Dtb>"},
        )
        self.assertEqual(list(boot_args.states), ["Online"])
        self.assertFalse(boot_args.states["Online"].transitions)
        self.assertEqual(
            result.model.objects["OpenSBI"].states["Ready"].transitions["Enable"].target_state,
            "Online",
        )
        self.assertEqual(
            result.model.children["BootInitFlow"],
            [
                "EntrySuccessorPhase",
                "CorePreparePhase",
                "MmCoreInitPhase",
                "SchedInitPhase",
                "IrqTimeInitPhase",
                "LocalIrqEnablePhase",
                "IrqOpenPreparePhase",
                "ProcessPreparePhase",
                "RawDtb",
                "BootInitRestInitPhase",
                "BootInitScheduleHandoffPhase",
            ],
        )
        self.assertEqual(result.model.objects["BootTask"].initial_state, "OnCpu")

    def test_builds_object_view(self) -> None:
        spec = Path(__file__).resolve().parents[3] / "spec" / "model" / "main.spec"

        result = build_model(parse_file(spec))
        view = build_object_view(result.model)
        text = render_text(view)
        dot = render_dot(view)

        self.assertIn("Computer: ComputerObject", text)
        self.assertIn("Computer -> Riscv64Platform [parent]", text)
        self.assertIn("Computer -> OpenSBI [parent]", text)
        self.assertIn("Computer -> Kernel [parent]", text)
        self.assertIn("Kernel -> Soc [parent]", text)
        self.assertIn("Kernel -> CpuGroup [parent]", text)
        self.assertNotIn("Soc -> CpuGroup [parent]", text)
        self.assertIn("CpuGroup.cpus[0] -> BootCpuRegisters [parent]", text)
        self.assertNotIn("BootCurrentCPU", text)
        self.assertIn("BootTask -> BootInitFlow [parent]", text)
        self.assertNotIn("\nEntryPreludePhase:", text)
        self.assertNotIn("drives", text)
        self.assertIn('"Computer" -> "Riscv64Platform"', dot)
        self.assertIn('"Computer" -> "OpenSBI"', dot)
        self.assertIn('"Computer" -> "Kernel"', dot)
        self.assertIn('"BootTask" -> "BootInitFlow"', dot)
        self.assertNotIn('"EntryPreludePhase"', dot)
        self.assertNotIn("drives", dot)

    def test_builds_drives_view(self) -> None:
        spec = Path(__file__).resolve().parents[3] / "spec" / "model" / "main.spec"

        result = build_model(parse_file(spec))
        view = build_drives_view(result.model)
        text = render_text(view)
        dot = render_dot(view)

        self.assertIn("Computer.Preset", text)
        self.assertIn("  -> Riscv64Platform.Preset", text)
        self.assertIn("  -> OpenSBI.Preset", text)
        self.assertIn("  -> Kernel.Preset", text)
        self.assertIn("external::Human", text)
        self.assertIn("  -> Computer.Preset", text)
        self.assertIn("  -> Computer.Setup", text)
        self.assertIn("  -> Computer.Enable [emits]", text)
        self.assertIn("Computer.Setup", text)
        self.assertIn("  -> Riscv64Platform.Setup", text)
        self.assertIn("  -> OpenSBI.Setup", text)
        self.assertIn("  -> Kernel.Setup", text)
        self.assertIn("Computer.Enable", text)
        self.assertIn("Computer.Enable", text)
        self.assertIn("  -> Riscv64Platform.Enable [emits]", text)
        self.assertIn("Riscv64Platform.Enable", text)
        self.assertIn("  -> OpenSBI.Enable [emits]", text)
        self.assertNotIn("-> BootCpuRegisters.", text)
        self.assertIn("OpenSBI.Enable", text)
        self.assertIn("  -> Kernel.Enable [emits]", text)
        self.assertIn("Kernel.Preset", text)
        self.assertIn("BootInitFlow.Preset", text)
        self.assertIn("  -> KernelAddrSpace.Preset", text)
        self.assertIn("  -> CurrentTask.Action.BindTask [drives_action]", text)
        self.assertIn("Kernel.Setup", text)
        self.assertNotIn("\nEntryPreludePhase.", text)
        self.assertIn("EntrySuccessorPhase.Setup", text)
        self.assertIn("CorePreparePhase.Setup", text)
        self.assertIn("MmCoreInitPhase.Setup", text)
        self.assertIn("PayloadPreparePhase.Setup", text)
        self.assertIn("rankdir=LR", dot)
        self.assertIn('"external::Human" -> "Computer.Preset"', dot)
        self.assertIn('"external::Human" -> "Computer.Setup"', dot)
        self.assertIn('"external::Human" -> "Computer.Enable"', dot)
        self.assertIn('"Computer.Preset" -> "Riscv64Platform.Preset"', dot)
        self.assertIn('"Computer.Enable" -> "Riscv64Platform.Enable"', dot)
        self.assertIn('"Riscv64Platform.Enable" -> "OpenSBI.Enable"', dot)
        self.assertIn('"OpenSBI.Enable" -> "Kernel.Enable"', dot)
        self.assertIn('"Kernel.Enable" -> "BootInitFlow.Preset"', dot)
        self.assertIn('"BootInitFlow.Preset" -> "KernelAddrSpace.Preset"', dot)
        self.assertIn(
            '"BootInitFlow.Preset" -> "CurrentTask.Action.BindTask"', dot
        )
        self.assertIn('"Kernel.Enable" -> "BootInitFlow.Setup"', dot)
        self.assertIn('"Kernel.Enable" -> "BootInitFlow.Enable"', dot)
        self.assertIn('"Kernel.Enable" -> "Scheduler.Action.Schedule"', dot)
        self.assertIn('"Kernel.Enable" -> "KernelInitFlow.Setup"', dot)
        self.assertIn('"Kernel.Enable" -> "KernelInitFlow.Enable"', dot)
        self.assertNotIn('"BootInitFlow.Preset" -> "BootInitFlow.Setup"', dot)
        self.assertIn('"BootInitFlow.Setup" -> "EntrySuccessorPhase.Preset"', dot)

    def test_builds_timeline_view(self) -> None:
        spec = Path(__file__).resolve().parents[3] / "spec" / "model" / "main.spec"

        result = build_model(parse_file(spec))
        view = build_timeline_view(result.model)
        text = render_text(view)
        svg = render_svg(view)

        self.assertIn("timeline view:", text)
        self.assertNotIn("BootCurrentCPU.State::", text)
        self.assertNotIn("BootCPU.State::", text)
        self.assertIn("  - PhysicalMemory.State::Online", text)
        self.assertNotIn("\nEntryPreludePhase:", text)
        self.assertIn("EntrySuccessorPhase: ready (State::Ready)", text)
        self.assertIn("CorePreparePhase: ready (State::Ready)", text)
        self.assertIn("MmCoreInitPhase: ready (State::Ready)", text)
        self.assertIn("  - BootInitFlow.State::Online", text)
        self.assertIn("  - Soc.State::Prepared", text)
        self.assertIn("  - Vm.State::Online", text)
        self.assertIn("  - KernelAddrSpace.State::Online", text)
        self.assertIn("  - SwapperVm.State::Ready", text)
        self.assertIn("  - MemBlock.State::Offline", text)
        self.assertNotIn("BootArgs.State::", text)
        self.assertIn("Config.State::Online", text)
        self.assertIn("Lds.State::Online", text)
        self.assertIn("Kernel: ready (State::Ready)", text)
        self.assertIn("Kernel: online (State::Online)", text)
        self.assertIn("PayloadPreparePhase: online (State::Online)", text)
        self.assertIn("PayloadHandoffPreparePhase: online (State::Online)", text)
        self.assertIn("<svg", svg)
        self.assertNotIn(">EntryPreludePhase<", svg)
        self.assertIn("EntrySuccessorPhase", svg)
        self.assertIn("CorePreparePhase", svg)
        self.assertIn("MmCoreInitPhase", svg)
        self.assertIn("PayloadPreparePhase", svg)
        self.assertIn("PayloadHandoffPreparePhase", svg)
        self.assertNotIn("BootArgs", svg)

    def test_accepts_exclusive_context_within_action_refs(self) -> None:
        document = parse_text(
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
                    transitions {
                        on Transition::Enable -> State::Online {
                            within WakeContext {
                                entered_by {
                                    TaskPiLock.Transition::LockIrqSave;
                                }

                                drives {
                                    A.Action::SetTaskState(TaskRuntimeState::Running);
                                    B.Action::Touch(task: A);
                                }

                                exited_by {
                                    TaskPiLock.Transition::UnlockIrqRestore;
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
                    transitions {
                        on Transition::Enable -> State::Online {
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

    def test_accepts_resource_context_guard_lock_action_refs(self) -> None:
        document = parse_text(
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

        result = build_model(document)

        self.assertTrue(result.ok, [diag.message for diag in result.errors])
        context = result.model.exclusive_contexts["RqLockContext"]
        self.assertEqual(context.lock_ref, "TaskRqLock")
        self.assertEqual(result.model.locks["TaskRqLock"].kind, "RawSpinLock")

    def test_rejects_within_boundary_override_for_context_guard(self) -> None:
        document = parse_text(
            """
            type RawSpinLock {
                processes {
                    Transition::LockIrqSave {
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
                            within WakeContext {
                                entered_by {
                                    TaskPiLock.Transition::LockIrqSave;
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

    def test_rejects_context_guard_unpaired_enter_boundary(self) -> None:
        document = parse_text(
            """
            type InterruptType {
                processes {
                    Action::EnableLocal {
                    }
                }
            }

            context BadContext: Context {
                guard {
                    entered_by {
                        BootCpuInterrupt.Action::EnableLocal;
                    }
                }

                obj_refs {
                    BootCpuInterrupt;
                }
            }

            object BootCpuInterrupt: InterruptType {
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
                "context guard on BadContext must pair entered_by with exited_by"
                in diag.message
                and diag.severity is Severity.ERROR
                for diag in result.errors
            )
        )

    def test_rejects_context_guard_boundary_and_holds_mix(self) -> None:
        document = parse_text(
            """
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
                "context guard on BadContext must not mix entered_by/exited_by with holds"
                in diag.message
                and diag.severity is Severity.ERROR
                for diag in result.errors
            )
        )

    def test_rejects_context_guard_mixed_enter_owners(self) -> None:
        document = parse_text(
            """
            type Mutex {
                processes {
                    Transition::Lock(current_task: TaskRef) {
                    }

                    Transition::Unlock(current_task: TaskRef) {
                    }
                }
            }

            lock TaskMutex: Mutex;

            context BadContext: ResourceExclusiveContext {
                guard {
                    lock_ref: TaskMutex;

                    entered_by {
                        TaskMutex.Transition::Lock(BootTaskRef);
                        TaskMutex.Transition::Lock(KernelInitTaskRef);
                    }

                    exited_by {
                        TaskMutex.Transition::Unlock(BootTaskRef);
                    }
                }

                obj_refs {
                    A;
                }
            }

            object A: T {
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
                "context guard entered_by must not mix owner arguments" in diag.message
                and diag.severity is Severity.ERROR
                for diag in result.errors
            )
        )

    def test_rejects_context_guard_mismatched_enter_exit_owner(self) -> None:
        document = parse_text(
            """
            type Mutex {
                processes {
                    Transition::Lock(current_task: TaskRef) {
                    }

                    Transition::Unlock(current_task: TaskRef) {
                    }
                }
            }

            lock TaskMutex: Mutex;

            context BadContext: ResourceExclusiveContext {
                guard {
                    lock_ref: TaskMutex;

                    entered_by {
                        TaskMutex.Transition::Lock(BootTaskRef);
                    }

                    exited_by {
                        TaskMutex.Transition::Unlock(KernelInitTaskRef);
                    }
                }

                obj_refs {
                    A;
                }
            }

            object A: T {
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
                "context guard entered_by/exited_by owner arguments must match"
                in diag.message
                and diag.severity is Severity.ERROR
                for diag in result.errors
            )
        )

    def test_rejects_within_boundary_for_non_context_lock(self) -> None:
        document = parse_text(
            """
            type RawSpinLock {
                processes {
                    Transition::LockIrqSave {
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
                    transitions {
                        on Transition::Enable -> State::Online {
                            within WakeContext {
                                entered_by {
                                    OtherLock.Transition::LockIrqSave;
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
                "lock transition reference outside exclusive_context lock_ref" in diag.message
                and diag.severity is Severity.ERROR
                for diag in result.errors
            )
        )

    def test_rejects_guard_action_for_non_context_lock(self) -> None:
        document = parse_text(
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
            lock OtherLock: RawSpinLock;

            context RqLockContext: ResourceExclusiveContext {
                guard {
                    lock_ref: TaskRqLock;

                    entered_by {
                        OtherLock.Action::Acquire;
                    }

                    exited_by {
                        OtherLock.Action::Release;
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
                "lock action reference outside exclusive_context lock_ref" in diag.message
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
                    transitions {
                        on Transition::Enable -> State::Online {
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
                    transitions {
                        on Transition::Setup -> State::Ready {
                            drives {
                                B.Transition::Missing;
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
            any("unknown transition reference: B.Transition::Missing" in diag.message for diag in result.errors)
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

    def test_rejects_duplicate_object_transition_names_across_states(self) -> None:
        document = parse_text(
            """
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
        )

        result = build_model(document)

        self.assertFalse(result.ok)
        self.assertTrue(
            any(
                "duplicate object transition declaration: A.Transition::Enable" in diag.message
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
                    transitions {
                        on Transition::Activate -> State::Done {
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
            any("A.Transition::Activate" in message for message in messages)
        )
        self.assertTrue(
            any("A.Transition::Activate -> State::Done" in message for message in messages)
        )

    def test_accepts_disable_and_offline_lifecycle_names_and_transitions(self) -> None:
        document = parse_text(
            """
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
        )

        result = build_model(document)

        self.assertTrue(result.ok, [diag.message for diag in result.errors])

    def test_accepts_terminal_disable_from_on_cpu_for_task(self) -> None:
        document = parse_text(
            """
            type Task {}
            object Worker: Task {
                initial_state: State::OnCpu;

                state State::OnCpu {
                    transitions {
                        on Transition::Disable -> State::Offline {}
                    }
                }

                state State::Offline {}
            }
            """
        )

        result = build_model(document)

        self.assertTrue(result.ok, [diag.message for diag in result.errors])

    def test_rejects_only_once_when_event_reachable_twice(self) -> None:
        document = parse_text(
            """
            context GuardedContext: Context {
            }

            object Computer: SystemObject {
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
        )

        result = build_model(document)

        self.assertFalse(result.ok)
        self.assertTrue(
            any(
                "within only-once proof failed: GuardedContext is reachable 2 times"
                in diag.message
                for diag in result.errors
            )
        )

    def test_derive_executes_ordered_body_members(self) -> None:
        document = parse_text(
            """
            context GuardedContext: Context {
            }

            object Computer: SystemObject {
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
        )
        model_result = build_model(document)
        self.assertTrue(model_result.ok, [diag.message for diag in model_result.errors])

        derive_result = derive(model_result.model, "Computer.Transition::Preset")

        self.assertTrue(derive_result.ok)
        order = [
            (transition.object_name, transition.transition_name)
            for transition in derive_result.transitions
        ]
        self.assertEqual(
            order,
            [
                ("B", "Setup"),
                ("C", "Setup"),
                ("D", "Setup"),
                ("A", "Setup"),
                ("Computer", "Preset"),
            ],
        )

    def test_rejects_lifecycle_transitions_outside_controlled_table(self) -> None:
        document = parse_text(
            """
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
        )

        result = build_model(document)

        self.assertFalse(result.ok)
        self.assertTrue(
            any(
                "invalid lifecycle transition: A.State::Base.Transition::Enable -> State::Online"
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
                    transitions {
                        on Transition::Disable -> State::Offline {
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
                "invalid lifecycle transition: A.State::Base.Transition::Disable -> State::Offline"
                in diag.message
                and diag.severity is Severity.ERROR
                for diag in result.errors
            )
        )


if __name__ == "__main__":
    unittest.main()
