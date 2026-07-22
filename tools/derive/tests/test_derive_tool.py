from __future__ import annotations

import contextlib
import io
import tempfile
import unittest
from pathlib import Path

from common import DERIVE_SCHEMA, DERIVE_VERSION, read_json
from derive_tool.__main__ import main as derive_main
from model_tool.__main__ import main as model_main
from parse_tool.__main__ import main as parse_main


class DeriveToolTests(unittest.TestCase):
    def setUp(self) -> None:
        self.spec = (
            Path(__file__).resolve().parents[3]
            / "spec"
            / "model"
            / "main.spec"
        )

    def _build_model_json(self, tmp: str) -> Path:
        ast = Path(tmp) / "model-main.ast.json"
        model = Path(tmp) / "model-main.model.json"
        self.assertEqual(parse_main([str(self.spec), "-o", str(ast)]), 0)
        self.assertEqual(model_main([str(ast), "-o", str(model)]), 0)
        return model

    def _derive_source(self, root: Path, source: str) -> dict:
        spec = root / "input.spec"
        ast = root / "input.ast.json"
        model = root / "input.model.json"
        derive = root / "input.derive.json"
        spec.write_text(source, encoding="utf-8")
        self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
        self.assertEqual(model_main([str(ast), "-o", str(model)]), 0)
        self.assertEqual(derive_main([str(model), "-o", str(derive)]), 0)
        return read_json(derive)

    @staticmethod
    def _dynamic_emit_fixture(*, strict_initial_emit: bool = False) -> str:
        initial_prefix = "" if strict_initial_emit else "lossy "
        return f"""
            type PhaseObject {{}}

            type TaskFlow: PhaseObject {{
                parent: Task;
                initial_state: State::Base;

                state State::Base {{
                    transitions {{
                        on Transition::Preset -> State::Prepared {{
                            depends_on {{
                                task_flow_dispatch_guard_satisfied(self, BootDispatchWindow);
                            }}
                            emits {{ Transition::Setup; }}
                        }}
                    }}
                }}
                state State::Prepared {{
                    transitions {{
                        on Transition::Setup -> State::Ready {{
                            depends_on {{
                                task_flow_dispatch_guard_satisfied(self, BootDispatchWindow);
                            }}
                            emits {{ Transition::Enable; }}
                        }}
                    }}
                }}
                state State::Ready {{
                    transitions {{
                        on Transition::Enable -> State::Online {{
                            depends_on {{
                                task_flow_dispatch_guard_satisfied(self, BootDispatchWindow);
                            }}
                        }}
                    }}
                }}
                state State::Online {{}}
            }}

            type Task {{
                associations {{ initial_flow: TaskFlow; }}
            }}

            type TaskRef {{}}

            type DispatchWindowObject {{
                associations {{ mutable current_task: TaskRef; }}
                processes {{
                    Transition::SwitchTo(next_ref: TaskRef) {{
                        updates {{ self.current_task = next_ref; }}
                        emits {{
                            lossy self.current_task.initial_flow.Transition::Preset;
                        }}
                    }}
                }}
            }}

            object BootTask: Task {{
                initial_state: State::Online;
                associations {{ initial_flow = BootInitFlow; }}
                state State::Online {{}}
            }}

            object KernelInitTask: Task {{
                initial_state: State::Online;
                state State::Online {{}}
            }}

            object BootInitFlow: TaskFlow {{
                parent: BootTask;
            }}

            object BootDispatchWindow: DispatchWindowObject {{
                initial_state: State::Online;
                associations {{ current_task = KernelInitTaskRef; }}
                state State::Online {{}}
            }}

            object ComputerProject: Task {{
                initial_state: State::Base;
                state State::Base {{
                    transitions {{
                        on Transition::Preset -> State::Prepared {{
                            emits {{
                                {initial_prefix}BootTask.initial_flow.Transition::Preset;
                                Transition::Setup;
                            }}
                        }}
                    }}
                }}
                state State::Prepared {{
                    transitions {{
                        on Transition::Setup -> State::Ready {{
                            drives {{
                                BootDispatchWindow.Transition::SwitchTo(BootTaskRef);
                            }}
                            emits {{ Transition::Enable; }}
                        }}
                    }}
                }}
                state State::Ready {{
                    transitions {{
                        on Transition::Enable -> State::Online {{
                            emits {{
                                lossy BootTask.initial_flow.Transition::Preset;
                            }}
                        }}
                    }}
                }}
                state State::Online {{}}
            }}
        """

    def test_dynamic_lossy_emit_uses_current_window_and_dereferences_task_ref(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            data = self._derive_source(
                Path(tmp), self._dynamic_emit_fixture()
            )

        self.assertEqual(data["summary"]["blocked"], 0)
        self.assertEqual(data["summary"]["contradiction"], 0)
        self.assertEqual(data["states"]["BootInitFlow"], "Online")
        flow_transitions = [
            item["transition"]
            for item in data["transitions"]
            if item["object"] == "BootInitFlow"
        ]
        self.assertEqual(flow_transitions, ["Preset", "Setup", "Enable"])
        discarded = [
            record
            for record in data["records"]
            if record["source_kind"] == "emits_lossy_discarded"
        ]
        self.assertEqual(len(discarded), 2)
        self.assertIn("current_task=KernelInitTask", discarded[0]["message"])
        self.assertIn("not enabled", discarded[1]["message"])
        self.assertTrue(
            any(
                record["source_kind"] == "updates"
                and "BootDispatchWindow.current_task = BootTaskRef"
                in record["message"]
                for record in data["records"]
            )
        )
    def test_strict_emit_blocks_when_dynamic_guard_is_false(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            data = self._derive_source(
                Path(tmp),
                self._dynamic_emit_fixture(strict_initial_emit=True),
            )

        self.assertGreater(data["summary"]["blocked"], 0)
        self.assertEqual(data["states"]["BootInitFlow"], "Base")
        self.assertFalse(
            any(
                record["source_kind"] == "emits_lossy_discarded"
                for record in data["records"]
            )
        )

    @staticmethod
    def _dynamic_fixture(factory_processes: str) -> str:
        return f"""
            type TaskFlow {{
            }}

            type UserAppFlow: TaskFlow {{
                lifecycle {{
                    Transition::Preset(owner_task: Task) {{
                    }}
                    Transition::Setup {{
                    }}
                    Transition::Enable {{
                    }}
                    Transition::Disable {{
                    }}
                    Transition::Cleanup {{
                    }}
                }}
            }}

            type Task {{
                initial_state: State::Base;

                state State::Base {{
                    transitions {{
                        on Transition::Preset(initial_flow: UserAppFlow) -> State::Prepared {{
                        }}
                    }}
                }}
                state State::Prepared {{
                    transitions {{
                        on Transition::Setup -> State::Ready {{
                        }}
                    }}
                }}
                state State::Ready {{
                    transitions {{
                        on Transition::Enable -> State::Online {{
                        }}
                    }}
                }}
                state State::Online {{
                    transitions {{
                        on Transition::Disable -> State::Offline {{
                        }}
                    }}
                }}
                state State::Offline {{
                    transitions {{
                        on Transition::Cleanup -> State::Destroyed {{
                        }}
                    }}
                }}
                state State::Destroyed {{
                }}
            }}

            type TaskRef {{
                processes {{
                    Action::SetCurrent(task: Task) {{
                    }}
                }}
            }}

            type FactoryType {{
                processes {{
                    {factory_processes}
                }}
            }}

            object Factory: FactoryType {{
                initial_state: State::Base;
                state State::Base {{
                }}
            }}

            object ComputerProject: FactoryType {{
                initial_state: State::Base;
                state State::Base {{
                    transitions {{
                        on Transition::Preset -> State::Prepared {{
                            drives {{
                                ComputerProject.Action::Run;
                            }}
                        }}
                    }}
                }}
                state State::Prepared {{
                }}
            }}
        """

    def test_derive_writes_derive_json(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            model = self._build_model_json(tmp)
            derive = Path(tmp) / "model-main.derive.json"

            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exit_code = derive_main([str(model), "-o", str(derive)])

            self.assertEqual(exit_code, 0)
            data = read_json(derive)
            self.assertEqual(data["schema"], DERIVE_SCHEMA)
            self.assertEqual(data["version"], DERIVE_VERSION)
            self.assertTrue(data["target"]["reached"])
            self.assertTrue(data["summary"]["ok"])
            self.assertEqual(data["summary"]["blocked"], 0)
            self.assertEqual(data["summary"]["contradiction"], 0)
            self.assertEqual(data["states"]["ComputerProject"], "Online")
            self.assertEqual(data["states"]["KernelProject"], "Online")
            self.assertEqual(data["states"]["OpenSBI"], "Online")
            self.assertNotIn("OpenSbi" + "Firmware", data["states"])
            self.assertEqual(data["states"]["Kernel"], "Online")
            self.assertIn("locks", data["model"])
            self.assertIn("exclusive_contexts", data["model"])

    def test_derive_json_has_separate_structured_boundary_counts(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            model = self._build_model_json(tmp)
            derive = Path(tmp) / "model-main.derive.json"
            self.assertEqual(derive_main([str(model), "-o", str(derive)]), 0)

            data = read_json(derive)
            boundaries = data["boundaries"]
            self.assertEqual(
                data["summary"]["deferred"],
                sum(item["status"] == "deferred" for item in boundaries),
            )
            self.assertEqual(
                data["summary"]["trimmed"],
                sum(item["status"] == "trimmed" for item in boundaries),
            )
            clone = next(item for item in boundaries if item["boundary_id"] == "user_clone.001")
            self.assertEqual(clone["boundary_category"], "Feature")
            self.assertEqual(
                clone["boundary_owner"],
                "UserCloneDeferredBoundaries.Transition::Setup",
            )
            self.assertTrue(clone["span"]["source_file"].endswith("objects/user_boot.spec"))
            self.assertGreater(clone["span"]["source_line"], 0)

    def test_unproved_boundary_evidence_creates_obligation(self) -> None:
        source = """
            object ComputerProject: ProjectObject {
                initial_state: State::Base;

                state State::Base {
                    transitions {
                        on Transition::Preset -> State::Prepared {
                            deferred demo.001 {
                                category: DeferredCategory::Proof;
                                summary: "Prove the missing demo fact.";
                                evidence { missing_demo_evidence(ComputerProject); }
                                close_when: "The fact has a formal provider and a regression test.";
                            }
                        }
                    }
                }

                state State::Prepared {
                }
            }
        """
        with tempfile.TemporaryDirectory() as tmp:
            spec = Path(tmp) / "missing-evidence.spec"
            ast = Path(tmp) / "missing-evidence.ast.json"
            model = Path(tmp) / "missing-evidence.model.json"
            derive = Path(tmp) / "missing-evidence.derive.json"
            spec.write_text(source, encoding="utf-8")

            self.assertEqual(parse_main([str(spec), "-o", str(ast)]), 0)
            self.assertEqual(model_main([str(ast), "-o", str(model)]), 0)
            self.assertEqual(derive_main([str(model), "-o", str(derive)]), 0)
            data = read_json(derive)

            self.assertTrue(data["target"]["reached"])
            self.assertEqual(data["summary"]["deferred"], 1)
            self.assertEqual(data["summary"]["trimmed"], 0)
            self.assertEqual(data["summary"]["obligation"], 1)
            obligation = next(
                item for item in data["records"] if item["status"] == "obligation"
            )
            self.assertIn("deferred demo.001 evidence", obligation["source_kind"])
            self.assertEqual(
                obligation["expression"], "missing_demo_evidence(ComputerProject)"
            )

    def test_derive_json_contains_records_transitions_and_trace(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            model = self._build_model_json(tmp)
            derive = Path(tmp) / "model-main.derive.json"
            self.assertEqual(derive_main([str(model), "-o", str(derive)]), 0)

            data = read_json(derive)
            self.assertGreaterEqual(data["summary"]["transitions"], 55)
            self.assertEqual(data["summary"]["obligation"], 0)
            self.assertIn("obligation_categories", data["summary"])
            self.assertNotIn(
                "assumption_candidate", data["summary"]["obligation_categories"]
            )
            self.assertNotIn("auto_candidate", data["summary"]["obligation_categories"])
            self.assertNotIn("spec_gap", data["summary"]["obligation_categories"])
            self.assertNotIn("unknown", data["summary"]["obligation_categories"])
            self.assertTrue(
                any(
                    transition["object"] == "ComputerProject"
                    and transition["transition"] == "Preset"
                    for transition in data["transitions"]
                )
            )

            self.assertTrue(
                any(
                    transition["object"] == "EntrySuccessorPhase"
                    and transition["transition"] == "Setup"
                    for transition in data["transitions"]
                )
            )
            self.assertEqual(data["states"]["EntryPreludePhase"], "Online")
            self.assertEqual(data["states"]["EntrySuccessorPhase"], "Online")
            self.assertEqual(data["states"]["CorePreparePhase"], "Online")
            self.assertEqual(data["states"]["MmCoreInitPhase"], "Online")
            self.assertEqual(data["states"]["SwapperVm"], "Online")
            self.assertEqual(data["states"]["MemBlock"], "Offline")
            self.assertEqual(data["states"]["PageAllocator"], "Ready")
            self.assertEqual(data["states"]["VmallocAllocator"], "Ready")
            self.assertEqual(len(data["trace"]), 1)
            root = data["trace"][0]
            self.assertEqual(root["object"], "ComputerProject")
            self.assertEqual(root["transition"], "Preset")
            self.assertEqual(root["source_state"], "Base")
            self.assertEqual(root["target_state"], "Prepared")
            self.assertEqual(root["status"], "proved")
            self.assertGreater(len(root["children"]), 0)
            self.assertEqual(root["children"][0]["edge_kind"], "emits")
            self.assertTrue(
                any(record["span"] is not None for record in data["records"])
            )
            obligations = [
                record for record in data["records"] if record["status"] == "obligation"
            ]
            proved = [record for record in data["records"] if record["status"] == "proved"]
            self.assertFalse(
                any(record["proof_class"] == "register_effect" for record in obligations)
            )
            self.assertFalse(
                any(
                    record["predicate"] in ("valid_task_ref", "valid_stack_pointer")
                    for record in obligations
                )
            )
            self.assertNotIn(
                "auto_candidate", data["summary"]["obligation_categories"]
            )
            self.assertFalse(
                any(record["proof_provider"] == "builtin_candidate" for record in obligations)
            )
            self.assertFalse(obligations)
            self.assertFalse(
                any(record["proof_provider"] == "derived_candidate" for record in obligations)
            )
            self.assertFalse(
                any(
                    record["predicate"]
                    in ("linear_map_area_reserved", "fixmap_adjacent_to_linear_map")
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "linear_map_area_reserved"
                    and record["proof_class"] == "address_layout"
                    and record["proof_provider"] == "config_address_layout"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "fixmap_adjacent_to_linear_map"
                    and record["proof_class"] == "address_layout"
                    and record["proof_provider"] == "config_address_layout"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "has_slot"
                    and record["proof_class"] == "config_structure"
                    and record["proof_provider"] == "builtin"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "readonly"
                    and record["proof_class"] == "object_attribute"
                    and record["proof_provider"] == "builtin"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"]
                    == "KernelInitTask.Transition::SetRuntimeState(state: TaskRuntimeState::Running)"
                    and record["proof_class"] == "type_process_commit"
                    and record["proof_provider"] == "within_context"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"]
                    == (
                        "let selected_rq: RunQueueRef <- "
                        "Scheduler.Action::SelectRunQueue(task_ref: KernelInitTaskRef)"
                    )
                    and record["display_expression"]
                    == "let selected_rq: RunQueueRef <- Scheduler.Action::SelectRunQueue(KernelInitTaskRef)"
                    and record["proof_class"] == "action_result_binding"
                    and record["proof_provider"] == "within_context"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"]
                    == "selected_rq.Transition::EnqueueTask(task_ref: KernelInitTaskRef)"
                    and record["display_expression"]
                    == "selected_rq.Transition::EnqueueTask(KernelInitTaskRef)"
                    and record["proof_class"] == "type_process_commit"
                    and record["proof_provider"] == "within_context"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"]
                    == "runqueue_contains_task(BootRunQueue, KernelInitTaskRef)"
                    and record["proof_class"] == "type_process_ensures"
                    and record["proof_provider"] == "within_context"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"]
                    == "task_runtime_state_is(KernelInitTask, TaskRuntimeState::Running)"
                    and record["proof_class"] == "type_process_ensures"
                    and record["proof_provider"] == "within_context"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "task_state_running(KernelInitTask)"
                    and record["proof_class"] == "derived_alias"
                    and record["proof_provider"] == "type_process_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"]
                    == "scheduler_select_runqueue_returns(Scheduler, KernelInitTaskRef, BootRunQueueRef)"
                    and record["proof_class"] == "type_process_ensures"
                    and record["proof_provider"] == "within_context"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "within EnqueueSelectedRunQueueContext"
                    and record["proof_class"] == "exclusive_context"
                    and record["proof_provider"] == "guard"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "BootRunQueueLock.Transition::LockIrqSave"
                    and record["proof_class"] == "context_guard_transition"
                    and record["proof_provider"] == "guard"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"]
                    == "let next: TaskRef <- CurrentRunQueueRef.Action::PickNextTask(prev_ref: CurrentTaskRef)"
                    and record["display_expression"]
                    == "let next: TaskRef <- CurrentRunQueueRef.Action::PickNextTask(CurrentTaskRef)"
                    and record["proof_class"] == "action_result_binding"
                    and record["proof_provider"] == "within_context"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"]
                    == "Scheduler.Action::SwitchTo(CurrentTaskRef, next)"
                    and record["proof_class"] == "action_commit"
                    and record["proof_provider"] == "within_context"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"]
                    == "CurrentTaskRef.Action::SaveCoreContext"
                    and record["proof_class"] == "type_process_commit"
                    and record["proof_provider"] == "within_context"
                    and "Scheduler.Action::SwitchTo" in (record["process_parent"] or "")
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"]
                    == "CurrentTaskRef.Action::SetCurrent(task: KernelInitTask)"
                    and record["proof_class"] == "type_process_commit"
                    and record["proof_provider"] == "within_context"
                    and "Scheduler.Action::SwitchTo" in (record["process_parent"] or "")
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "task_ref_targets(CurrentTaskRef, KernelInitTask)"
                    and record["proof_class"] == "type_process_ensures"
                    and record["proof_provider"] == "within_context"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"]
                    == "next.Action::RestoreCoreContext"
                    and record["proof_class"] == "type_process_commit"
                    and record["proof_provider"] == "within_context"
                    and "Scheduler.Action::SwitchTo" in (record["process_parent"] or "")
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"]
                    == "task_thread_context_core_restored(KernelInitTask.thread_context)"
                    and record["proof_class"] == "type_process_ensures"
                    and record["proof_provider"] == "within_context"
                    for record in proved
                )
            )
            self.assertFalse(
                any(record["predicate"] == "disjoint" for record in obligations)
            )
            self.assertTrue(
                any(
                    record["predicate"] == "disjoint"
                    and record["proof_class"] == "platform_memory_layout"
                    and record["proof_provider"] == "fdt_memory_layout"
                    for record in proved
                )
            )
            self.assertFalse(
                any(
                    record["predicate"]
                    in ("kernel_fpu_disabled", "kernel_vector_disabled")
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "kernel_fpu_disabled"
                    and record["proof_class"] == "riscv_status_register"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "kernel_vector_disabled"
                    and record["proof_class"] == "riscv_status_register"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"]
                    == "firmware_dtb_blob_accessible_at_kernel_entry"
                    and record["proof_class"] == "firmware_entry_state"
                    and record["proof_provider"] == "prior_derivation_facts"
                    and record["expression"]
                    == "firmware_dtb_blob_accessible_at_kernel_entry(BootArgs.dtb_pa)"
                    for record in proved
                )
            )
            self.assertFalse(
                any(
                    record["expression"]
                    in ("boot_hartid == Riscv64.a0", "dtb_pa == Riscv64.a1")
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "boot_hartid == Riscv64.a0"
                    and record["proof_class"] == "boot_arguments"
                    and record["proof_provider"] == "riscv_boot_protocol"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "dtb_pa == Riscv64.a1"
                    and record["proof_class"] == "boot_arguments"
                    and record["proof_provider"] == "riscv_boot_protocol"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "header_range.start == BootArgs.dtb_pa"
                    and record["proof_class"] == "dtb_header_range"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "range.end == BootArgs.dtb_pa + header.total_size"
                    and record["proof_class"] == "dtb_range"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertFalse(
                any(
                    record["expression"] == "fdt_slot == Config.fixmap.fdt"
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "fdt_slot == Config.fixmap.fdt"
                    and record["proof_class"] == "fixmap_layout"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"]
                    == "Riscv64.stvec == virt_addr(EventStream.formal_event_entry, EarlyVm, KernelImageMap)"
                    and record["proof_class"] == "register_effect"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "Riscv64.tp == virt_addr(BootTask.storage, EarlyVm, KernelImageMap)"
                    and record["proof_class"] == "register_effect"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "Riscv64.sp == phys_addr(Lds.init_stack_end - Config.pt_size_on_stack)"
                    and record["proof_class"] == "register_effect"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "Riscv64.satp == satp_of(EarlyVm.pg_dir, Config.satp_mode)"
                    and record["proof_class"] == "register_effect"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "kernel_image_va_window_size >= pmd_size"
                    and record["proof_class"] == "configuration"
                    and record["proof_provider"] == "config_source"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "valid_satp_mode"
                    and record["proof_class"] == "configuration"
                    and record["proof_provider"] == "config_source"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "kernel_end > kernel_start"
                    and record["proof_class"] == "linker_layout"
                    and record["proof_provider"] == "linux_linker_script"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "segments.bss.range == range(Lds.bss_start, Lds.bss_end)"
                    and record["proof_class"] == "linker_layout"
                    and record["proof_provider"] == "linux_linker_script"
                    for record in proved
                )
            )
            self.assertFalse(
                any(
                    record["expression"] == "Lds.init_stack_end - Lds.init_stack_start >= Config.page_size"
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "Lds.init_stack_end - Lds.init_stack_start >= Config.page_size"
                    and record["proof_class"] == "stack_layout"
                    and record["proof_provider"] == "config_and_linker"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "boot_cpu_hartid_ready(BootCPU, BootArgs.boot_hartid)"
                    and record["proof_class"] == "boot_hart_identity"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "valid_dtb_magic"
                    and record["proof_class"] == "boot_input"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "valid_dtb_header"
                    and record["proof_class"] == "boot_input"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertFalse(
                any(record["predicate"] == "valid_hart_id" for record in obligations)
            )
            self.assertTrue(
                any(
                    record["expression"] == "platform_hart_id_valid(BootArgs.boot_hartid)"
                    and record["object"] == "PlatformCpuInfo"
                    and record["proof_class"] == "platform_cpu_description"
                    and record["proof_provider"] == "fdt_cpu_description"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "cpu_group_possible_cpu_boundary_ready(CpuGroup)"
                    and record["proof_class"] == "cpu_topology"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "boot_cpu_online(BootCPU)"
                    and record["proof_class"] == "cpu_state"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertFalse(
                any(
                    record["predicate"]
                    in (
                        "context_is",
                        "interrupt_concurrency_closed",
                        "task_concurrency_closed",
                    )
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "context_is"
                    and record["proof_class"] == "phase_context"
                    and record["proof_provider"] == "prior_derivation_facts"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "interrupt_concurrency_closed"
                    and record["proof_class"] == "system_exclusive_context"
                    and record["proof_provider"] == "prior_derivation_facts"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "task_concurrency_closed"
                    and record["proof_class"] == "system_exclusive_context"
                    and record["proof_provider"] == "prior_derivation_facts"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "sbi_hsm_available"
                    and record["proof_class"] == "sbi_hsm"
                    and record["proof_provider"] == "riscv_sbi_spec"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "primary_hart_sie_clear_at_kernel_entry"
                    and record["proof_class"] == "firmware_entry_state"
                    and record["proof_provider"] == "opensbi_firmware"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "primary_hart_only_at_kernel_entry"
                    and record["proof_class"] == "firmware_entry_state"
                    and record["proof_provider"] == "opensbi_firmware"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "ordered_booting_enabled"
                    and record["proof_class"] == "firmware_boot_policy"
                    and record["proof_provider"] == "opensbi_firmware"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "firmware_dtb_blob_in_ram_at_kernel_entry"
                    and record["proof_class"] == "firmware_entry_state"
                    and record["proof_provider"] == "opensbi_firmware"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"]
                    == "firmware_dtb_blob_complete_at_kernel_entry"
                    and record["proof_class"] == "firmware_entry_state"
                    and record["proof_provider"] == "opensbi_firmware"
                    and record["expression"]
                    == "firmware_dtb_blob_complete_at_kernel_entry(BootArgs.dtb_pa)"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"]
                    == "firmware_dtb_blob_accessible_at_kernel_entry"
                    and record["proof_class"] == "firmware_entry_state"
                    and record["proof_provider"] == "opensbi_firmware"
                    and record["expression"]
                    == "firmware_dtb_blob_accessible_at_kernel_entry(BootArgs.dtb_pa)"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"]
                    == "firmware_dtb_blob_complete_at_kernel_entry"
                    and record["proof_class"] == "firmware_entry_state"
                    and record["proof_provider"] == "prior_derivation_facts"
                    and record["expression"]
                    == "firmware_dtb_blob_complete_at_kernel_entry(BootArgs.dtb_pa)"
                    for record in proved
                )
            )
            self.assertFalse(
                any(
                    record["predicate"] == "slot_contains"
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "slot_contains(fdt_slot, RawDtb)"
                    and record["proof_class"] == "fixmap_slot_content"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "slot_contains(FixMap.fdt_slot, RawDtb)"
                    and record["proof_class"] == "fixmap_slot_content"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "slot_contains(FixMap.fdt_slot, RawDtb)"
                    and record["proof_class"] == "fixmap_slot_content"
                    and record["proof_provider"] == "prior_derivation_facts"
                    for record in proved
                )
            )
            self.assertFalse(
                any(
                    record["predicate"] == "fits_in_kernel_image_map"
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "fits_in_kernel_image_map"
                    and record["proof_class"] == "address_mapping"
                    and record["proof_provider"] == "config_and_linker"
                    for record in proved
                )
            )
            self.assertFalse(
                any(
                    record["predicate"] == "fits_in_fixmap_slot"
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "fits_in_fixmap_slot"
                    and record["proof_class"] == "address_mapping"
                    and record["proof_provider"] == "riscv_fixmap_layout"
                    for record in proved
                )
            )
            self.assertFalse(
                any(
                    record["predicate"]
                    in (
                        "kernel_image_mapping_ready",
                        "fixmap_slot_mapping_ready",
                    )
                    for record in obligations
                )
            )
            self.assertFalse(
                any(
                    record["predicate"] == "kernel_image_accessible"
                    for record in obligations
                )
            )
            self.assertFalse(
                any(
                    record["predicate"] == "fixmap_slot_accessible"
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "kernel_image_accessible"
                    and record["proof_class"] == "address_mapping"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "fixmap_slot_accessible"
                    and record["proof_class"] == "address_mapping"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "valid_virt_addr"
                    and record["proof_class"] == "address_mapping"
                    and record["proof_provider"] == "config_source"
                    for record in proved
                )
            )
            self.assertFalse(
                any(
                    record["predicate"] == "trampoline_mapping_ready"
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "trampoline_mapping_ready"
                    and record["proof_class"] == "address_mapping"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "trampoline_mapping_ready"
                    and record["proof_class"] == "address_mapping"
                    and record["proof_provider"] == "prior_derivation_facts"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "kernel_image_mapping_ready"
                    and record["proof_class"] == "address_mapping"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "fixmap_slot_mapping_ready"
                    and record["proof_class"] == "address_mapping"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertFalse(
                any(
                    record["predicate"] == "valid_trampoline_map"
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "valid_trampoline_map"
                    and record["proof_class"] == "address_mapping"
                    and record["proof_provider"] == "config_and_linker"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "valid_object_storage"
                    and record["proof_class"] == "object_storage"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "valid_function_symbol"
                    and record["proof_class"] == "linker_symbol"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "valid_page_table_storage"
                    and record["proof_class"] == "object_storage"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertFalse(
                any(
                    record["predicate"] == "valid_phys_range_set"
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "valid_phys_range_set"
                    and record["proof_class"] == "platform"
                    and record["proof_provider"] == "fdt_memory_layout"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "valid_task_storage"
                    and record["proof_class"] == "object_storage"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "valid_task_ref"
                    and record["proof_class"] == "object_storage"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "valid_stack_pointer"
                    and record["proof_class"] == "architecture_state"
                    and record["proof_provider"] == "prior_derivation_facts"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"]
                    == "inside(Riscv64.sp, Lds.init_stack_end, Lds.init_stack_start, Lds.init_stack_end)"
                    and record["proof_class"] == "stack_layout"
                    and record["proof_provider"] == "prior_derivation_facts"
                    for record in proved
                )
            )
            self.assertFalse(
                any(
                    record["predicate"] == "soc_early_platform_ready"
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "soc_early_platform_ready"
                    and record["proof_class"] == "platform"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertFalse(
                any(
                    record["predicate"] == "phys_to_virt_transition_completed"
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "valid_segment_set"
                    and record["proof_class"] == "linker_layout"
                    and record["proof_provider"] == "linux_linker_script"
                    for record in proved
                )
            )
            self.assertFalse(
                any(record["predicate"] == "memory_zeroed" for record in obligations)
            )
            self.assertTrue(
                any(
                    record["predicate"] == "memory_zeroed"
                    and record["proof_class"] == "memory_content"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertFalse(
                any(
                    record["predicate"] == "gp_relative_access_ready"
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "phys_to_virt_transition_completed"
                    and record["proof_class"] == "architecture_state"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "gp_relative_access_ready"
                    and record["proof_class"] == "architecture_state"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "swapper_vm_mappings_ready"
                    and record["proof_class"] == "address_mapping"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "memblock_allocator_ready"
                    and record["proof_class"] == "physical_memory_management"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            attrs_providers = {
                record["object"]: (record["proof_class"], record["proof_provider"])
                for record in obligations
                if record["predicate"] == "attrs_accessible"
            }
            self.assertNotIn("BootArgs", attrs_providers)
            self.assertTrue(
                any(
                    record["object"] == "BootArgs"
                    and record["predicate"] == "attrs_accessible"
                    and record["proof_class"] == "boot_arguments"
                    and record["proof_provider"] == "riscv_boot_protocol"
                    for record in proved
                )
            )
            self.assertNotIn("Config", attrs_providers)
            self.assertTrue(
                any(
                    record["object"] == "Config"
                    and record["predicate"] == "attrs_accessible"
                    and record["proof_class"] == "config_attributes"
                    and record["proof_provider"] == "config_source"
                    for record in proved
                )
            )
            self.assertNotIn("FixMap", attrs_providers)
            self.assertTrue(
                any(
                    record["object"] == "FixMap"
                    and record["predicate"] == "attrs_accessible"
                    and record["proof_class"] == "fixmap_layout"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertNotIn("Lds", attrs_providers)
            self.assertTrue(
                any(
                    record["object"] == "Lds"
                    and record["predicate"] == "attrs_accessible"
                    and record["proof_class"] == "linker_layout"
                    and record["proof_provider"] == "linux_linker_script"
                    for record in proved
                )
            )
            self.assertNotIn("PhysicalMemory", attrs_providers)
            self.assertTrue(
                any(
                    record["object"] == "PhysicalMemory"
                    and record["predicate"] == "attrs_accessible"
                    and record["proof_class"] == "platform_memory_layout"
                    and record["proof_provider"] == "fdt_memory_layout"
                    for record in proved
                )
            )
            self.assertNotIn("Riscv64", attrs_providers)
            self.assertTrue(
                any(
                    record["object"] == "Riscv64"
                    and record["predicate"] == "attrs_accessible"
                    and record["proof_class"] == "architecture_register_file"
                    and record["proof_provider"] == "riscv_isa_spec"
                    for record in proved
                )
            )
            self.assertNotIn("BootTask", attrs_providers)
            self.assertTrue(
                any(
                    record["object"] == "BootTask"
                    and record["predicate"] == "attrs_accessible"
                    and record["proof_class"] == "static_object_binding"
                    and record["proof_provider"] == "transition_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "no_service"
                    and record["proof_class"] == "state_alias"
                    and record["proof_provider"] == "builtin"
                    for record in proved
                )
            )

    def test_repeated_fork_declarations_create_independent_tasks_flows_and_refs(self) -> None:
        processes = """
            Action::Fork {
                drives {
                    declare child of Task;
                    declare child_ref of TaskRef;
                    declare flow of UserAppFlow;
                    child_ref.Action::SetCurrent(task: child);
                    child.Transition::Preset(initial_flow: flow);
                    flow.Transition::Preset(owner_task: child);
                    flow.Transition::Setup;
                    child.Transition::Setup;
                    child.Transition::Enable;
                    flow.Transition::Enable;
                }
            }

            Action::Run {
                drives {
                    self.Action::Fork;
                    self.Action::Fork;
                }
            }
        """
        with tempfile.TemporaryDirectory() as tmp:
            data = self._derive_source(Path(tmp), self._dynamic_fixture(processes))

        self.assertTrue(data["summary"]["ok"])
        instances = data["runtime_instances"]
        tasks = [item for item in instances if item["declared_type"] == "Task"]
        flows = [item for item in instances if item["declared_type"] == "UserAppFlow"]
        refs = [item for item in instances if item["declared_type"] == "TaskRef"]
        self.assertEqual(len(tasks), 2)
        self.assertEqual(len(flows), 2)
        self.assertEqual(len(refs), 2)
        self.assertEqual({item["occurrence"] for item in tasks}, {1, 2})
        self.assertEqual({item["state"] for item in tasks}, {"Online"})
        self.assertEqual({item["state"] for item in flows}, {"Online"})
        self.assertEqual(
            {item["ref_target"] for item in refs},
            {item["runtime_instance_id"] for item in tasks},
        )
        self.assertEqual(
            {item["owner_task"] for item in flows},
            {item["runtime_instance_id"] for item in tasks},
        )
        self.assertTrue(all(len(item["owned_flows"]) == 1 for item in tasks))

    def test_static_and_runtime_instances_share_type_lifecycle_with_self_substitution(self) -> None:
        source = """
            type Token {
                processes {
                    Action::Touch {
                        ensures { token_touched(self); }
                    }
                }
            }

            type Gate {
            }

            type Carrier {
                initial_state: State::Base;
                state State::Base {
                    transitions {
                        on Transition::Preset(token: Token) -> State::Prepared {
                            depends_on { token.state == State::Ready; }
                            drives {
                                token.Action::Touch;
                                GateObject.Action::Open;
                            }
                            ensures { carrier_prepared(self); }
                        }
                    }
                }
                state State::Prepared {
                    invariant { carrier_prepared(self); }
                    transitions {
                        on Transition::Setup -> State::Ready {
                            depends_on { gate_open(GateObject); }
                            ensures { carrier_ready(self); }
                        }
                    }
                }
                state State::Ready {
                    invariant { carrier_ready(self); }
                }
            }

            type Factory {
                processes {
                    Action::Run {
                        drives {
                            StaticCarrier.Transition::Preset(token: TokenObject);
                            StaticCarrier.Transition::Setup;
                            declare child of Carrier;
                            child.Transition::Preset(token: TokenObject);
                            child.Transition::Setup;
                        }
                    }
                }
            }

            object TokenObject: Token {
                initial_state: State::Ready;
                state State::Ready {}
            }

            object GateObject: Gate {
                initial_state: State::Ready;
                state State::Ready {
                    actions {
                        Action::Open {
                            ensures { gate_open(self); }
                        }
                    }
                }
            }

            object StaticCarrier: Carrier {
            }

            object ComputerProject: Factory {
                initial_state: State::Base;
                state State::Base {
                    transitions {
                        on Transition::Preset -> State::Prepared {
                            drives { ComputerProject.Action::Run; }
                        }
                    }
                }
                state State::Prepared {}
            }
        """
        with tempfile.TemporaryDirectory() as tmp:
            data = self._derive_source(Path(tmp), source)

        self.assertTrue(data["summary"]["ok"])
        child = data["runtime_instances"][0]
        self.assertEqual(data["states"]["StaticCarrier"], "Ready")
        self.assertEqual(child["state"], "Ready")
        invariant_expressions = {
            record["expression"]
            for record in data["records"]
            if record["source_kind"] == "invariant"
        }
        self.assertIn("carrier_prepared(StaticCarrier)", invariant_expressions)
        self.assertIn("carrier_ready(StaticCarrier)", invariant_expressions)
        self.assertIn(
            f"carrier_prepared({child['runtime_instance_id']})",
            invariant_expressions,
        )
        self.assertIn(
            f"carrier_ready({child['runtime_instance_id']})",
            invariant_expressions,
        )
        proved_expressions = {
            record["expression"]
            for record in data["records"]
            if record["status"] == "proved"
        }
        self.assertIn("token_touched(TokenObject)", proved_expressions)
        self.assertIn("gate_open(GateObject)", proved_expressions)

    def test_runtime_type_lifecycle_rejects_transition_from_wrong_state(self) -> None:
        source = """
            type Carrier {
                initial_state: State::Base;
                state State::Base {
                    transitions { on Transition::Preset -> State::Prepared {} }
                }
                state State::Prepared {
                    transitions { on Transition::Setup -> State::Ready {} }
                }
                state State::Ready {}
            }

            type Factory {
                processes {
                    Action::Run {
                        drives {
                            declare child of Carrier;
                            child.Transition::Setup;
                        }
                    }
                }
            }

            object ComputerProject: Factory {
                initial_state: State::Base;
                state State::Base {
                    transitions {
                        on Transition::Preset -> State::Prepared {
                            drives { ComputerProject.Action::Run; }
                        }
                    }
                }
                state State::Prepared {}
            }
        """
        with tempfile.TemporaryDirectory() as tmp:
            data = self._derive_source(Path(tmp), source)

        self.assertFalse(data["summary"]["ok"])
        self.assertTrue(
            any(
                "illegal runtime lifecycle transition" in record["message"]
                for record in data["records"]
            )
        )

    def test_repeated_exec_keeps_task_identity_and_creates_fresh_flows(self) -> None:
        processes = """
            Action::Exec(task: Task) {
                drives {
                    declare exec_flow of UserAppFlow;
                    exec_flow.Transition::Preset(owner_task: task);
                    exec_flow.Transition::Setup;
                    exec_flow.Transition::Enable;
                    exec_flow.Transition::Disable;
                    exec_flow.Transition::Cleanup;
                }
            }

            Action::Run {
                drives {
                    declare task of Task;
                    self.Action::Exec(task: task);
                    self.Action::Exec(task: task);
                }
            }
        """
        with tempfile.TemporaryDirectory() as tmp:
            data = self._derive_source(Path(tmp), self._dynamic_fixture(processes))

        self.assertTrue(data["summary"]["ok"])
        tasks = [
            item for item in data["runtime_instances"] if item["declared_type"] == "Task"
        ]
        flows = [
            item
            for item in data["runtime_instances"]
            if item["declared_type"] == "UserAppFlow"
        ]
        self.assertEqual(len(tasks), 1)
        self.assertEqual(len(flows), 2)
        self.assertEqual({item["occurrence"] for item in flows}, {1, 2})
        self.assertEqual({item["state"] for item in flows}, {"Destroyed"})
        self.assertEqual(
            {item["owner_task"] for item in flows},
            {tasks[0]["runtime_instance_id"]},
        )
        self.assertEqual(
            set(tasks[0]["owned_flows"]),
            {item["runtime_instance_id"] for item in flows},
        )

    def test_runtime_relations_reject_flow_sharing_and_early_task_cleanup(self) -> None:
        cases = {
            "shared-flow": (
                """
                    Action::Run {
                        drives {
                            declare first of Task;
                            declare second of Task;
                            declare flow of UserAppFlow;
                            first.Transition::Preset(initial_flow: flow);
                            second.Transition::Preset(initial_flow: flow);
                        }
                    }
                """,
                "Flow already belongs to a different Task",
            ),
            "early-cleanup": (
                """
                    Action::Run {
                        drives {
                            declare task of Task;
                            declare flow of UserAppFlow;
                            task.Transition::Preset(initial_flow: flow);
                            flow.Transition::Preset(owner_task: task);
                            flow.Transition::Setup;
                            task.Transition::Setup;
                            task.Transition::Enable;
                            flow.Transition::Enable;
                            flow.Transition::Disable;
                            task.Transition::Disable;
                            task.Transition::Cleanup;
                        }
                    }
                """,
                "Task cannot be Destroyed before every owned Flow is Destroyed",
            ),
            "illegal-lifecycle": (
                """
                    Action::Run {
                        drives {
                            declare flow of UserAppFlow;
                            flow.Transition::Enable;
                        }
                    }
                """,
                "illegal runtime lifecycle transition",
            ),
        }
        for name, (processes, expected) in cases.items():
            with self.subTest(name=name), tempfile.TemporaryDirectory() as tmp:
                data = self._derive_source(Path(tmp), self._dynamic_fixture(processes))

                self.assertFalse(data["summary"]["ok"])
                self.assertGreater(data["summary"]["contradiction"], 0)
                self.assertTrue(
                    any(expected in record["message"] for record in data["records"])
                )
                self.assertGreater(len(data["runtime_instances"]), 0)

    def test_runtime_identity_is_stable_across_unrelated_line_movement(self) -> None:
        process_template = """
            Action::Run {{
                {padding}
                drives {{
                    declare flow of UserAppFlow;
                    flow.Transition::Preset(owner_task: ComputerProject);
                }}
            }}
        """
        identities = []
        for padding in ("", "/* unrelated diagnostic comment */\n\n"):
            with tempfile.TemporaryDirectory() as tmp:
                data = self._derive_source(
                    Path(tmp),
                    self._dynamic_fixture(process_template.format(padding=padding)),
                )
                identities.append(data["runtime_instances"][0]["runtime_instance_id"])

        self.assertEqual(identities[0], identities[1])

    def test_declaration_occurrence_stress_keeps_128_instances_distinct(self) -> None:
        calls = "\n".join("self.Action::Make;" for _ in range(128))
        processes = f"""
            Action::Make {{
                drives {{
                    declare flow of UserAppFlow;
                    flow.Transition::Preset(owner_task: ComputerProject);
                }}
            }}
            Action::Run {{
                drives {{
                    {calls}
                }}
            }}
        """
        with tempfile.TemporaryDirectory() as tmp:
            data = self._derive_source(Path(tmp), self._dynamic_fixture(processes))

        instances = data["runtime_instances"]
        self.assertTrue(data["summary"]["ok"])
        self.assertEqual(len(instances), 128)
        self.assertEqual(len({item["runtime_instance_id"] for item in instances}), 128)
        self.assertEqual({item["state"] for item in instances}, {"Prepared"})
        self.assertEqual({item["occurrence"] for item in instances}, set(range(1, 129)))

    def test_invalid_model_schema_returns_usage_error_code(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            model = Path(tmp) / "bad.model.json"
            derive = Path(tmp) / "bad.derive.json"
            model.write_text('{"schema": "wrong", "version": 1}\n', encoding="utf-8")

            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exit_code = derive_main([str(model), "-o", str(derive)])

            self.assertEqual(exit_code, 2)
            self.assertIn("error: invalid model JSON", stderr.getvalue())

    def test_derive_tool_does_not_import_pyveri(self) -> None:
        source_root = Path(__file__).resolve().parents[1] / "src" / "derive_tool"

        for path in source_root.rglob("*.py"):
            text = path.read_text(encoding="utf-8")
            self.assertNotIn("from pyveri", text, str(path))
            self.assertNotIn("import pyveri", text, str(path))


if __name__ == "__main__":
    unittest.main()
