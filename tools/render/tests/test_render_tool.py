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
    render_svg,
)
from view_tool.__main__ import main as view_main


class RenderToolTests(unittest.TestCase):
    def setUp(self) -> None:
        self.spec = (
            Path(__file__).resolve().parents[3]
            / "spec"
            / "entry-prelude-object-model.spec"
        )

    def _build_view_json(self, tmp: str, view_name: str) -> Path:
        ast = Path(tmp) / "entry-prelude-object-model.ast.json"
        model = Path(tmp) / "entry-prelude-object-model.model.json"
        view = Path(tmp) / f"entry-prelude-object-model.{view_name}.view.json"
        self.assertEqual(parse_main([str(self.spec), "-o", str(ast)]), 0)
        self.assertEqual(model_main([str(ast), "-o", str(model)]), 0)
        view_input = model
        if view_name == "trace":
            derive = Path(tmp) / "entry-prelude-object-model.derive.json"
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
            self.assertIn("StartupTimeline: TimelineObject", stdout.getvalue())

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
            self.assertIn("StartupTimeline.Event::Setup", text)
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
            self.assertIn("StartupTimeline.Setup", text)
            self.assertIn("PreparePhase.Setup", text)
            self.assertNotIn("PreparePhase.Enable", text)
            self.assertIn("phase-arrow", text)
            self.assertIn('marker id="dot"', text)
            self.assertIn(">Riscv64</tspan>", text)
            self.assertIn('dy="12">Online</tspan>', text)
            self.assertIn("<tspan", text)
            self.assertIn("depends-arrow", text)
            self.assertNotIn("StartupTimeline.Base", text)

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
                        "kind": "context_guard",
                        "label": "within.guard",
                        "group_role": "context_guard",
                    },
                    {"index": 5, "kind": "gap", "label": "body.end"},
                    {"index": 6, "kind": "state", "label": "target"},
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
                        row=6,
                        column=0,
                        label="KernelInitTask.State::Runnable",
                    ),
                    TraceCell(
                        id="event",
                        kind="event_span",
                        row=1,
                        column=0,
                        label="KernelInitTask.Event::Enable",
                        row_span=4,
                    ),
                    TraceCell(
                        id="context",
                        kind="context_span",
                        row=2,
                        column=1,
                        label=(
                            "WakeUpNewTaskContext"
                            "|lock=KernelInitTaskPiLock"
                            "|guard=RawSpinLockIrqSaveGuard"
                            "|enter=KernelInitTaskPiLock.Event::LockIrqSave"
                            "|exit=KernelInitTaskPiLock.Event::UnlockIrqRestore"
                        ),
                        row_span=3,
                        column_span=2,
                    ),
                    TraceCell(
                        id="action-0",
                        kind="context_action",
                        row=2,
                        column=1,
                        label="KernelInitTask.Event::SetRuntimeState(Runnable)",
                        column_span=2,
                    ),
                    TraceCell(
                        id="action-1",
                        kind="context_action",
                        row=3,
                        column=1,
                        label="Scheduler.Action::SelectRunQueue(KernelInitTask)",
                        column_span=2,
                    ),
                ),
                "trace_arrows": (
                    TraceArrow(source="source", target="target", kind="state"),
                    TraceArrow(source="event", target="context", kind="within"),
                    TraceArrow(
                        source="action-0", target="action-1", kind="context_order"
                    ),
                ),
            },
        )

        text = render_svg(view)

        self.assertIn("context-box", text)
        self.assertIn("context-action", text)
        self.assertIn("within-arrow", text)
        self.assertIn("context-order", text)
        self.assertIn("WakeUpNewTaskContext", text)
        self.assertIn("lock: KernelInitTaskPiLock", text)
        self.assertNotIn("guard: RawSpinLockIrqSaveGuard", text)
        self.assertNotIn("enter: KernelInitTaskPiLock.LockIrqSave", text)
        self.assertNotIn("exit: KernelInitTaskPiLock.UnlockIrqRestore", text)
        self.assertIn("SetRuntimeState", text)
        self.assertIn("SelectRunQueue", text)

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
  "events": {
    "Vm.Event::Setup": "build early mappings"
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
  "events": {
    "Vm.Event::Setup": "建立早期虚拟地址空间并完成跳板页表切换后的连续中文说明"
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
