from __future__ import annotations

import contextlib
import io
import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[3]
TOOL_PATH = REPO_ROOT / "tools" / "checkpoints" / "summarize_linux_checkpoint_mapping.py"

spec = importlib.util.spec_from_file_location("summarize_linux_checkpoint_mapping", TOOL_PATH)
assert spec is not None
summarize_linux_checkpoint_mapping = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = summarize_linux_checkpoint_mapping
spec.loader.exec_module(summarize_linux_checkpoint_mapping)


def _mapping_rows() -> list[dict[str, object]]:
    return [
        {
            "checkpoint_index": 0,
            "checkpoint_name": "Kernel.Started",
            "checkpoint_variant": "KernelStarted",
            "linux_file": "init/main.c",
            "linux_symbol": "start_kernel",
            "linux_anchor": "start_kernel() definition line 1",
            "mapping_kind": "exact",
            "confidence": "high",
            "notes": "fixture",
        },
        {
            "checkpoint_index": 1,
            "checkpoint_name": "EntryPreludePhase.Started",
            "checkpoint_variant": "EntryPreludePhaseStarted",
            "linux_file": "arch/riscv/kernel/head.S",
            "linux_symbol": "_start",
            "linux_anchor": "_start definition line 1",
            "mapping_kind": "range",
            "confidence": "medium",
            "notes": "fixture",
        },
        {
            "checkpoint_index": 2,
            "checkpoint_name": "SyscallTable.OpenAt",
            "checkpoint_variant": "SyscallTableOpenAt",
            "linux_file": None,
            "linux_symbol": None,
            "linux_anchor": None,
            "mapping_kind": "unmapped",
            "confidence": "none",
            "notes": "fixture",
        },
        {
            "checkpoint_index": 3,
            "checkpoint_name": "SyscallTable.Read",
            "checkpoint_variant": "SyscallTableRead",
            "linux_file": None,
            "linux_symbol": None,
            "linux_anchor": None,
            "mapping_kind": "unmapped",
            "confidence": "none",
            "notes": "fixture",
        },
        {
            "checkpoint_index": 4,
            "checkpoint_name": "Scheduler.Tick",
            "checkpoint_variant": "SchedulerTick",
            "linux_file": None,
            "linux_symbol": None,
            "linux_anchor": None,
            "mapping_kind": "unmapped",
            "confidence": "none",
            "notes": "fixture",
        },
        {
            "checkpoint_index": 5,
            "checkpoint_name": "SoloBoundary",
            "checkpoint_variant": "SoloBoundary",
            "linux_file": None,
            "linux_symbol": None,
            "linux_anchor": None,
            "mapping_kind": "unmapped",
            "confidence": "none",
            "notes": "fixture",
        },
    ]


class SummarizeLinuxCheckpointMappingTests(unittest.TestCase):
    def _write_mapping(self, tmp: str) -> Path:
        path = Path(tmp) / "linux_checkpoint_mapping.json"
        path.write_text(json.dumps(_mapping_rows()), encoding="utf-8")
        return path

    def test_summary_counts_and_markdown_filter(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            records = summarize_linux_checkpoint_mapping.load_mapping(
                self._write_mapping(tmp)
            )
            coverage = summarize_linux_checkpoint_mapping.summarize_mapping(records)
            markdown = summarize_linux_checkpoint_mapping.render_markdown(coverage)

        self.assertEqual(coverage.total_checkpoints, 6)
        self.assertEqual(
            [(row.mapping_kind, row.count) for row in coverage.mapping_kind_counts],
            [("exact", 1), ("range", 1), ("unmapped", 4)],
        )
        self.assertEqual(
            [(row.confidence, row.count) for row in coverage.confidence_counts],
            [("none", 4), ("high", 1), ("medium", 1)],
        )
        self.assertEqual(
            [(row.linux_file, row.count) for row in coverage.mapped_linux_file_counts],
            [("arch/riscv/kernel/head.S", 1), ("init/main.c", 1)],
        )
        self.assertIn("| SyscallTable | 2 |", markdown)
        self.assertNotIn("| Scheduler | 1 |", markdown)
        self.assertNotIn("| SoloBoundary | 1 |", markdown)
        self.assertIn("singleton unmapped families: 2", markdown)

    def test_write_outputs_uses_aggregate_fields_only(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            records = summarize_linux_checkpoint_mapping.load_mapping(
                self._write_mapping(tmp)
            )
            coverage = summarize_linux_checkpoint_mapping.summarize_mapping(records)
            json_path, markdown_path = summarize_linux_checkpoint_mapping.write_outputs(
                coverage,
                Path(tmp) / "out",
            )
            data = json.loads(json_path.read_text(encoding="utf-8"))
            json_text = json_path.read_text(encoding="utf-8")
            markdown = markdown_path.read_text(encoding="utf-8")

        self.assertEqual(
            list(data.keys()),
            [
                "total_checkpoints",
                "mapping_kind_counts",
                "confidence_counts",
                "mapped_linux_file_counts",
                "unmapped_checkpoint_family_counts",
                "unmapped_singleton_family_count",
            ],
        )
        self.assertEqual(
            list(data["mapping_kind_counts"][0].keys()),
            ["mapping_kind", "count"],
        )
        self.assertNotIn("checkpoint_index", json_text)
        self.assertNotIn("checkpoint_name", json_text)
        self.assertNotIn("timestamp", json_text.lower())
        self.assertNotIn("timestamp", markdown.lower())
        self.assertIn("# Linux Checkpoint Mapping Coverage", markdown)

    def test_check_mode_accepts_current_outputs(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            mapping_path = self._write_mapping(tmp)
            records = summarize_linux_checkpoint_mapping.load_mapping(mapping_path)
            coverage = summarize_linux_checkpoint_mapping.summarize_mapping(records)
            out_dir = Path(tmp) / "out"
            summarize_linux_checkpoint_mapping.write_outputs(coverage, out_dir)

            stdout = io.StringIO()
            with contextlib.redirect_stdout(stdout):
                rc = summarize_linux_checkpoint_mapping.main(
                    [
                        "--input",
                        str(mapping_path),
                        "--out-dir",
                        str(out_dir),
                        "--check",
                    ]
                )

        self.assertEqual(rc, 0)
        self.assertIn("artifacts are current", stdout.getvalue())

    def test_check_mode_reports_drift_without_rewriting_outputs(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            mapping_path = self._write_mapping(tmp)
            records = summarize_linux_checkpoint_mapping.load_mapping(mapping_path)
            coverage = summarize_linux_checkpoint_mapping.summarize_mapping(records)
            out_dir = Path(tmp) / "out"
            _json_path, markdown_path = summarize_linux_checkpoint_mapping.write_outputs(
                coverage,
                out_dir,
            )
            stale_markdown = "# stale\n"
            markdown_path.write_text(stale_markdown, encoding="utf-8")

            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                rc = summarize_linux_checkpoint_mapping.main(
                    [
                        "--input",
                        str(mapping_path),
                        "--out-dir",
                        str(out_dir),
                        "--check",
                    ]
                )
            markdown_after_check = markdown_path.read_text(encoding="utf-8")

        self.assertEqual(rc, 1)
        self.assertEqual(markdown_after_check, stale_markdown)
        self.assertIn("content differs", stderr.getvalue())

    def test_real_mapping_summary_uses_tracked_mapping(self) -> None:
        records = summarize_linux_checkpoint_mapping.load_mapping()
        coverage = summarize_linux_checkpoint_mapping.summarize_mapping(records)

        self.assertEqual(coverage.total_checkpoints, len(records))
        self.assertGreater(coverage.total_checkpoints, 0)
        self.assertEqual(coverage.mapping_kind_counts[-1].mapping_kind, "unmapped")


if __name__ == "__main__":
    unittest.main()
