from __future__ import annotations

import contextlib
import io
import sys
import tempfile
import unittest
from pathlib import Path

TOOLS_ROOT = Path(__file__).resolve().parents[2]
DERIVE_SRC = TOOLS_ROOT / "derive" / "src"
if str(DERIVE_SRC) not in sys.path:
    sys.path.insert(0, str(DERIVE_SRC))

from common import VIEW_SCHEMA, VIEW_VERSION, read_json
from derive_tool.__main__ import main as derive_main
from model_tool.__main__ import main as model_main
from parse_tool.__main__ import main as parse_main
from view_tool.builder import build_trace_view
from view_tool.__main__ import main as view_main


class ViewToolTests(unittest.TestCase):
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

    def _build_derive_json(self, tmp: str) -> Path:
        model = self._build_model_json(tmp)
        derive = Path(tmp) / "model-main.derive.json"
        self.assertEqual(derive_main([str(model), "-o", str(derive)]), 0)
        return derive

    def test_object_view_writes_view_json(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            model = self._build_model_json(tmp)
            output = Path(tmp) / "object.view.json"

            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exit_code = view_main([str(model), "object", "-o", str(output)])

            self.assertEqual(exit_code, 0)
            data = read_json(output)
            self.assertEqual(data["schema"], VIEW_SCHEMA)
            self.assertEqual(data["version"], VIEW_VERSION)
            self.assertEqual(data["view"], "object")
            self.assertIn("ComputerProject", data["nodes"])
            self.assertTrue(
                any(
                    edge["source"] == "ComputerProject"
                    and edge["target"] == "KernelProject"
                    and edge["kind"] == "parent"
                    for edge in data["edges"]
                )
            )

    def test_drives_view_contains_event_edges(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            model = self._build_model_json(tmp)
            output = Path(tmp) / "drives.view.json"
            self.assertEqual(view_main([str(model), "drives", "-o", str(output)]), 0)

            data = read_json(output)
            self.assertEqual(data["view"], "drives")
            self.assertEqual(data["rankdir"], "LR")
            self.assertIn("ComputerProject.Preset", data["nodes"])
            self.assertTrue(
                any(
                    edge["source"] == "ComputerProject.Preset"
                    and edge["target"] == "ComputerProject.Setup"
                    and edge["kind"] == "emits"
                    for edge in data["edges"]
                )
            )
            self.assertTrue(
                any(
                    edge["source"] == "ComputerProject.Enable"
                    and edge["target"] == "KernelProject.Preset"
                    and edge["kind"] == "drives"
                    for edge in data["edges"]
                )
            )

    def test_boundary_view_contains_stable_inventory_and_source_locations(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            model = self._build_model_json(tmp)
            output = Path(tmp) / "boundaries.view.json"
            self.assertEqual(view_main([str(model), "boundaries", "-o", str(output)]), 0)

            data = read_json(output)
            self.assertEqual(data["view"], "boundaries")
            summary = data["metadata"]["summary"]
            inventory = data["metadata"]["inventory"]
            self.assertEqual(summary["legacy_boundaries"], 0)
            self.assertEqual(
                summary["deferred"],
                sum(item["status"] == "deferred" for item in inventory),
            )
            self.assertEqual(
                summary["trimmed"],
                sum(item["status"] == "trimmed" for item in inventory),
            )
            clone = next(item for item in inventory if item["id"] == "user_clone.001")
            self.assertEqual(clone["category"], "Feature")
            self.assertEqual(
                clone["owner"],
                "UserCloneDeferredBoundaries.Transition::Setup",
            )
            self.assertTrue(clone["source_file"].endswith("objects/user_boot.spec"))
            self.assertGreater(clone["source_line"], 0)
            self.assertIn("user_clone.001", data["nodes"])

    def test_timeline_view_contains_rows_metadata(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            model = self._build_model_json(tmp)
            output = Path(tmp) / "timeline.view.json"
            self.assertEqual(view_main([str(model), "timeline", "-o", str(output)]), 0)

            data = read_json(output)
            self.assertEqual(data["view"], "timeline")
            self.assertEqual(data["graph_format"], "svg")
            rows = data["metadata"]["timeline_rows"]
            self.assertFalse(any(row["phase"] == "PreparePhase" for row in rows))
            self.assertTrue(any(row["phase"] == "BootPhase" for row in rows))
            self.assertTrue(any(row["phase"] == "InterruptPhase" for row in rows))
            self.assertTrue(any(row["phase"] == "PayloadPhase" for row in rows))

    def test_trace_view_contains_cell_layout_metadata(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            derive = self._build_derive_json(tmp)
            output = Path(tmp) / "trace.view.json"
            self.assertEqual(view_main([str(derive), "trace", "-o", str(output)]), 0)

            data = read_json(output)
            self.assertEqual(data["view"], "trace")
            metadata = data["metadata"]
            self.assertTrue(
                any(column["kind"] == "gap" for column in metadata["trace_columns"])
            )
            self.assertTrue(
                any(column["kind"] == "phase" for column in metadata["trace_columns"])
            )
            self.assertTrue(
                any(column["kind"] == "object" for column in metadata["trace_columns"])
            )
            self.assertTrue(any(row["kind"] == "gap" for row in metadata["trace_rows"]))
            self.assertTrue(
                any(
                    cell["kind"] == "transition_span"
                    and cell["label"] == "ComputerProject.Transition::Preset"
                    for cell in metadata["trace_cells"]
                )
            )
            self.assertTrue(
                any(arrow["kind"] == "emits" for arrow in metadata["trace_arrows"])
            )
            self.assertTrue(
                any(arrow["kind"] == "drives" for arrow in metadata["trace_arrows"])
            )
            self.assertTrue(
                any(
                    arrow["kind"] == "depends_on"
                    for arrow in metadata["trace_arrows"]
                )
            )
            self.assertTrue(
                any(
                    cell["kind"] == "verified_state"
                    and cell["label"] == "Riscv64.State::Online"
                    for cell in metadata["trace_cells"]
                )
            )
            riscv64_cell = next(
                cell
                for cell in metadata["trace_cells"]
                if cell["kind"] == "verified_state"
                and cell["label"] == "Riscv64.State::Online"
            )
            root_stream_cell = next(
                cell
                for cell in metadata["trace_cells"]
                if cell["kind"] == "state"
                and cell["label"] == "RootStream.State::Base"
            )
            self.assertEqual(riscv64_cell["column"], root_stream_cell["column"])
            self.assertFalse(
                any(
                    cell["kind"] == "transition_span"
                    and cell["label"] == "PreparePhase.Transition::Enable"
                    for cell in metadata["trace_cells"]
                )
            )
            self.assertFalse(
                any(
                    cell["label"].startswith("PreparePhase.")
                    for cell in metadata["trace_cells"]
                )
            )

            def trace_cell(kind: str, label: str) -> dict[str, object]:
                return next(
                    cell
                    for cell in metadata["trace_cells"]
                    if cell["kind"] == kind and cell["label"] == label
                )

            def assert_emit_at_transition_end(
                transition_cell: dict[str, object], emit_cell: dict[str, object]
            ) -> None:
                transition_end = transition_cell["row"] + transition_cell["row_span"]
                self.assertEqual(emit_cell["row"], transition_end - 2)

            computer_preset_cell = trace_cell(
                "transition_span", "ComputerProject.Transition::Preset"
            )
            computer_setup_emit_cell = trace_cell(
                "emit_event", "ComputerProject.Transition::Setup"
            )
            computer_setup_cell = trace_cell(
                "transition_span", "ComputerProject.Transition::Setup"
            )
            computer_enable_emit_cell = trace_cell(
                "emit_event", "ComputerProject.Transition::Enable"
            )
            computer_enable_cell = trace_cell(
                "transition_span", "ComputerProject.Transition::Enable"
            )
            kernel_project_preset_cell = trace_cell(
                "transition_span", "KernelProject.Transition::Preset"
            )
            kernel_project_setup_emit_cell = trace_cell(
                "emit_event", "KernelProject.Transition::Setup"
            )
            kernel_project_setup_cell = trace_cell(
                "transition_span", "KernelProject.Transition::Setup"
            )
            kernel_project_enable_emit_cell = trace_cell(
                "emit_event", "KernelProject.Transition::Enable"
            )
            kernel_project_enable_cell = trace_cell(
                "transition_span", "KernelProject.Transition::Enable"
            )
            kernel_preset_cell = trace_cell(
                "transition_span", "Kernel.Transition::Preset"
            )
            boot_setup_cell = trace_cell(
                "transition_span", "BootPhase.Transition::Setup"
            )
            kernel_setup_emit_cell = trace_cell(
                "emit_event", "Kernel.Transition::Setup"
            )
            kernel_setup_cell = trace_cell(
                "transition_span", "Kernel.Transition::Setup"
            )
            interrupt_setup_cell = trace_cell(
                "transition_span", "InterruptPhase.Transition::Setup"
            )
            kernel_enable_emit_cell = trace_cell(
                "emit_event", "Kernel.Transition::Enable"
            )
            entry_prelude_setup_cell = trace_cell(
                "transition_span", "EntryPreludePhase.Transition::Setup"
            )
            self.assertEqual(
                computer_setup_emit_cell["column"], computer_preset_cell["column"]
            )
            self.assertEqual(computer_setup_cell["column"], computer_preset_cell["column"])
            self.assertEqual(
                computer_enable_emit_cell["column"], computer_preset_cell["column"]
            )
            self.assertEqual(
                computer_enable_cell["column"], computer_preset_cell["column"]
            )
            assert_emit_at_transition_end(
                computer_preset_cell, computer_setup_emit_cell
            )
            assert_emit_at_transition_end(
                computer_setup_cell, computer_enable_emit_cell
            )
            for phase_cell in (
                computer_preset_cell,
                computer_setup_cell,
                computer_enable_cell,
                kernel_project_preset_cell,
                kernel_project_setup_cell,
                kernel_project_enable_cell,
                kernel_preset_cell,
                kernel_setup_cell,
            ):
                self.assertGreaterEqual(phase_cell["row_span"], 24)
            self.assertEqual(
                kernel_project_preset_cell["column"], computer_enable_cell["column"] + 1
            )
            self.assertEqual(
                kernel_project_setup_emit_cell["column"],
                kernel_project_preset_cell["column"],
            )
            self.assertEqual(
                kernel_project_setup_cell["column"], kernel_project_preset_cell["column"]
            )
            self.assertEqual(
                kernel_project_enable_emit_cell["column"],
                kernel_project_preset_cell["column"],
            )
            self.assertEqual(
                kernel_project_enable_cell["column"], kernel_project_preset_cell["column"]
            )
            assert_emit_at_transition_end(
                kernel_project_preset_cell, kernel_project_setup_emit_cell
            )
            assert_emit_at_transition_end(
                kernel_project_setup_cell, kernel_project_enable_emit_cell
            )
            self.assertEqual(
                kernel_preset_cell["column"], kernel_project_enable_cell["column"] + 1
            )
            self.assertEqual(boot_setup_cell["column"], kernel_preset_cell["column"] + 1)
            self.assertEqual(kernel_setup_emit_cell["column"], kernel_preset_cell["column"])
            self.assertGreater(
                kernel_setup_cell["row"],
                kernel_preset_cell["row"] + kernel_preset_cell["row_span"],
            )
            self.assertEqual(kernel_setup_cell["column"], kernel_preset_cell["column"])
            self.assertEqual(interrupt_setup_cell["column"], kernel_setup_cell["column"] + 1)
            self.assertEqual(kernel_enable_emit_cell["column"], kernel_setup_cell["column"])
            assert_emit_at_transition_end(kernel_preset_cell, kernel_setup_emit_cell)
            assert_emit_at_transition_end(kernel_setup_cell, kernel_enable_emit_cell)
            self.assertEqual(entry_prelude_setup_cell["column"], boot_setup_cell["column"])
            self.assertTrue(
                any(
                    row.get("group_role") == "body_start"
                    and row.get("label") == "RootStream.Transition::Preset.body.start"
                    for row in metadata["trace_rows"]
                )
            )

    def test_trace_view_rejects_negative_action_depth(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            derive = self._build_derive_json(tmp)
            output = Path(tmp) / "trace.view.json"
            stderr = io.StringIO()

            with contextlib.redirect_stderr(stderr), self.assertRaises(SystemExit) as caught:
                view_main(
                    [
                        str(derive),
                        "trace",
                        "--trace-action-depth",
                        "-1",
                        "-o",
                        str(output),
                    ]
                )

            self.assertEqual(caught.exception.code, 2)
            self.assertIn("non-negative integer", stderr.getvalue())

    def test_trace_view_keeps_phase_with_ordinary_action(self) -> None:
        view = build_trace_view(
            {
                "trace": [
                    {
                        "object": "UpMultitaskPhase",
                        "transition": "Setup",
                        "source_state": "Base",
                        "target_state": "Ready",
                        "children": [
                            {
                                "object": "BootInitScheduleHandoffPhase",
                                "transition": "Setup",
                                "source_state": "Base",
                                "target_state": "Ready",
                                "children": [],
                            }
                        ],
                    }
                ],
                "records": [
                    {
                        "status": "proved",
                        "object": "BootInitScheduleHandoffPhase",
                        "transition": "Setup",
                        "source_kind": "drives",
                        "proof_class": "action_commit",
                        "proof_provider": "action_drive",
                        "expression": "Scheduler.Action::Schedule",
                    }
                ],
            }
        )

        cells = view.metadata["trace_cells"]
        self.assertTrue(
            any(
                cell.kind == "transition_span"
                and cell.label
                == "BootInitScheduleHandoffPhase.Transition::Setup"
                for cell in cells
            )
        )
        self.assertTrue(
            any(
                cell.kind == "action"
                and cell.label == "Scheduler.Action::Schedule"
                for cell in cells
            )
        )

    def test_trace_view_places_within_context_actions(self) -> None:
        view = build_trace_view(
            {
                "trace": [
                    {
                        "object": "KernelInitTask",
                        "transition": "Enable",
                        "source_state": "Created",
                        "target_state": "Runnable",
                        "children": [],
                    }
                ],
                "model": {
                    "exclusive_contexts": {
                        "WakeUpNewTaskContext": {
                            "lock_ref": "KernelInitTaskPiLock",
                            "guard": {
                                "lock_ref": "KernelInitTaskPiLock",
                                "entered_by": [
                                    {"body": "KernelInitTaskPiLock.Transition::LockIrqSave;"}
                                ],
                                "exited_by": [
                                    {
                                        "body": "KernelInitTaskPiLock.Transition::UnlockIrqRestore;"
                                    }
                                ],
                            },
                        },
                        "EnqueueSelectedRunQueueContext": {
                            "lock_ref": "BootRunQueueLock",
                            "guard": {
                                "lock_ref": "BootRunQueueLock",
                                "entered_by": [
                                    {"body": "BootRunQueueLock.Transition::LockIrqSave;"}
                                ],
                                "exited_by": [
                                    {"body": "BootRunQueueLock.Transition::UnlockIrqRestore;"}
                                ],
                            },
                        },
                    }
                },
                "records": [
                    {
                        "object": "KernelInitTask",
                        "transition": "Enable",
                        "source_kind": "within",
                        "proof_class": "exclusive_context",
                        "expression": "within WakeUpNewTaskContext",
                    },
                    {
                        "object": "KernelInitTask",
                        "transition": "Enable",
                        "proof_class": "type_process_commit",
                        "proof_provider": "within_context",
                        "expression": "KernelInitTask.Transition::SetRuntimeState(state: TaskRuntimeState::Running)",
                        "display_expression": "KernelInitTask.Transition::SetRuntimeState(TaskRuntimeState::Running)",
                    },
                    {
                        "object": "KernelInitTask",
                        "transition": "Enable",
                        "proof_class": "action_result_binding",
                        "proof_provider": "within_context",
                        "expression": "let selected_rq: RunQueueRef <- Scheduler.Action::SelectRunQueue(task_ref: KernelInitTaskRef)",
                        "display_expression": "let selected_rq: RunQueueRef <- Scheduler.Action::SelectRunQueue(KernelInitTaskRef)",
                    },
                    {
                        "object": "KernelInitTask",
                        "transition": "Enable",
                        "source_kind": "within",
                        "proof_class": "exclusive_context",
                        "expression": "within EnqueueSelectedRunQueueContext",
                    },
                    {
                        "object": "KernelInitTask",
                        "transition": "Enable",
                        "proof_class": "type_process_commit",
                        "proof_provider": "within_context",
                        "expression": "selected_rq.Transition::EnqueueTask(task_ref: KernelInitTaskRef)",
                        "display_expression": "selected_rq.Transition::EnqueueTask(KernelInitTaskRef)",
                    },
                    {
                        "object": "KernelInitTask",
                        "transition": "Enable",
                        "source_kind": "within",
                        "proof_class": "exclusive_context",
                        "expression": "within EnqueueSelectedRunQueueContext exited",
                    },
                    {
                        "object": "KernelInitTask",
                        "transition": "Enable",
                        "source_kind": "within",
                        "proof_class": "exclusive_context",
                        "expression": "within WakeUpNewTaskContext exited",
                    },
                ],
            }
        )

        metadata = view.metadata
        cells = metadata["trace_cells"]
        columns = {
            column["index"]: column
            for column in metadata["trace_columns"]
            if isinstance(column.get("index"), int)
        }
        rows = metadata["trace_rows"]
        arrows = metadata["trace_arrows"]
        event_cell = next(cell for cell in cells if cell.kind == "transition_span")
        context_cell = next(
            cell
            for cell in cells
            if cell.kind == "context_span"
            and "WakeUpNewTaskContext" in cell.label
        )
        enqueue_context_cell = next(
            cell
            for cell in cells
            if cell.kind == "context_span"
            and "EnqueueSelectedRunQueueContext" in cell.label
        )
        self.assertGreater(context_cell.column, event_cell.column)
        self.assertEqual(columns[context_cell.column]["kind"], "object")
        self.assertEqual(context_cell.column_span, 2)
        self.assertGreater(context_cell.row_span, enqueue_context_cell.row_span)
        self.assertEqual(enqueue_context_cell.column, context_cell.column)
        self.assertEqual(enqueue_context_cell.column_span, context_cell.column_span)
        self.assertGreaterEqual(enqueue_context_cell.row, context_cell.row)
        self.assertLessEqual(
            enqueue_context_cell.row + enqueue_context_cell.row_span,
            context_cell.row + context_cell.row_span,
        )
        self.assertIn("WakeUpNewTaskContext", context_cell.label)
        self.assertIn("lock=KernelInitTaskPiLock", context_cell.label)
        self.assertIn(
            "enter=KernelInitTaskPiLock.Transition::LockIrqSave", context_cell.label
        )
        self.assertIn(
            "exit=KernelInitTaskPiLock.Transition::UnlockIrqRestore", context_cell.label
        )
        self.assertIn("lock=BootRunQueueLock", enqueue_context_cell.label)
        action_cells = [
            cell
            for cell in cells
            if cell.kind == "context_action"
            and cell.id.startswith(context_cell.id)
        ]
        enqueue_action_cells = [
            cell
            for cell in cells
            if cell.kind == "context_action"
            and cell.id.startswith(enqueue_context_cell.id)
        ]
        self.assertTrue(
            all(
                cell.column == context_cell.column and cell.column_span == 2
                for cell in action_cells
            )
        )
        self.assertTrue(
            all(
                cell.column == enqueue_context_cell.column and cell.column_span == 2
                for cell in enqueue_action_cells
            )
        )
        self.assertEqual(
            [cell.label for cell in sorted(action_cells, key=lambda cell: cell.row)],
            [
                "KernelInitTask.Transition::SetRuntimeState(TaskRuntimeState::Running)",
                "let selected_rq: RunQueueRef <- Scheduler.Action::SelectRunQueue(KernelInitTaskRef)",
            ],
        )
        self.assertEqual(
            [cell.label for cell in enqueue_action_cells],
            ["selected_rq.Transition::EnqueueTask(KernelInitTaskRef)"],
        )
        self.assertTrue(
            all(
                row.get("group_role") == "context_action"
                for row in rows
                if row["kind"] == "context_action"
            )
        )
        self.assertTrue(any(arrow.kind == "within" for arrow in arrows))
        self.assertEqual(
            sum(1 for arrow in arrows if arrow.kind == "context_order"),
            2,
        )

    def test_trace_view_places_phase_within_context_actions(self) -> None:
        view = build_trace_view(
            {
                "trace": [
                    {
                        "object": "BootInitRestInitPhase",
                        "transition": "Preset",
                        "source_state": "Ready",
                        "target_state": "Prepared",
                        "children": [
                            {
                                "object": "BootTask",
                                "transition": "Enable",
                                "source_state": "Prepared",
                                "target_state": "Online",
                                "children": [],
                            }
                        ],
                    }
                ],
                "model": {
                    "exclusive_contexts": {
                        "ScheduleRunQueueContext": {
                            "lock_ref": "BootRunQueueLock",
                            "guard": {
                                "lock_ref": "BootRunQueueLock",
                                "entered_by": [
                                    {"body": "BootRunQueueLock.Transition::LockIrqSave;"}
                                ],
                                "exited_by": [
                                    {"body": "BootRunQueueLock.Transition::UnlockIrqRestore;"}
                                ],
                            },
                        },
                    }
                },
                "records": [
                    {
                        "status": "proved",
                        "object": "BootInitRestInitPhase",
                        "transition": "Preset",
                        "source_kind": "drives",
                        "proof_class": "action_commit",
                        "proof_provider": "action_drive",
                        "expression": "Scheduler.Action::Schedule",
                    },
                    {
                        "object": "BootInitRestInitPhase",
                        "transition": "Preset",
                        "source_kind": "within",
                        "proof_class": "exclusive_context",
                        "expression": "within ScheduleRunQueueContext",
                        "process_parent": "Scheduler.Action::Schedule",
                    },
                    {
                        "object": "BootInitRestInitPhase",
                        "transition": "Preset",
                        "source_kind": "drives",
                        "proof_class": "action_result_binding",
                        "proof_provider": "within_context",
                        "expression": "let next: TaskRef <- CurrentRunQ.Action::PickNextTask(prev_ref: CurrentTaskRef)",
                        "display_expression": "let next: TaskRef <- CurrentRunQ.Action::PickNextTask(CurrentTaskRef)",
                        "process_parent": "Scheduler.Action::Schedule",
                    },
                    {
                        "object": "BootInitRestInitPhase",
                        "transition": "Preset",
                        "proof_class": "action_commit",
                        "proof_provider": "within_context",
                        "expression": "Scheduler.Action::SwitchTo(CurrentTaskRef, next)",
                        "process_parent": "Scheduler.Action::Schedule",
                    },
                    {
                        "object": "BootInitRestInitPhase",
                        "transition": "Preset",
                        "proof_class": "type_process_commit",
                        "proof_provider": "within_context",
                        "expression": "CurrentTaskRef.Action::SaveCoreContext",
                        "process_parent": "Scheduler.Action::SwitchTo(CurrentTaskRef, next)",
                    },
                    {
                        "object": "BootInitRestInitPhase",
                        "transition": "Preset",
                        "proof_class": "type_process_commit",
                        "proof_provider": "within_context",
                        "expression": "next.Action::RestoreCoreContext",
                        "process_parent": "Scheduler.Action::SwitchTo(CurrentTaskRef, next)",
                    },
                    {
                        "object": "BootInitRestInitPhase",
                        "transition": "Preset",
                        "source_kind": "within",
                        "proof_class": "exclusive_context",
                        "expression": "within ScheduleRunQueueContext exited",
                        "process_parent": "Scheduler.Action::Schedule",
                    },
                ],
            }
        )

        cells = view.metadata["trace_cells"]
        arrows = view.metadata["trace_arrows"]
        rest_init_cell = next(
            cell
            for cell in cells
            if cell.kind == "transition_span"
            and cell.label == "BootInitRestInitPhase.Transition::Preset"
        )
        context_cell = next(
            cell
            for cell in cells
            if cell.kind == "context_span"
            and "ScheduleRunQueueContext" in cell.label
        )
        action_cells = [
            cell
            for cell in cells
            if cell.kind == "context_action"
            and cell.id.startswith(context_cell.id)
        ]
        ordinary_action_cells = [cell for cell in cells if cell.kind == "action"]

        self.assertGreater(context_cell.column, rest_init_cell.column)
        schedule_cell = next(
            cell for cell in ordinary_action_cells if cell.label == "Scheduler.Action::Schedule"
        )
        self.assertEqual(context_cell.column, schedule_cell.column + 2)
        self.assertEqual(context_cell.column_span, 4)
        self.assertLessEqual(context_cell.row, schedule_cell.row)
        self.assertLess(
            schedule_cell.row,
            context_cell.row + context_cell.row_span,
        )
        self.assertEqual(
            [cell.label for cell in sorted(action_cells, key=lambda cell: cell.row)],
            [
                "let next: TaskRef <- CurrentRunQ.Action::PickNextTask(CurrentTaskRef)",
                "Scheduler.Action::SwitchTo(CurrentTaskRef, next)",
                "CurrentTaskRef.Action::SaveCoreContext",
                "next.Action::RestoreCoreContext",
            ],
        )
        pick_cell = next(
            cell
            for cell in action_cells
            if cell.label
            == "let next: TaskRef <- CurrentRunQ.Action::PickNextTask(CurrentTaskRef)"
        )
        switch_cell = next(
            cell
            for cell in action_cells
            if cell.label
            == "Scheduler.Action::SwitchTo(CurrentTaskRef, next)"
        )
        save_cell = next(
            cell
            for cell in action_cells
            if cell.label == "CurrentTaskRef.Action::SaveCoreContext"
        )
        restore_cell = next(
            cell
            for cell in action_cells
            if cell.label == "next.Action::RestoreCoreContext"
        )
        self.assertEqual(pick_cell.column, schedule_cell.column + 2)
        self.assertEqual(switch_cell.column, pick_cell.column)
        self.assertEqual(save_cell.column, pick_cell.column + 2)
        self.assertEqual(restore_cell.column, pick_cell.column + 2)
        self.assertTrue(
            all(
                cell.column_span == 2
                for cell in (pick_cell, switch_cell, save_cell, restore_cell)
            )
        )
        self.assertEqual(schedule_cell.row, pick_cell.row)
        self.assertEqual(
            schedule_cell.row + schedule_cell.row_span,
            switch_cell.row + switch_cell.row_span,
        )
        self.assertGreater(schedule_cell.row_span, 1)
        self.assertGreater(switch_cell.row, pick_cell.row)
        self.assertEqual(switch_cell.row, save_cell.row)
        self.assertEqual(
            switch_cell.row + switch_cell.row_span,
            restore_cell.row + restore_cell.row_span,
        )
        self.assertGreater(switch_cell.row_span, 1)
        self.assertGreater(restore_cell.row, save_cell.row)
        self.assertTrue(
            any(
                arrow.kind == "within"
                and arrow.source == schedule_cell.id
                and arrow.target == context_cell.id
                for arrow in arrows
            )
        )
        self.assertEqual(
            sum(1 for arrow in arrows if arrow.kind == "context_order"),
            0,
        )
        self.assertTrue(
            any(
                arrow.kind == "drives"
                and arrow.source == schedule_cell.id
                and arrow.target == pick_cell.id
                for arrow in arrows
            )
        )
        self.assertTrue(
            any(
                arrow.kind == "drives"
                and arrow.source == schedule_cell.id
                and arrow.target == switch_cell.id
                for arrow in arrows
            )
        )
        self.assertTrue(
            any(
                arrow.kind == "drives"
                and arrow.source == switch_cell.id
                and arrow.target == save_cell.id
                for arrow in arrows
            )
        )
        self.assertTrue(
            any(
                arrow.kind == "drives"
                and arrow.source == switch_cell.id
                and arrow.target == restore_cell.id
                for arrow in arrows
            )
        )

    def test_trace_view_places_contexts_that_only_wrap_child_transitions(self) -> None:
        view = build_trace_view(
            {
                "trace": [
                    {
                        "object": "InterruptPhase",
                        "transition": "Setup",
                        "source_state": "Base",
                        "target_state": "Ready",
                        "children": [
                            {
                                "object": "IrqTimeInitPhase",
                                "transition": "Setup",
                                "source_state": "Base",
                                "target_state": "Ready",
                                "children": [
                                    {
                                        "object": "IrqController",
                                        "transition": "Setup",
                                        "source_state": "Prepared",
                                        "target_state": "Ready",
                                        "children": [],
                                    }
                                ],
                            },
                            {
                                "object": "LocalIrqEnablePhase",
                                "transition": "Setup",
                                "source_state": "Base",
                                "target_state": "Ready",
                                "children": [
                                    {
                                        "object": "InterruptStream",
                                        "transition": "Enable",
                                        "source_state": "Ready",
                                        "target_state": "Online",
                                        "children": [],
                                    }
                                ],
                            },
                            {
                                "object": "IrqOpenPreparePhase",
                                "transition": "Setup",
                                "source_state": "Base",
                                "target_state": "Ready",
                                "children": [
                                    {
                                        "object": "SchedClock",
                                        "transition": "Setup",
                                        "source_state": "Prepared",
                                        "target_state": "Ready",
                                        "children": [],
                                    }
                                ],
                            },
                            {
                                "object": "ProcessPreparePhase",
                                "transition": "Setup",
                                "source_state": "Base",
                                "target_state": "Ready",
                                "children": [
                                    {
                                        "object": "RootPidNamespace",
                                        "transition": "Setup",
                                        "source_state": "Prepared",
                                        "target_state": "Ready",
                                        "children": [],
                                    }
                                ],
                            },
                        ],
                    }
                ],
                "model": {
                    "exclusive_contexts": {
                        "SingleTaskContext": {
                            "guard": {
                                "holds": [
                                    {
                                        "body": (
                                            "cpu_concurrency: single_cpu;"
                                            " local_interrupts: disabled;"
                                        )
                                    }
                                ]
                            }
                        },
                        "SingleTaskInterruptStreamContext": {
                            "guard": {
                                "holds": [
                                    {
                                        "body": (
                                            "cpu_concurrency: single_cpu;"
                                            " local_interrupts: enabled;"
                                        )
                                    }
                                ]
                            }
                        },
                    }
                },
                "records": [
                    {
                        "object": "InterruptPhase",
                        "transition": "Setup",
                        "source_kind": "within",
                        "proof_class": "exclusive_context",
                        "expression": "within SingleTaskContext",
                    },
                    {
                        "status": "proved",
                        "object": "IrqTimeInitPhase",
                        "transition": "Setup",
                        "message": (
                            "transition: IrqTimeInitPhase.Transition::Setup "
                            "State::Base -> State::Ready"
                        ),
                    },
                    {
                        "object": "InterruptPhase",
                        "transition": "Setup",
                        "source_kind": "within",
                        "proof_class": "exclusive_context",
                        "expression": "within SingleTaskContext exited",
                    },
                    {
                        "status": "proved",
                        "object": "LocalIrqEnablePhase",
                        "transition": "Setup",
                        "message": (
                            "transition: LocalIrqEnablePhase.Transition::Setup "
                            "State::Base -> State::Ready"
                        ),
                    },
                    {
                        "object": "InterruptPhase",
                        "transition": "Setup",
                        "source_kind": "within",
                        "proof_class": "exclusive_context",
                        "expression": "within SingleTaskInterruptStreamContext",
                    },
                    {
                        "status": "proved",
                        "object": "IrqOpenPreparePhase",
                        "transition": "Setup",
                        "message": (
                            "transition: IrqOpenPreparePhase.Transition::Setup "
                            "State::Base -> State::Ready"
                        ),
                    },
                    {
                        "status": "proved",
                        "object": "ProcessPreparePhase",
                        "transition": "Setup",
                        "message": (
                            "transition: ProcessPreparePhase.Transition::Setup "
                            "State::Base -> State::Ready"
                        ),
                    },
                    {
                        "object": "InterruptPhase",
                        "transition": "Setup",
                        "source_kind": "within",
                        "proof_class": "exclusive_context",
                        "expression": "within SingleTaskInterruptStreamContext exited",
                    },
                ],
            }
        )

        cells = view.metadata["trace_cells"]
        context_labels = [
            cell.label for cell in cells if cell.kind == "context_span"
        ]
        context_action_labels = [
            cell.label for cell in cells if cell.kind == "context_action"
        ]
        single_task_context = next(
            cell
            for cell in cells
            if cell.kind == "context_span" and cell.label == "SingleTaskContext"
        )
        interrupt_context = next(
            cell
            for cell in cells
            if cell.kind == "context_span"
            and cell.label == "SingleTaskInterruptStreamContext"
        )
        irq_time_phase = next(
            cell
            for cell in cells
            if cell.kind == "transition_span"
            and cell.label == "IrqTimeInitPhase.Transition::Setup"
        )
        local_irq_enable_phase = next(
            cell
            for cell in cells
            if cell.kind == "transition_span"
            and cell.label == "LocalIrqEnablePhase.Transition::Setup"
        )
        irq_open_phase = next(
            cell
            for cell in cells
            if cell.kind == "transition_span"
            and cell.label == "IrqOpenPreparePhase.Transition::Setup"
        )
        process_prepare_phase = next(
            cell
            for cell in cells
            if cell.kind == "transition_span"
            and cell.label == "ProcessPreparePhase.Transition::Setup"
        )

        self.assertIn("SingleTaskContext", context_labels)
        self.assertIn("SingleTaskInterruptStreamContext", context_labels)
        self.assertNotIn("", context_action_labels)
        self.assertLessEqual(single_task_context.row, irq_time_phase.row)
        self.assertGreaterEqual(
            single_task_context.row + single_task_context.row_span,
            irq_time_phase.row + irq_time_phase.row_span,
        )
        self.assertFalse(
            single_task_context.row
            <= local_irq_enable_phase.row
            < single_task_context.row + single_task_context.row_span
        )
        self.assertLessEqual(interrupt_context.row, irq_open_phase.row)
        self.assertGreaterEqual(
            interrupt_context.row + interrupt_context.row_span,
            process_prepare_phase.row + process_prepare_phase.row_span,
        )

    def test_trace_view_places_within_ensures_as_context_facts(self) -> None:
        view = build_trace_view(
            {
                "trace": [
                    {
                        "object": "ResourceTree",
                        "transition": "Setup",
                        "source_state": "Base",
                        "target_state": "Ready",
                        "children": [],
                    }
                ],
                "model": {
                    "exclusive_contexts": {
                        "ResourceTreeWriteContext": {
                            "lock_ref": "ResourceLock",
                            "guard": {
                                "lock_ref": "ResourceLock",
                                "entered_by": [
                                    {
                                        "body": (
                                            "ResourceLock.Transition::WriteLock"
                                            "(BootInitTaskRef);"
                                        )
                                    }
                                ],
                                "exited_by": [
                                    {
                                        "body": (
                                            "ResourceLock.Transition::WriteUnlock"
                                            "(BootInitTaskRef);"
                                        )
                                    }
                                ],
                            },
                        },
                    }
                },
                "records": [
                    {
                        "object": "ResourceTree",
                        "transition": "Setup",
                        "source_kind": "within",
                        "proof_class": "exclusive_context",
                        "expression": "within ResourceTreeWriteContext",
                    },
                    {
                        "status": "proved",
                        "object": "ResourceTree",
                        "transition": "Setup",
                        "source_kind": "within ensures",
                        "proof_class": "exclusive_context_fact",
                        "proof_provider": "within_ensures",
                        "expression": "resource_tree_ready(ResourceTree, MemBlock)",
                    },
                    {
                        "object": "ResourceTree",
                        "transition": "Setup",
                        "source_kind": "within",
                        "proof_class": "exclusive_context",
                        "expression": "within ResourceTreeWriteContext exited",
                    },
                ],
            }
        )

        cells = view.metadata["trace_cells"]
        context = next(cell for cell in cells if cell.kind == "context_span")
        fact = next(cell for cell in cells if cell.kind == "context_fact")

        self.assertIn("ResourceTreeWriteContext", context.label)
        self.assertEqual(fact.label, "resource_tree_ready(ResourceTree, MemBlock)")
        self.assertLessEqual(context.row, fact.row)
        self.assertGreaterEqual(
            context.row + context.row_span,
            fact.row + fact.row_span,
        )

    def test_trace_view_limits_nested_context_action_depth(self) -> None:
        derive_data = {
            "trace": [
                {
                    "object": "BootIdleRuntime",
                    "transition": "Enable",
                    "source_state": "Ready",
                    "target_state": "Online",
                    "children": [],
                }
            ],
            "model": {},
            "records": [
                {
                    "status": "proved",
                    "object": "BootIdleRuntime",
                    "transition": "Enable",
                    "source_kind": "within",
                    "proof_class": "exclusive_context",
                    "expression": "within BootIdleStartupContext",
                },
                {
                    "status": "proved",
                    "object": "BootIdleRuntime",
                    "transition": "Enable",
                    "source_kind": "drives",
                    "proof_class": "action_commit",
                    "proof_provider": "within_context",
                    "expression": "BootIdleRuntime.Action::RunIdleLoop",
                },
                {
                    "status": "proved",
                    "object": "BootIdleRuntime",
                    "transition": "Enable",
                    "source_kind": "drives",
                    "proof_class": "action_commit",
                    "proof_provider": "within_context",
                    "expression": "BootIdleRuntime.Action::DoIdleCycle",
                    "process_parent": "BootIdleRuntime.Action::RunIdleLoop",
                },
                {
                    "status": "proved",
                    "object": "BootIdleRuntime",
                    "transition": "Enable",
                    "source_kind": "drives",
                    "proof_class": "action_commit",
                    "proof_provider": "within_context",
                    "expression": "BootIdleRuntime.Action::ScheduleIfNeedResched",
                    "process_parent": "BootIdleRuntime.Action::DoIdleCycle",
                },
                {
                    "status": "proved",
                    "object": "BootIdleRuntime",
                    "transition": "Enable",
                    "source_kind": "drives",
                    "proof_class": "action_commit",
                    "proof_provider": "within_context",
                    "expression": "Scheduler.Action::ScheduleIdle",
                    "process_parent": "BootIdleRuntime.Action::ScheduleIfNeedResched",
                },
                {
                    "status": "proved",
                    "object": "BootIdleRuntime",
                    "transition": "Enable",
                    "source_kind": "drives",
                    "proof_class": "action_commit",
                    "proof_provider": "within_context",
                    "expression": "Scheduler.Action::Schedule",
                    "process_parent": "Scheduler.Action::ScheduleIdle",
                },
                {
                    "status": "proved",
                    "object": "BootIdleRuntime",
                    "transition": "Enable",
                    "source_kind": "within",
                    "proof_class": "exclusive_context",
                    "expression": "within ScheduleRunQueueContext",
                    "process_parent": "Scheduler.Action::Schedule",
                },
                {
                    "status": "proved",
                    "object": "BootIdleRuntime",
                    "transition": "Enable",
                    "source_kind": "drives",
                    "proof_class": "action_result_binding",
                    "proof_provider": "within_context",
                    "expression": "let next: TaskRef <- CurrentRunQueueRef.Action::PickNextTask(prev_ref: CurrentTaskRef)",
                    "display_expression": "let next: TaskRef <- CurrentRunQueueRef.Action::PickNextTask(CurrentTaskRef)",
                    "process_parent": "Scheduler.Action::Schedule",
                },
                {
                    "status": "proved",
                    "object": "BootIdleRuntime",
                    "transition": "Enable",
                    "source_kind": "within",
                    "proof_class": "exclusive_context",
                    "expression": "within ScheduleRunQueueContext exited",
                    "process_parent": "Scheduler.Action::Schedule",
                },
                {
                    "status": "proved",
                    "object": "BootIdleRuntime",
                    "transition": "Enable",
                    "source_kind": "within",
                    "proof_class": "exclusive_context",
                    "expression": "within BootIdleStartupContext exited",
                },
            ],
        }

        limited = build_trace_view(derive_data, max_action_depth=3)
        limited_cells = limited.metadata["trace_cells"]
        limited_labels = [
            cell.label for cell in limited_cells if cell.kind == "context_action"
        ]
        limited_context = next(
            cell
            for cell in limited_cells
            if cell.kind == "context_span"
            and "BootIdleStartupContext" in cell.label
        )

        self.assertIn("BootIdleRuntime.Action::RunIdleLoop", limited_labels)
        self.assertIn("BootIdleRuntime.Action::DoIdleCycle", limited_labels)
        self.assertIn("BootIdleRuntime.Action::ScheduleIfNeedResched", limited_labels)
        self.assertIn("Scheduler.Action::ScheduleIdle", limited_labels)
        self.assertNotIn("Scheduler.Action::Schedule", limited_labels)
        self.assertNotIn(
            "let next: TaskRef <- CurrentRunQueueRef.Action::PickNextTask(CurrentTaskRef)",
            limited_labels,
        )
        self.assertFalse(
            any(
                cell.kind == "context_span"
                and "ScheduleRunQueueContext" in cell.label
                for cell in limited_cells
            )
        )
        self.assertEqual(limited_context.column_span, 8)

        full = build_trace_view(derive_data, max_action_depth=None)
        full_cells = full.metadata["trace_cells"]
        full_labels = [cell.label for cell in full_cells if cell.kind == "context_action"]
        full_context = next(
            cell
            for cell in full_cells
            if cell.kind == "context_span"
            and "BootIdleStartupContext" in cell.label
        )

        self.assertIn("Scheduler.Action::Schedule", full_labels)
        self.assertIn(
            "let next: TaskRef <- CurrentRunQueueRef.Action::PickNextTask(CurrentTaskRef)",
            full_labels,
        )
        self.assertTrue(
            any(
                cell.kind == "context_span"
                and "ScheduleRunQueueContext" in cell.label
                for cell in full_cells
            )
        )
        self.assertEqual(full_context.column_span, 10)

    def test_trace_view_places_ordinary_drives_actions(self) -> None:
        view = build_trace_view(
            {
                "trace": [
                    {
                        "object": "BootInitRestInitPhase",
                        "transition": "Preset",
                        "source_state": "Ready",
                        "target_state": "Prepared",
                        "children": [
                            {
                                "object": "KernelInitTask",
                                "transition": "Enable",
                                "source_state": "Ready",
                                "target_state": "Online",
                                "children": [],
                            },
                            {
                                "object": "KthreaddTask",
                                "transition": "Preset",
                                "source_state": "Base",
                                "target_state": "Prepared",
                                "children": [],
                            },
                        ],
                    }
                ],
                "model": {},
                "records": [
                    {
                        "status": "proved",
                        "object": "KernelInitTask",
                        "transition": "Enable",
                        "message": "transition: KernelInitTask.Transition::Enable State::Ready -> State::Online",
                    },
                    {
                        "status": "proved",
                        "object": "BootInitRestInitPhase",
                        "transition": "Preset",
                        "source_kind": "drives",
                        "proof_class": "action_commit",
                        "proof_provider": "action_drive",
                        "expression": "KernelInitTask.Action::PinToBootCpu(cpu_ref: BootCPURef)",
                        "display_expression": "KernelInitTask.Action::PinToBootCpu(BootCPURef)",
                    },
                    {
                        "status": "proved",
                        "object": "BootInitRestInitPhase",
                        "transition": "Preset",
                        "source_kind": "drives",
                        "proof_class": "action_commit",
                        "proof_provider": "within_context",
                        "expression": "KernelInitTask.Action::InsideContext()",
                    },
                    {
                        "status": "proved",
                        "object": "KthreaddTask",
                        "transition": "Preset",
                        "message": "transition: KthreaddTask.Transition::Preset State::Base -> State::Prepared",
                    },
                ],
            }
        )

        cells = view.metadata["trace_cells"]
        arrows = view.metadata["trace_arrows"]
        action_cells = [cell for cell in cells if cell.kind == "action"]
        self.assertEqual(len(action_cells), 1)
        action_cell = action_cells[0]
        self.assertEqual(
            action_cell.label,
            "KernelInitTask.Action::PinToBootCpu(BootCPURef)",
        )
        self.assertFalse(any("InsideContext" in cell.label for cell in action_cells))
        rest_init_cell = next(
            cell
            for cell in cells
            if cell.kind == "transition_span"
            and cell.label == "BootInitRestInitPhase.Transition::Preset"
        )
        kernel_enable_cell = next(
            cell
            for cell in cells
            if cell.kind == "transition_span"
            and cell.label == "KernelInitTask.Transition::Enable"
        )
        kthreadd_preset_cell = next(
            cell
            for cell in cells
            if cell.kind == "transition_span"
            and cell.label == "KthreaddTask.Transition::Preset"
        )
        self.assertGreater(
            action_cell.row,
            kernel_enable_cell.row + kernel_enable_cell.row_span,
        )
        self.assertLess(action_cell.row, kthreadd_preset_cell.row)
        self.assertGreater(action_cell.column, rest_init_cell.column)
        self.assertTrue(
            any(
                arrow.kind == "action"
                and arrow.source == rest_init_cell.id
                and arrow.target == action_cell.id
                for arrow in arrows
            )
        )

    def test_invalid_model_schema_returns_usage_error_code(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            model = Path(tmp) / "bad.model.json"
            output = Path(tmp) / "bad.view.json"
            model.write_text('{"schema": "wrong", "version": 1}\n', encoding="utf-8")

            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exit_code = view_main([str(model), "object", "-o", str(output)])

            self.assertEqual(exit_code, 2)
            self.assertIn("error: invalid input JSON", stderr.getvalue())

    def test_view_tool_does_not_import_pyveri(self) -> None:
        source_root = Path(__file__).resolve().parents[1] / "src" / "view_tool"

        for path in source_root.rglob("*.py"):
            text = path.read_text(encoding="utf-8")
            self.assertNotIn("from pyveri", text, str(path))
            self.assertNotIn("import pyveri", text, str(path))


if __name__ == "__main__":
    unittest.main()
