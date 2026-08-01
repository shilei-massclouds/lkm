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
TOOL_PATH = REPO_ROOT / "tools" / "checkpoints" / "list_checkpoints.py"

spec = importlib.util.spec_from_file_location("list_checkpoints", TOOL_PATH)
assert spec is not None
list_checkpoints = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = list_checkpoints
spec.loader.exec_module(list_checkpoints)


FIXTURE = """
pub enum Checkpoint {
    AlphaStarted,
    #[allow(dead_code)]
    BetaReady,
    GammaOnline,
}

impl Checkpoint {
    pub(crate) const fn early_byte(self) -> u8 {
        match self {
            Self::AlphaStarted => b'A',
            Self::GammaOnline => b'9',
            _ => b'?',
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::AlphaStarted => "Alpha.Started",
            Self::BetaReady => {
                "Beta.Ready"
            }
            Self::GammaOnline => "Gamma.Online",
        }
    }
}
"""


class ListCheckpointsTests(unittest.TestCase):
    def _write_fixture(self, tmp: str) -> Path:
        source = Path(tmp) / "mod.rs"
        source.write_text(FIXTURE, encoding="utf-8")
        return source

    def test_fixture_preserves_enum_order_and_mappings(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            records = list_checkpoints.parse_checkpoints(self._write_fixture(tmp))

        self.assertEqual(
            [(record.index, record.variant, record.name, record.early_byte) for record in records],
            [
                (0, "AlphaStarted", "Alpha.Started", "A"),
                (1, "BetaReady", "Beta.Ready", None),
                (2, "GammaOnline", "Gamma.Online", "9"),
            ],
        )

    def test_output_json_uses_fixed_record_fields(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            source = self._write_fixture(tmp)
            records = list_checkpoints.parse_checkpoints(source)
            out_dir = Path(tmp) / "out"
            json_path, markdown_path = list_checkpoints.write_outputs(records, out_dir)

            rows = json.loads(json_path.read_text(encoding="utf-8"))
            self.assertEqual(
                list(rows[0].keys()),
                ["index", "variant", "name", "early_byte", "source_file"],
            )
            self.assertIsNone(rows[1]["early_byte"])
            markdown = markdown_path.read_text(encoding="utf-8")
            self.assertIn("| 0 | AlphaStarted | Alpha.Started | A |", markdown)
            self.assertIn("| 1 | BetaReady | Beta.Ready | null |", markdown)

    def test_check_mode_accepts_current_outputs(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            source = self._write_fixture(tmp)
            records = list_checkpoints.parse_checkpoints(source)
            out_dir = Path(tmp) / "out"
            list_checkpoints.write_outputs(records, out_dir)

            stdout = io.StringIO()
            with contextlib.redirect_stdout(stdout):
                rc = list_checkpoints.main(
                    ["--source", str(source), "--out-dir", str(out_dir), "--check"]
                )

        self.assertEqual(rc, 0)
        self.assertIn("artifacts are current", stdout.getvalue())

    def test_check_mode_reports_drift_without_rewriting_outputs(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            source = self._write_fixture(tmp)
            records = list_checkpoints.parse_checkpoints(source)
            out_dir = Path(tmp) / "out"
            _json_path, markdown_path = list_checkpoints.write_outputs(records, out_dir)
            stale_markdown = "# stale\n"
            markdown_path.write_text(stale_markdown, encoding="utf-8")

            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                rc = list_checkpoints.main(
                    ["--source", str(source), "--out-dir", str(out_dir), "--check"]
                )

            markdown_after_check = markdown_path.read_text(encoding="utf-8")

        self.assertEqual(rc, 1)
        self.assertEqual(markdown_after_check, stale_markdown)
        self.assertIn("content differs", stderr.getvalue())

    def test_real_trace_file_contains_expected_checkpoints(self) -> None:
        records = list_checkpoints.parse_checkpoints()
        by_name = {record.name: record for record in records}

        self.assertIn("Kernel.Started", by_name)
        self.assertIn("Kernel.Online", by_name)
        self.assertIn("BootTask.OnCpu", by_name)
        self.assertIn("BootInitFlow.Started", by_name)
        self.assertNotIn("EntryPreludePhase.Started", by_name)
        self.assertIn("PayloadPreparePhase.Online", by_name)
        self.assertIn("PayloadHandoffPreparePhase.Online", by_name)
        self.assertIn("KernelInitFlow.PayloadHandoffCommitted", by_name)
        self.assertEqual(by_name["Kernel.Started"].variant, "KernelStarted")
        self.assertEqual(by_name["Kernel.Online"].early_byte, "L")
        self.assertEqual(by_name["BootTask.OnCpu"].early_byte, "T")
        self.assertEqual(by_name["BootInitFlow.Started"].early_byte, "O")
        self.assertFalse(any(name.startswith("EntrySuccessorPhase.") for name in by_name))
        for phase in (
            "CorePreparePhase",
            "MmCoreInitPhase",
            "SchedInitPhase",
            "IrqTimeInitPhase",
            "LocalIrqEnablePhase",
            "IrqOpenPreparePhase",
            "ProcessPreparePhase",
            "BootInitFlow",
            "BootInitRestInitPhase",
            "BootInitScheduleHandoffPhase",
            "BootIdleEntryPhase",
            "PreSmpInitPhase",
            "SmpBringupPhase",
            "ApEntryPreludePhase",
            "ApSmpCallinPhase",
            "ApOnlineIdlePhase",
            "RuntimeCorePhase",
            "InitcallPhase",
            "RootfsPhase",
            "FinalizePhase",
            "PayloadPreparePhase",
            "PayloadHandoffPreparePhase",
        ):
            for boundary in ("Started", "Prepared", "Ready", "Online"):
                self.assertIn(f"{phase}.{boundary}", by_name)

        for removed in ("BootPhase", "InterruptPhase", "SmpRuntimePhase", "PayloadPhase"):
            for boundary in ("Started", "Prepared", "Ready", "Online"):
                self.assertNotIn(f"{removed}.{boundary}", by_name)

        self.assertEqual([record.index for record in records], list(range(len(records))))
        self.assertEqual(len(records), 474)


if __name__ == "__main__":
    unittest.main()
