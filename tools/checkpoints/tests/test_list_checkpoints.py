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
        self.assertIn("BootTask.Online", by_name)
        self.assertIn("BootInitFlow.Started", by_name)
        self.assertIn("BootPhase.Started", by_name)
        self.assertIn("EntryPreludePhase.Started", by_name)
        self.assertIn("PayloadPhase.Online", by_name)
        self.assertEqual(by_name["Kernel.Started"].variant, "KernelStarted")
        self.assertEqual(by_name["BootTask.Online"].early_byte, "T")
        self.assertEqual(by_name["BootInitFlow.Started"].early_byte, "O")
        self.assertEqual(by_name["BootPhase.Started"].early_byte, "B")
        self.assertEqual(
            by_name["EntryPreludePhase.Started"].early_byte,
            "A",
        )

        for phase in (
            "BootPhase",
            "EntryPreludePhase",
            "EntrySuccessorPhase",
            "CorePreparePhase",
            "MmCoreInitPhase",
            "SchedInitPhase",
            "InterruptPhase",
            "IrqTimeInitPhase",
            "LocalIrqEnablePhase",
            "IrqOpenPreparePhase",
            "ProcessPreparePhase",
            "BootInitFlow",
            "BootInitRestInitPhase",
            "BootInitScheduleHandoffPhase",
            "BootIdleEntryPhase",
            "SmpRuntimePhase",
            "PreSmpInitPhase",
            "SmpBringupPhase",
            "ApEntryPreludePhase",
            "ApSmpCallinPhase",
            "ApOnlineIdlePhase",
            "RuntimeCorePhase",
            "InitcallPhase",
            "RootfsPhase",
            "FinalizePhase",
            "PayloadPhase",
        ):
            for boundary in ("Started", "Prepared", "Ready", "Online"):
                self.assertIn(f"{phase}.{boundary}", by_name)

        stable_interrupt_ids = {
            "InterruptPhaseStarted": 8,
            "InterruptPhaseReady": 9,
            "IrqTimeInitPhaseStarted": 173,
            "IrqTimeInitPhaseReady": 174,
            "IrqTimeInitPhaseOnline": 175,
            "LocalIrqEnablePhaseStarted": 210,
            "LocalIrqEnablePhaseReady": 211,
            "IrqOpenPreparePhaseStarted": 213,
            "IrqOpenPreparePhaseReady": 214,
            "ProcessPreparePhaseStarted": 232,
            "ProcessPreparePhaseReady": 233,
        }
        boot_init_ids = {
            "BootTaskOnline": 1,
            "BootInitFlowStarted": 2,
            "BootInitFlowPrepared": 267,
            "BootInitFlowReady": 268,
            "BootInitRestInitPhaseReady": 269,
            "BootInitScheduleHandoffPhaseReady": 270,
            "BootIdleEntryPhaseReady": 271,
            "BootInitFlowOnline": 449,
        }
        stable_smp_runtime_ids = {
            "PreSmpInitPhaseStarted": 293,
            "PreSmpInitPhaseReady": 294,
            "SmpRuntimePhaseStarted": 302,
            "SmpRuntimePhaseReady": 303,
            "SmpBringupPhaseStarted": 304,
            "SmpBringupPhaseReady": 305,
            "RuntimeCorePhaseStarted": 339,
            "RuntimeCorePhaseReady": 340,
            "InitcallPhaseStarted": 347,
            "InitcallPhaseReady": 348,
            "RootfsPhaseStarted": 368,
            "RootfsPhaseReady": 369,
            "FinalizePhaseStarted": 378,
            "FinalizePhaseReady": 379,
        }
        stable_ap_phase_ids = {
            "ApEntryPreludePhaseStarted": 320,
            "ApEntryPreludeBootDataConsumed": 321,
            "ApEntryPreludeCurrentStackEstablished": 322,
            "ApEntryPreludePhaseReady": 323,
            "ApSmpCallinPhaseStarted": 324,
            "ApSmpCallinCpuRunningProduced": 325,
            "ApSmpCallinPhaseReady": 326,
            "ApOnlineIdlePhaseStarted": 327,
            "ApOnlineIdleDoneUpProduced": 328,
            "ApOnlineIdlePhaseReady": 329,
        }
        appended_smp_runtime_ids = {
            "SmpRuntimePhasePrepared": 459,
            "SmpRuntimePhaseOnline": 460,
            "PreSmpInitPhasePrepared": 461,
            "PreSmpInitPhaseOnline": 462,
            "SmpBringupPhasePrepared": 463,
            "SmpBringupPhaseOnline": 464,
            "RuntimeCorePhasePrepared": 465,
            "RuntimeCorePhaseOnline": 466,
            "InitcallPhasePrepared": 467,
            "InitcallPhaseOnline": 468,
            "RootfsPhasePrepared": 469,
            "RootfsPhaseOnline": 470,
            "FinalizePhasePrepared": 471,
            "FinalizePhaseOnline": 472,
        }
        appended_ap_phase_ids = {
            "ApEntryPreludePhasePrepared": 473,
            "ApEntryPreludePhaseOnline": 474,
            "ApSmpCallinPhasePrepared": 475,
            "ApSmpCallinPhaseOnline": 476,
            "ApOnlineIdlePhasePrepared": 477,
            "ApOnlineIdlePhaseOnline": 478,
        }
        stable_payload_ids = {
            "PayloadPhaseReady": 431,
            "PayloadPhaseOnline": 432,
        }
        appended_payload_ids = {
            "PayloadPhaseStarted": 479,
            "PayloadPhasePrepared": 480,
        }
        appended_user_stack_ids = {
            "UserStackGrowComplete": 481,
            "UserStackGrowRejected": 482,
        }
        appended_task_flow_ids = {
            "KernelInitFlowOffline": 483,
            "KernelInitFlowDestroyed": 484,
            "UserAppFlowPrepared": 485,
            "UserAppFlowReady": 486,
            "UserAppFlowOnline": 487,
        }
        by_variant = {record.variant: record for record in records}
        self.assertEqual(
            {
                variant: by_variant[variant].index
                for variant in stable_interrupt_ids
            },
            stable_interrupt_ids,
        )
        self.assertEqual(
            {
                variant: by_variant[variant].index
                for variant in boot_init_ids
            },
            boot_init_ids,
        )
        self.assertEqual(
            {
                variant: by_variant[variant].index
                for variant in stable_smp_runtime_ids
            },
            stable_smp_runtime_ids,
        )
        self.assertEqual(
            {
                variant: by_variant[variant].index
                for variant in stable_ap_phase_ids
            },
            stable_ap_phase_ids,
        )
        self.assertEqual(
            {
                variant: by_variant[variant].index
                for variant in appended_smp_runtime_ids
            },
            appended_smp_runtime_ids,
        )
        self.assertEqual(
            {
                variant: by_variant[variant].index
                for variant in appended_ap_phase_ids
            },
            appended_ap_phase_ids,
        )
        self.assertEqual(
            {
                variant: by_variant[variant].index for variant in stable_payload_ids
            },
            stable_payload_ids,
        )
        self.assertEqual(
            {
                variant: by_variant[variant].index for variant in appended_payload_ids
            },
            appended_payload_ids,
        )
        self.assertEqual(
            {
                variant: by_variant[variant].index for variant in appended_user_stack_ids
            },
            appended_user_stack_ids,
        )
        self.assertEqual(
            {
                variant: by_variant[variant].index for variant in appended_task_flow_ids
            },
            appended_task_flow_ids,
        )
        self.assertEqual(len(records), 488)


if __name__ == "__main__":
    unittest.main()
