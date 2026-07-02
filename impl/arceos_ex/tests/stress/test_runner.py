#!/usr/bin/env python3
"""Unit tests for the stress runner."""

from __future__ import annotations

from datetime import datetime, timezone
import sys
import tempfile
from pathlib import Path
import unittest

import runner


class StressRunnerTests(unittest.TestCase):
    def test_default_selection_uses_standard_suite(self) -> None:
        selected = runner._selected_case_paths([])
        self.assertEqual(
            [path.name for path in selected],
            [
                "df-0001-user-boot.toml",
                "df-0002-smoke-initcall.toml",
                "df-0003-distro-sh-ls.toml",
            ],
        )

    def test_explicit_selection_replaces_default_suite(self) -> None:
        selected = runner._selected_case_paths(
            [Path("impl/arceos_ex/tests/stress/cases/df-0002-smoke-initcall.toml")]
        )
        self.assertEqual(len(selected), 1)
        self.assertEqual(selected[0].name, "df-0002-smoke-initcall.toml")

    def test_case_failed_reads_failure_total(self) -> None:
        self.assertFalse(runner._case_failed({"summary": {"totals": {"failure": 0}}}))
        self.assertTrue(runner._case_failed({"summary": {"totals": {"failure": 1}}}))

    def test_extracts_user_boot_success_events(self) -> None:
        events = runner._extract_events("noise\nuser hello\nuser exit status=0\n")
        self.assertEqual(
            [runner._event_token(event) for event in events],
            ["user_output:UserHello", "user_exit:UserExitStatus:status=0"],
        )

    def test_extracts_distro_shell_ls_events(self) -> None:
        events = runner._extract_events(
            "wait4 child handoff sepc=0x1\n"
            "etc         lost+found  opt\n"
            "user exit status=0\n"
        )
        self.assertEqual(
            [runner._event_token(event) for event in events],
            [
                "boundary:Wait4ChildHandoff",
                "user_output:DistroLsRootListing",
                "user_exit:UserExitStatus:status=0",
            ],
        )

    def test_extracts_smoke_success_event(self) -> None:
        events = runner._extract_events("result: \x1b[32mok\x1b[0m. passed=78 failed=0 total=78\n")
        self.assertEqual(
            [runner._event_token(event) for event in events],
            ["smoke_result:SmokeResult:passed=78:failed=0:total=78"],
        )

    def test_extracts_checkpoint_announce_event(self) -> None:
        events = runner._extract_events("checkpoint: EarlyVm.Ready task=boot_idle\n")
        self.assertEqual(
            [runner._event_token(event) for event in events],
            ["checkpoint:EarlyVm.Ready"],
        )
        self.assertEqual(events[0]["source"], "announce")
        self.assertEqual(events[0]["task"], "boot_idle")

    def test_extracts_legacy_trace_checkpoint_event(self) -> None:
        events = runner._extract_events("trace: EarlyVm.Ready task=boot_idle\n")
        self.assertEqual(
            [runner._event_token(event) for event in events],
            ["checkpoint:EarlyVm.Ready"],
        )
        self.assertEqual(events[0]["source"], "legacy-trace")

    def test_extracts_ready_check_failed_event(self) -> None:
        events = runner._extract_events(
            "ready_check_failed phase=InitcallPhase "
            "check=initcall_phase_ready "
            "first_failed=platform_bus.ns16550a_probe_called\n"
        )
        self.assertEqual(
            [runner._event_token(event) for event in events],
            [
                "ready_check_failed:ReadyCheckFailed:phase=InitcallPhase:"
                "check=initcall_phase_ready:first_failed=platform_bus.ns16550a_probe_called"
            ],
        )

    def test_extracts_failure_diagnostic_event(self) -> None:
        events = runner._extract_events(
            "failure_diagnostic phase=InitcallPhase "
            "step=InitcallBoundary.setup object=TtyXmitFifoProbe "
            "check=tty_xmit_fifo_probe.no_overflow_observed "
            "first_failed=tty_xmit_fifo_probe.no_overflow_observed\n"
        )
        self.assertEqual(
            [runner._event_token(event) for event in events],
            [
                "failure_diagnostic:FailureDiagnostic:phase=InitcallPhase:"
                "step=InitcallBoundary.setup:object=TtyXmitFifoProbe:"
                "check=tty_xmit_fifo_probe.no_overflow_observed:"
                "first_failed=tty_xmit_fifo_probe.no_overflow_observed"
            ],
        )

    def test_extracts_df0001_failure_event(self) -> None:
        events = runner._extract_events("read user ELF failed\n")
        self.assertEqual(len(events), 1)
        self.assertEqual(events[0]["kind"], "symptom")
        self.assertEqual(events[0]["name"], "ReadUserElfFailed")

    def test_uses_stress_mem_text_when_present(self) -> None:
        text = "result: \x1b[32mok\x1b[0m. passed=2 failed=0 total=2\n"
        size = len(text.encode())
        line = (
            f"stress_mem: v=1 encoding=hex bytes={size} total={size} "
            f"overflow=0 dropped=0 data={text.encode().hex()}\n"
        )
        observed, stress_mem = runner._observed_text("host noise\n" + line)
        self.assertEqual(observed, text)
        self.assertIsNotNone(stress_mem)
        events = runner._extract_events(observed)
        self.assertEqual(
            [runner._event_token(event) for event in events],
            ["smoke_result:SmokeResult:passed=2:failed=0:total=2"],
        )

    def test_failure_rules_take_precedence_over_success_rules(self) -> None:
        rules = [
            {
                "id": "df-0001-read-user-elf-failed",
                "result": "failure",
                "contains": ["read user ELF failed"],
            },
            {
                "id": "user-boot-success",
                "result": "success",
                "contains": ["user exit status=0"],
            },
        ]
        result = runner._classify(
            "user exit status=0\nread user ELF failed\n",
            returncode=0,
            timed_out=False,
            rules=rules,
        )
        self.assertEqual(result["result"], "failure")
        self.assertEqual(result["id"], "df-0001-read-user-elf-failed")

    def test_classify_smoke_success_after_ansi_stripping(self) -> None:
        rules = [
            {
                "id": "smoke-success",
                "result": "success",
                "contains": ["result: ok. passed=", " failed=0 total="],
            },
        ]
        result = runner._classify(
            "result: \x1b[32mok\x1b[0m. passed=54 failed=0 total=54\n",
            returncode=0,
            timed_out=False,
            rules=rules,
        )
        self.assertEqual(result["result"], "success")
        self.assertEqual(result["id"], "smoke-success")

    def test_delayed_stdin_writes_after_marker(self) -> None:
        script = (
            "import sys\n"
            "print('/ #', flush=True)\n"
            "line = sys.stdin.readline()\n"
            "print('got=' + line.strip(), flush=True)\n"
        )
        stdout, returncode, timed_out, stdin_result = runner._run_command_capture(
            [sys.executable, "-c", script],
            Path.cwd(),
            {},
            5,
            runner.DelayedStdin(ready_marker="/ #", payload="ls\n"),
        )
        self.assertEqual(returncode, 0)
        self.assertFalse(timed_out)
        self.assertTrue(stdin_result["stdin_sent"])
        self.assertEqual(stdin_result["stdin_payload_bytes"], 3)
        self.assertIn("got=ls", stdout)

    def test_delayed_stdin_config_is_optional(self) -> None:
        self.assertIsNone(runner._delayed_stdin({}))
        config = runner._delayed_stdin(
            {"delayed_stdin": {"ready_marker": "ready", "payload": "input\n"}}
        )
        self.assertIsNotNone(config)
        assert config is not None
        self.assertEqual(config.ready_marker, "ready")
        self.assertEqual(config.payload, "input\n")

    def test_records_duplicate_sequence_once(self) -> None:
        events = runner._extract_events("user hello\nuser exit status=0\n")
        tokens = [runner._event_token(event) for event in events]
        run = {
            "result": "success",
            "class_id": "user-boot-success",
            "sequence_hash": runner._sequence_hash(tokens),
            "run_id": "run-0001",
            "sequence_tokens": tokens,
            "events_data": events,
        }
        sequences = {}
        runner._record_sequence(sequences, run)
        runner._record_sequence(sequences, {**run, "run_id": "run-0002"})
        self.assertEqual(len(sequences), 1)
        entry = next(iter(sequences.values()))
        self.assertEqual(entry["count"], 2)
        self.assertEqual(entry["first_run"], "run-0001")
        self.assertEqual(entry["run_ids"], ["run-0001", "run-0002"])

    def test_duplicate_stress_mem_sequence_saves_first_artifacts_only(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            events = runner._extract_events("result: ok. passed=1 failed=0 total=1\n")
            tokens = [runner._event_token(event) for event in events]
            sequence_hash = runner._sequence_hash(tokens)

            base = {
                "result": "success",
                "class_id": "smoke-success",
                "sequence_hash": sequence_hash,
                "sequence_tokens": tokens,
                "events_data": events,
                "captured_stdout": "stress_mem: ...\n",
                "stress_mem_text": "result: ok. passed=1 failed=0 total=1\n",
            }
            first = {
                **base,
                "run_id": "run-0001",
                "events_saved": False,
                "stdout_saved": False,
                "_events_candidate": str(root / "run-0001" / "events.first-seen.jsonl"),
                "_stdout_candidate": str(root / "run-0001" / "stdout.first-seen.log"),
            }
            second = {
                **base,
                "run_id": "run-0002",
                "events_saved": False,
                "stdout_saved": False,
                "_events_candidate": str(root / "run-0002" / "events.first-seen.jsonl"),
                "_stdout_candidate": str(root / "run-0002" / "stdout.first-seen.log"),
            }
            (root / "run-0001").mkdir()
            (root / "run-0002").mkdir()

            sequences = {}
            runner._record_sequence(sequences, first)
            runner._record_sequence(sequences, second)

            self.assertTrue(first["events_saved"])
            self.assertTrue(first["stdout_saved"])
            self.assertFalse(second["events_saved"])
            self.assertFalse(second["stdout_saved"])
            self.assertTrue((root / "run-0001" / "events.first-seen.jsonl").exists())
            self.assertTrue((root / "run-0001" / "stdout.first-seen.log").exists())
            self.assertFalse((root / "run-0002" / "events.first-seen.jsonl").exists())
            self.assertFalse((root / "run-0002" / "stdout.first-seen.log").exists())

    def test_summary_reports_total_and_average_time(self) -> None:
        summary = runner._build_summary(
            "case",
            2,
            [
                {
                    "result": "success",
                    "class_id": "ok",
                    "duration_seconds": 1.2,
                },
                {
                    "result": "failure",
                    "class_id": "bad",
                    "duration_seconds": 1.8,
                },
            ],
            {},
            dry_run=False,
            started=datetime(2026, 6, 27, 1, 2, 3, tzinfo=timezone.utc),
            ended=datetime(2026, 6, 27, 1, 2, 6, tzinfo=timezone.utc),
            duration_seconds=3.01,
        )

        self.assertEqual(summary["started_at"], "2026-06-27T01:02:03+00:00")
        self.assertEqual(summary["ended_at"], "2026-06-27T01:02:06+00:00")
        self.assertEqual(summary["total_seconds"], 3.01)
        self.assertEqual(summary["average_run_seconds"], 1.5)
        self.assertEqual(summary["completed_runs"], 2)

    def test_summary_average_time_is_none_without_runs(self) -> None:
        summary = runner._build_summary(
            "case",
            0,
            [],
            {},
            dry_run=True,
            started=datetime(2026, 6, 27, 1, 2, 3, tzinfo=timezone.utc),
            ended=datetime(2026, 6, 27, 1, 2, 3, tzinfo=timezone.utc),
            duration_seconds=0.0,
        )

        self.assertIsNone(summary["average_run_seconds"])
        self.assertEqual(summary["total_seconds"], 0.0)

    def test_report_scalar_formats_none_as_null(self) -> None:
        self.assertEqual(runner._report_scalar(None), "null")
        self.assertEqual(runner._report_scalar(1.25), "1.25")


if __name__ == "__main__":
    unittest.main()
