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

from common.view_types import TraceArrow, TraceCell, ViewModel
from model_tool.__main__ import main as model_main
from parse_tool.__main__ import main as parse_main
from derive_tool.__main__ import main as derive_main
from render_tool.__main__ import main as render_main
from render_tool.render import (
    _choose_annotation_box,
    _expand_rect,
    _rects_overlap,
    _trace_action_rect,
    _trace_annotation_occupied_boxes,
    _trace_context_action_rect,
    _trace_context_box_rect,
    _trace_row_metrics,
    render_svg,
)
from view_tool.__main__ import main as view_main


class RenderToolTests(unittest.TestCase):
    def setUp(self) -> None:
        self.spec = (
            Path(__file__).resolve().parents[3]
            / "spec"
            / "model"
            / "main.spec"
        )

    def _build_view_json(self, tmp: str, view_name: str) -> Path:
        ast = Path(tmp) / "model-main.ast.json"
        model = Path(tmp) / "model-main.model.json"
        view = Path(tmp) / f"model-main.{view_name}.view.json"
        self.assertEqual(parse_main([str(self.spec), "-o", str(ast)]), 0)
        self.assertEqual(model_main([str(ast), "-o", str(model)]), 0)
        view_input = model
        if view_name == "trace":
            derive = Path(tmp) / "model-main.derive.json"
            self.assertEqual(derive_main([str(model), "-o", str(derive)]), 0)
            view_input = derive
        self.assertEqual(view_main([str(view_input), view_name, "-o", str(view)]), 0)
        return view

    def test_render_text_to_stdout(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            view = self._build_view_json(tmp, "object")

            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exit_code = render_main([str(view), "--format", "text"])

            self.assertEqual(exit_code, 0)
            self.assertIn("object view:", stdout.getvalue())
            self.assertIn("ComputerProject: ProjectObject", stdout.getvalue())

    def test_render_dot_writes_ascii_file(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            view = self._build_view_json(tmp, "object")
            output = Path(tmp) / "object.gv"

            exit_code = render_main([str(view), "--format", "dot", "-o", str(output)])

            self.assertEqual(exit_code, 0)
            data = output.read_bytes()
            self.assertTrue(data.startswith(b"digraph ObjectView"))
            data.decode("ascii")

    def test_render_svg_from_timeline_view(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            view = self._build_view_json(tmp, "timeline")
            output = Path(tmp) / "timeline.svg"

            exit_code = render_main([str(view), "--format", "svg", "-o", str(output)])

            self.assertEqual(exit_code, 0)
            text = output.read_text(encoding="utf-8")
            self.assertTrue(text.startswith('<?xml version="1.0" encoding="UTF-8"?>'))
            self.assertIn("<svg", text)
            self.assertIn("PreparePhase", text)
            self.assertIn("BootPhase", text)

    def test_render_text_from_trace_view(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            view = self._build_view_json(tmp, "trace")

            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exit_code = render_main([str(view), "--format", "text"])

            self.assertEqual(exit_code, 0)
            text = stdout.getvalue()
            self.assertIn("trace view:", text)
            self.assertIn("columns:", text)
            self.assertIn("ComputerProject.Transition::Preset", text)
            self.assertIn("[emits]", text)
            self.assertIn("[drives]", text)

    def test_render_svg_from_trace_view(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            view = self._build_view_json(tmp, "trace")
            output = Path(tmp) / "trace.svg"

            exit_code = render_main([str(view), "--format", "svg", "-o", str(output)])

            self.assertEqual(exit_code, 0)
            text = output.read_text(encoding="utf-8")
            self.assertTrue(text.startswith('<?xml version="1.0" encoding="UTF-8"?>'))
            self.assertIn("<svg", text)
            self.assertIn("ComputerProject.Preset", text)
            self.assertIn("ComputerProject.Setup", text)
            self.assertIn("PreparePhase.Setup", text)
            self.assertNotIn("PreparePhase.Enable", text)
            self.assertIn("phase-arrow", text)
            self.assertIn('marker id="dot"', text)
            self.assertIn(">Riscv64</tspan>", text)
            self.assertIn('dy="12">Online</tspan>', text)
            self.assertIn("<tspan", text)
            self.assertIn("depends-arrow", text)
            self.assertNotIn("ComputerProject.Base", text)

    def test_render_svg_from_trace_view_draws_within_context(self) -> None:
        view = ViewModel(
            name="trace",
            graph_format="svg",
            metadata={
                "trace_columns": [
                    {"index": 0, "kind": "object", "depth": 0},
                    {"index": 1, "kind": "gap", "depth": 0},
                    {"index": 2, "kind": "object", "depth": 1},
                ],
                "trace_rows": [
                    {"index": 0, "kind": "state", "label": "source"},
                    {"index": 1, "kind": "gap", "label": "body.start"},
                    {
                        "index": 2,
                        "kind": "context_action",
                        "label": "within.0",
                        "group_role": "context_action",
                    },
                    {
                        "index": 3,
                        "kind": "context_action",
                        "label": "within.1",
                        "group_role": "context_action",
                    },
                    {
                        "index": 4,
                        "kind": "context_action",
                        "label": "within.2",
                        "group_role": "context_action",
                    },
                    {
                        "index": 5,
                        "kind": "context_guard",
                        "label": "within.guard",
                        "group_role": "context_guard",
                    },
                    {"index": 6, "kind": "gap", "label": "body.end"},
                    {"index": 7, "kind": "state", "label": "target"},
                ],
                "trace_cells": (
                    TraceCell(
                        id="source",
                        kind="state",
                        row=0,
                        column=0,
                        label="KernelInitTask.State::Created",
                    ),
                    TraceCell(
                        id="target",
                        kind="state",
                        row=7,
                        column=0,
                        label="KernelInitTask.State::Runnable",
                    ),
                    TraceCell(
                        id="transition",
                        kind="transition_span",
                        row=1,
                        column=0,
                        label="KernelInitTask.Transition::Enable",
                        row_span=5,
                    ),
                    TraceCell(
                        id="context",
                        kind="context_span",
                        row=2,
                        column=1,
                        label=(
                            "WakeUpNewTaskContext"
                            "|lock=KernelInitTaskPiLock"
                            "|enter=KernelInitTaskPiLock.Transition::LockIrqSave"
                            "|exit=KernelInitTaskPiLock.Transition::UnlockIrqRestore"
                        ),
                        row_span=4,
                        column_span=2,
                    ),
                    TraceCell(
                        id="nested-context",
                        kind="context_span",
                        row=4,
                        column=1,
                        label="EnqueueSelectedRunQueueContext|lock=BootRunQueueLock",
                        row_span=1,
                        column_span=2,
                    ),
                    TraceCell(
                        id="action-0",
                        kind="context_action",
                        row=2,
                        column=1,
                        label="KernelInitTask.Transition::SetRuntimeState(TaskRuntimeState::Running)",
                        column_span=2,
                    ),
                    TraceCell(
                        id="action-1",
                        kind="context_action",
                        row=3,
                        column=1,
                        label="let selected_rq: RunQueueRef <- Scheduler.Action::SelectRunQueue(KernelInitTaskRef)",
                        column_span=2,
                    ),
                    TraceCell(
                        id="action-2",
                        kind="context_action",
                        row=4,
                        column=1,
                        label="selected_rq.Transition::EnqueueTask(KernelInitTaskRef)",
                        column_span=2,
                    ),
                ),
                "trace_arrows": (
                    TraceArrow(source="source", target="target", kind="state"),
                    TraceArrow(source="transition", target="context", kind="within"),
                    TraceArrow(
                        source="action-0", target="action-1", kind="context_order"
                    ),
                    TraceArrow(
                        source="action-1", target="action-2", kind="context_order"
                    ),
                ),
            },
        )

        text = render_svg(view)
        context_rect = _trace_context_box_rect((206.0, 122.0, 259.0, 196.0))
        action_rect = _trace_context_action_rect((206.0, 262.0, 259.0, 56.0))

        self.assertIn("context-box", text)
        self.assertIn("context-action", text)
        self.assertIn('x="208.0" y="128.0" width="255.0"', text)
        self.assertIn('x="214.0" y="273.0" width="243.0"', text)
        self.assertEqual(action_rect[2], 243.0)
        self.assertEqual(
            action_rect[0] + action_rect[2] / 2,
            context_rect[0] + context_rect[2] / 2,
        )
        self.assertIn("within-arrow", text)
        self.assertIn("context-order", text)
        self.assertIn("WakeUpNewTaskContext", text)
        self.assertIn("lock: KernelInitTaskPiLock", text)
        self.assertNotIn("guard:", text)
        self.assertNotIn("enter: KernelInitTaskPiLock.LockIrqSave", text)
        self.assertNotIn("exit: KernelInitTaskPiLock.UnlockIrqRestore", text)
        self.assertIn("SetRuntimeState", text)
        self.assertIn("selected_rq: RunQueueRef &lt;-", text)
        self.assertIn("Scheduler.SelectRunQueue", text)
        self.assertIn("EnqueueTask", text)

    def test_render_svg_from_trace_view_hides_top_level_contexts(self) -> None:
        view = ViewModel(
            name="trace",
            graph_format="svg",
            metadata={
                "trace_columns": [
                    {"index": 0, "kind": "object", "depth": 0},
                    {"index": 1, "kind": "gap", "depth": 0},
                    {"index": 2, "kind": "object", "depth": 1},
                ],
                "trace_rows": [
                    {"index": 0, "kind": "gap", "label": "body.start"},
                    {
                        "index": 1,
                        "kind": "context_action",
                        "label": "within.0",
                        "group_role": "context_action",
                    },
                    {
                        "index": 2,
                        "kind": "context_fact",
                        "label": "within.fact",
                        "group_role": "context_action",
                    },
                    {"index": 3, "kind": "gap", "label": "body.end"},
                ],
                "trace_cells": (
                    TraceCell(
                        id="transition",
                        kind="transition_span",
                        row=0,
                        column=0,
                        label="KernelInitTask.Transition::Enable",
                        row_span=4,
                    ),
                    TraceCell(
                        id="single-task-context",
                        kind="context_span",
                        row=1,
                        column=1,
                        label="SingleTaskContext",
                        row_span=2,
                        column_span=2,
                    ),
                    TraceCell(
                        id="interrupt-stream-context",
                        kind="context_span",
                        row=1,
                        column=1,
                        label="SingleTaskInterruptStreamContext",
                        row_span=2,
                        column_span=2,
                    ),
                    TraceCell(
                        id="lock-context",
                        kind="context_span",
                        row=1,
                        column=1,
                        label="ResourceLockContext|lock=BootRunQueueLock",
                        row_span=2,
                        column_span=2,
                    ),
                    TraceCell(
                        id="context-action",
                        kind="context_action",
                        row=1,
                        column=1,
                        label="BootRunQueueLock.Transition::LockIrqSave",
                        column_span=2,
                    ),
                    TraceCell(
                        id="context-fact",
                        kind="context_fact",
                        row=2,
                        column=1,
                        label="local_interrupts = enabled",
                        column_span=2,
                    ),
                ),
                "trace_arrows": (
                    TraceArrow(
                        source="transition", target="single-task-context", kind="within"
                    ),
                    TraceArrow(
                        source="transition",
                        target="interrupt-stream-context",
                        kind="within",
                    ),
                    TraceArrow(source="transition", target="lock-context", kind="within"),
                ),
            },
        )

        unfiltered = render_svg(view)
        text = render_svg(
            view,
            trace_hidden_contexts=(
                "SingleTaskContext",
                "SingleTaskInterruptStreamContext",
            ),
        )

        self.assertIn("SingleTaskContext", unfiltered)
        self.assertIn("SingleTaskInterruptStreamContext", unfiltered)
        self.assertNotIn("SingleTaskContext", text)
        self.assertNotIn("SingleTaskInterruptStreamContext", text)
        self.assertIn("ResourceLockContext", text)
        self.assertIn("BootRunQueueLock.Transition::LockIrqSave", text)
        self.assertIn("local_interrupts = enabled", text)
        self.assertEqual(text.count('class="context-box"'), 1)
        self.assertEqual(text.count('class="within-arrow"'), 1)

    def test_render_svg_from_trace_view_draws_ordinary_action(self) -> None:
        view = ViewModel(
            name="trace",
            graph_format="svg",
            metadata={
                "trace_columns": [
                    {"index": 0, "kind": "phase", "depth": 0},
                    {"index": 1, "kind": "phase_object_gap", "depth": 0},
                    {"index": 2, "kind": "object", "depth": 0},
                    {"index": 3, "kind": "gap", "depth": 0},
                ],
                "trace_rows": [
                    {"index": 0, "kind": "gap", "label": "body.start"},
                    {"index": 1, "kind": "action", "label": "action"},
                    {"index": 2, "kind": "gap", "label": "body.end"},
                ],
                "trace_cells": (
                    TraceCell(
                        id="transition",
                        kind="transition_span",
                        row=0,
                        column=0,
                        label="BootInitRestInitPhase.Transition::Preset",
                        row_span=3,
                    ),
                    TraceCell(
                        id="action",
                        kind="action",
                        row=1,
                        column=3,
                        label="KernelInitTask.Action::PinToBootCpu(BootCPURef)",
                        column_span=2,
                    ),
                ),
                "trace_arrows": (
                    TraceArrow(source="transition", target="action", kind="action"),
                ),
            },
        )

        text = render_svg(view)

        self.assertIn("action", text)
        self.assertIn("action-arrow", text)
        self.assertIn("PinToBootCpu", text)
        self.assertIn("KernelInitTask.Action::PinToBootCpu(BootCPURef)", text)
        self.assertIn(">KernelInitTask</tspan>", text)
        self.assertIn('dy="12">PinToBootCpu</tspan>', text)

    def test_render_svg_from_trace_view_aligns_action_arrow_to_action_box(self) -> None:
        view = ViewModel(
            name="trace",
            graph_format="svg",
            metadata={
                "trace_columns": [
                    {"index": 0, "kind": "phase", "depth": 0},
                    {"index": 1, "kind": "phase_object_gap", "depth": 0},
                    {"index": 2, "kind": "object", "depth": 0},
                    {"index": 3, "kind": "gap", "depth": 0},
                ],
                "trace_rows": [
                    {"index": 0, "kind": "gap", "label": "body.start"},
                    {"index": 1, "kind": "gap", "label": "spacer"},
                    {"index": 2, "kind": "action", "label": "action"},
                    {"index": 3, "kind": "gap", "label": "body.end"},
                ],
                "trace_cells": (
                    TraceCell(
                        id="transition",
                        kind="transition_span",
                        row=0,
                        column=0,
                        label="BootInitRestInitPhase.Transition::Preset",
                        row_span=4,
                    ),
                    TraceCell(
                        id="action",
                        kind="action",
                        row=2,
                        column=3,
                        label="KernelInitTask.Action::PinToBootCpu(BootCPURef)",
                        column_span=2,
                    ),
                ),
                "trace_arrows": (
                    TraceArrow(source="transition", target="action", kind="action"),
                ),
            },
        )

        text = render_svg(view)

        self.assertIn(
            '<line class="action-arrow" x1="112.0" y1="97.0" x2="354.0" y2="97.0" />',
            text,
        )

    def test_render_svg_from_trace_view_points_context_drives_to_child_boxes(self) -> None:
        view = ViewModel(
            name="trace",
            graph_format="svg",
            metadata={
                "trace_columns": [
                    {"index": 0, "kind": "object", "depth": 0},
                    {"index": 1, "kind": "gap", "depth": 0},
                    {"index": 2, "kind": "object", "depth": 1},
                ],
                "trace_rows": [
                    {
                        "index": 0,
                        "kind": "context_action",
                        "label": "schedule",
                        "group_role": "context_action",
                    },
                    {
                        "index": 1,
                        "kind": "context_action",
                        "label": "pick",
                        "group_role": "context_action",
                    },
                    {
                        "index": 2,
                        "kind": "context_action",
                        "label": "switch",
                        "group_role": "context_action",
                    },
                    {
                        "index": 3,
                        "kind": "context_action",
                        "label": "save",
                        "group_role": "context_action",
                    },
                    {
                        "index": 4,
                        "kind": "context_action",
                        "label": "restore",
                        "group_role": "context_action",
                    },
                ],
                "trace_cells": (
                    TraceCell(
                        id="schedule",
                        kind="action",
                        row=1,
                        column=0,
                        label="Scheduler.Action::Schedule",
                        row_span=2,
                    ),
                    TraceCell(
                        id="pick",
                        kind="context_action",
                        row=1,
                        column=2,
                        label="let next: TaskRef <- CurrentRunQ.Action::PickNextTask(CurrentTaskRef)",
                    ),
                    TraceCell(
                        id="switch",
                        kind="context_action",
                        row=2,
                        column=2,
                        label="Scheduler.Action::SwitchTo(CurrentTaskRef, next)",
                    ),
                    TraceCell(
                        id="save",
                        kind="context_action",
                        row=3,
                        column=2,
                        label="CurrentTaskRef.Action::SaveCoreContext",
                    ),
                    TraceCell(
                        id="restore",
                        kind="context_action",
                        row=4,
                        column=2,
                        label="next.Action::RestoreCoreContext",
                    ),
                ),
                "trace_arrows": (
                    TraceArrow(source="schedule", target="pick", kind="drives"),
                    TraceArrow(source="schedule", target="switch", kind="drives"),
                    TraceArrow(source="switch", target="save", kind="drives"),
                    TraceArrow(source="switch", target="restore", kind="drives"),
                ),
            },
        )

        text = render_svg(view)

        self.assertIn(
            '<line class="drive-arrow" x1="198.0" y1="225.0" x2="301.0" y2="225.0" />',
            text,
        )
        self.assertIn(
            '<line class="drive-arrow" x1="198.0" y1="169.0" x2="301.0" y2="169.0" />',
            text,
        )
        self.assertIn(
            '<line class="drive-arrow" x1="376.0" y1="151.0" x2="376.0" y2="129.0" />',
            text,
        )
        self.assertIn(
            '<line class="drive-arrow" x1="376.0" y1="151.0" x2="376.0" y2="73.0" />',
            text,
        )

    def test_render_svg_from_trace_view_does_not_overlap_phase_actions(self) -> None:
        rows = [
            {"index": 0, "kind": "gap", "label": "BootInitRestInitPhase.Transition::Preset.body.start"},
            {"index": 1, "kind": "action", "label": "BootInitRestInitPhase.Transition::Preset.action.1"},
            {"index": 2, "kind": "action", "label": "BootInitRestInitPhase.Transition::Preset.action.2"},
            {"index": 3, "kind": "gap", "label": "BootInitRestInitPhase.Transition::Preset.body.end"},
        ]
        view = ViewModel(
            name="trace",
            graph_format="svg",
            metadata={
                "trace_columns": [
                    {"index": 0, "kind": "phase", "depth": 0},
                    {"index": 1, "kind": "phase_object_gap", "depth": 0},
                    {"index": 2, "kind": "object", "depth": 0},
                    {"index": 3, "kind": "gap", "depth": 0},
                    {"index": 4, "kind": "object", "depth": 1},
                ],
                "trace_rows": rows,
                "trace_cells": (
                    TraceCell(
                        id="transition",
                        kind="transition_span",
                        row=0,
                        column=0,
                        label="BootInitRestInitPhase.Transition::Preset",
                        row_span=4,
                    ),
                    TraceCell(
                        id="action-1",
                        kind="action",
                        row=1,
                        column=3,
                        label="KthreaddReadyGate.Transition::Complete",
                        column_span=2,
                    ),
                    TraceCell(
                        id="action-2",
                        kind="action",
                        row=2,
                        column=3,
                        label="Scheduler.Action::Schedule",
                        column_span=2,
                    ),
                ),
                "trace_arrows": (
                    TraceArrow(source="transition", target="action-1", kind="action"),
                    TraceArrow(source="transition", target="action-2", kind="action"),
                ),
            },
        )

        render_svg(view)
        row_metrics = _trace_row_metrics(rows)
        total_height = 72 + 80 + sum(metric[1] for metric in row_metrics.values())

        def row_box(row: int) -> tuple[float, float, float, float]:
            y = total_height - 80 - sum(
                row_metrics[index][1]
                for index in range(row + 1)
                if index in row_metrics
            )
            return (344.0, y, 260.0, row_metrics[row][1])

        first = _trace_action_rect(row_box(1))
        second = _trace_action_rect(row_box(2))
        self.assertFalse(_rects_overlap(first, second))

    def test_render_svg_from_trace_view_with_annotations(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            view = self._build_view_json(tmp, "trace")
            annotations = Path(tmp) / "notes.json"
            annotations.write_text(
                """
{
  "states": {
    "Vm.State::Ready": "early virtual address space available"
  },
  "transitions": {
    "Vm.Transition::Setup": "build early mappings"
  }
}
""".strip()
                + "\n",
                encoding="utf-8",
            )
            output = Path(tmp) / "trace.svg"

            exit_code = render_main(
                [
                    str(view),
                    "--format",
                    "svg",
                    "--annotations",
                    str(annotations),
                    "-o",
                    str(output),
                ]
            )

            self.assertEqual(exit_code, 0)
            text = output.read_text(encoding="utf-8")
            self.assertIn("annotation-box", text)
            self.assertIn("annotation-leader", text)
            self.assertIn("early virtual", text)
            self.assertIn("early virtual address", text)
            self.assertIn("space available", text)
            self.assertIn("build early mappings", text)

    def test_render_svg_from_trace_view_wraps_unspaced_annotations(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            view = self._build_view_json(tmp, "trace")
            annotations = Path(tmp) / "notes.json"
            annotations.write_text(
                """
{
  "transitions": {
    "Vm.Transition::Setup": "建立早期虚拟地址空间并完成跳板页表切换后的连续中文说明"
  }
}
""".strip()
                + "\n",
                encoding="utf-8",
            )
            output = Path(tmp) / "trace.svg"

            exit_code = render_main(
                [
                    str(view),
                    "--format",
                    "svg",
                    "--annotations",
                    str(annotations),
                    "-o",
                    str(output),
                ]
            )

            self.assertEqual(exit_code, 0)
            text = output.read_text(encoding="utf-8")
            self.assertIn("建立早期虚拟地址空间并完成", text)
            self.assertIn("跳板页表切换后的连续中文说", text)
            self.assertIn("明", text)

    def test_trace_annotation_fallback_stays_near_target_y(self) -> None:
        target = (100.0, 1000.0, 50.0, 30.0)
        occupied = [(0.0, 900.0, 400.0, 250.0)]

        box = _choose_annotation_box(
            target,
            width=160.0,
            height=38.0,
            occupied=occupied,
            fallback_x=500.0,
            fallback_y=18.0,
            min_y=18.0,
        )

        self.assertEqual(box[0], 500.0)
        self.assertGreater(box[1], 900.0)

    def test_trace_annotation_collision_uses_clearance_rects(self) -> None:
        occupied = _expand_rect((100.0, 100.0, 80.0, 30.0), 6.0)

        box = _choose_annotation_box(
            target=(10.0, 100.0, 30.0, 30.0),
            width=80.0,
            height=30.0,
            occupied=[occupied],
            fallback_x=300.0,
            fallback_y=18.0,
            min_y=18.0,
        )

        self.assertFalse(_rects_overlap(box, occupied))

    def test_trace_annotation_occupied_boxes_include_actions(self) -> None:
        action = TraceCell(
            id="action",
            kind="action",
            row=0,
            column=0,
            label="KernelInitTask.Action::PinToBootCpu(BootCPURef)",
            column_span=2,
        )

        boxes = _trace_annotation_occupied_boxes(
            (action,),
            lambda _cell: (100.0, 200.0, 260.0, 46.0),
            set(),
            set(),
        )

        self.assertEqual(boxes, [_trace_action_rect((100.0, 200.0, 260.0, 46.0))])

    def test_svg_requires_timeline_or_trace_view(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            view = self._build_view_json(tmp, "object")
            output = Path(tmp) / "object.svg"

            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exit_code = render_main([str(view), "--format", "svg", "-o", str(output)])

            self.assertEqual(exit_code, 2)
            self.assertIn("error: cannot render view JSON", stderr.getvalue())

    def test_missing_annotations_returns_usage_error_code(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            view = self._build_view_json(tmp, "trace")
            output = Path(tmp) / "trace.svg"
            missing = Path(tmp) / "missing-notes.json"

            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exit_code = render_main(
                    [
                        str(view),
                        "--format",
                        "svg",
                        "--annotations",
                        str(missing),
                        "-o",
                        str(output),
                    ]
                )

            self.assertEqual(exit_code, 2)
            self.assertIn(f"error: cannot read {missing}", stderr.getvalue())

    def test_invalid_view_schema_returns_usage_error_code(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            view = Path(tmp) / "bad.view.json"
            view.write_text('{"schema": "wrong", "version": 1}\n', encoding="utf-8")

            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exit_code = render_main([str(view), "--format", "text"])

            self.assertEqual(exit_code, 2)
            self.assertIn("error: cannot render view JSON", stderr.getvalue())

    def test_render_tool_does_not_import_pyveri(self) -> None:
        source_root = Path(__file__).resolve().parents[1] / "src" / "render_tool"

        for path in source_root.rglob("*.py"):
            text = path.read_text(encoding="utf-8")
            self.assertNotIn("from pyveri", text, str(path))
            self.assertNotIn("import pyveri", text, str(path))


if __name__ == "__main__":
    unittest.main()
