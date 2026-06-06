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
            / "entry-prelude-object-model.spec"
        )

    def _build_model_json(self, tmp: str) -> Path:
        ast = Path(tmp) / "entry-prelude-object-model.ast.json"
        model = Path(tmp) / "entry-prelude-object-model.model.json"
        self.assertEqual(parse_main([str(self.spec), "-o", str(ast)]), 0)
        self.assertEqual(model_main([str(ast), "-o", str(model)]), 0)
        return model

    def _build_derive_json(self, tmp: str) -> Path:
        model = self._build_model_json(tmp)
        derive = Path(tmp) / "entry-prelude-object-model.derive.json"
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
            self.assertIn("StartupTimeline", data["nodes"])
            self.assertTrue(
                any(
                    edge["source"] == "StartupTimeline"
                    and edge["target"] == "PreparePhase"
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
            self.assertIn("StartupTimeline.Setup", data["nodes"])
            self.assertTrue(
                any(
                    edge["source"] == "StartupTimeline.Setup"
                    and edge["target"] == "PreparePhase.Setup"
                    for edge in data["edges"]
                )
            )

    def test_timeline_view_contains_rows_metadata(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            model = self._build_model_json(tmp)
            output = Path(tmp) / "timeline.view.json"
            self.assertEqual(view_main([str(model), "timeline", "-o", str(output)]), 0)

            data = read_json(output)
            self.assertEqual(data["view"], "timeline")
            self.assertEqual(data["graph_format"], "svg")
            rows = data["metadata"]["timeline_rows"]
            self.assertTrue(any(row["phase"] == "PreparePhase" for row in rows))
            self.assertTrue(any(row["phase"] == "BootPhase" for row in rows))
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
                    cell["kind"] == "event_span"
                    and cell["label"] == "StartupTimeline.Event::Setup"
                    for cell in metadata["trace_cells"]
                )
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
            prepare_setup_cell = next(
                cell
                for cell in metadata["trace_cells"]
                if cell["kind"] == "event_span"
                and cell["label"] == "PreparePhase.Event::Setup"
            )
            self.assertGreater(riscv64_cell["column"], prepare_setup_cell["column"])
            root_stream_cell = next(
                cell
                for cell in metadata["trace_cells"]
                if cell["kind"] == "state"
                and cell["label"] == "RootStream.State::Base"
            )
            self.assertEqual(riscv64_cell["column"], root_stream_cell["column"])
            self.assertFalse(
                any(
                    cell["kind"] == "event_span"
                    and cell["label"] == "PreparePhase.Event::Enable"
                    for cell in metadata["trace_cells"]
                )
            )
            boot_setup_cell = next(
                cell
                for cell in metadata["trace_cells"]
                if cell["kind"] == "event_span"
                and cell["label"] == "BootPhase.Event::Setup"
            )
            entry_prelude_setup_cell = next(
                cell
                for cell in metadata["trace_cells"]
                if cell["kind"] == "event_span"
                and cell["label"] == "EntryPreludePhase.Event::Setup"
            )
            self.assertEqual(boot_setup_cell["column"], prepare_setup_cell["column"])
            self.assertEqual(
                entry_prelude_setup_cell["column"], boot_setup_cell["column"] + 1
            )
            self.assertTrue(
                any(
                    row.get("group_role") == "body_start"
                    and row.get("label") == "RootStream.Event::Preset.body.start"
                    for row in metadata["trace_rows"]
                )
            )
            prepare_ready_cells = [
                cell
                for cell in metadata["trace_cells"]
                if cell["kind"] == "state"
                and cell["label"] == "PreparePhase.State::Ready"
            ]
            self.assertEqual(len(prepare_ready_cells), 1)

    def test_trace_view_places_within_context_actions(self) -> None:
        view = build_trace_view(
            {
                "trace": [
                    {
                        "object": "KernelInitTask",
                        "event": "Enable",
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
                                "kind": "RawSpinLockIrqSaveGuard",
                                "lock_ref": "KernelInitTaskPiLock",
                                "entered_by": [
                                    {"body": "KernelInitTaskPiLock.Event::LockIrqSave;"}
                                ],
                                "exited_by": [
                                    {
                                        "body": "KernelInitTaskPiLock.Event::UnlockIrqRestore;"
                                    }
                                ],
                            },
                        }
                    }
                },
                "records": [
                    {
                        "object": "KernelInitTask",
                        "event": "Enable",
                        "source_kind": "within",
                        "proof_class": "exclusive_context",
                        "expression": "within WakeUpNewTaskContext",
                    },
                    {
                        "object": "KernelInitTask",
                        "event": "Enable",
                        "proof_class": "action_commit",
                        "proof_provider": "within_context",
                        "expression": "KernelInitTask.Action::SetTaskState(Runnable)",
                    },
                    {
                        "object": "KernelInitTask",
                        "event": "Enable",
                        "proof_class": "action_commit",
                        "proof_provider": "within_context",
                        "expression": "Scheduler.Action::SelectRunQueue(KernelInitTask)",
                    },
                    {
                        "object": "KernelInitTask",
                        "event": "Enable",
                        "proof_class": "action_commit",
                        "proof_provider": "within_context",
                        "expression": "BootRunQueue.Action::EnqueueTask(KernelInitTask)",
                    },
                    {
                        "object": "KernelInitTask",
                        "event": "Enable",
                        "source_kind": "within",
                        "proof_class": "exclusive_context",
                        "expression": "within WakeUpNewTaskContext exited",
                    },
                ],
            }
        )

        metadata = view.metadata
        cells = metadata["trace_cells"]
        rows = metadata["trace_rows"]
        arrows = metadata["trace_arrows"]
        event_cell = next(cell for cell in cells if cell.kind == "event_span")
        context_cell = next(cell for cell in cells if cell.kind == "context_span")
        self.assertEqual(context_cell.column, event_cell.column + 1)
        self.assertEqual(context_cell.column_span, 2)
        self.assertEqual(context_cell.row_span, 4)
        self.assertIn("WakeUpNewTaskContext", context_cell.label)
        self.assertIn("lock=KernelInitTaskPiLock", context_cell.label)
        self.assertIn("guard=RawSpinLockIrqSaveGuard", context_cell.label)
        self.assertIn(
            "enter=KernelInitTaskPiLock.Event::LockIrqSave", context_cell.label
        )
        self.assertIn(
            "exit=KernelInitTaskPiLock.Event::UnlockIrqRestore", context_cell.label
        )
        action_cells = [cell for cell in cells if cell.kind == "context_action"]
        self.assertTrue(
            all(
                cell.column == context_cell.column and cell.column_span == 2
                for cell in action_cells
            )
        )
        self.assertEqual(
            [cell.label for cell in sorted(action_cells, key=lambda cell: cell.row)],
            [
                "KernelInitTask.Action::SetTaskState(Runnable)",
                "Scheduler.Action::SelectRunQueue(KernelInitTask)",
                "BootRunQueue.Action::EnqueueTask(KernelInitTask)",
            ],
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
