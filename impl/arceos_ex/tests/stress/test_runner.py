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
            "etc         lost+found  opt\n"
            "user exit status=0\n"
        )
        self.assertEqual(
            [runner._event_token(event) for event in events],
            [
                "user_output:DistroLsRootListing",
                "user_exit:UserExitStatus:status=0",
            ],
        )

    def test_extracts_legacy_wait4_handoff_event(self) -> None:
        events = runner._extract_events("wait4 child handoff sepc=0x1\n")
        self.assertEqual(
            [runner._event_token(event) for event in events],
            ["boundary:Wait4ChildHandoff"],
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

    def test_extracts_early_byte_checkpoint_events(self) -> None:
        events = runner._extract_events("AV9\n")
        self.assertEqual(
            [runner._event_token(event) for event in events],
            [
                "checkpoint:EntryPreludePhase.Started",
                "checkpoint:EventStream.Prepared",
                "checkpoint:ExceptionStream.Prepared",
            ],
        )
        self.assertEqual(events[0]["source"], "early-byte")

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

    def test_uses_stress_mem_text_with_early_byte_prefix(self) -> None:
        text = "checkpoint: EntryPreludePhase.Ready\n"
        size = len(text.encode())
        line = (
            f"AIKOZHTSstress_mem: v=1 encoding=hex bytes={size} total={size} "
            f"overflow=0 dropped=0 data={text.encode().hex()}\n"
        )
        observed, stress_mem = runner._observed_text(line)

        self.assertEqual(observed, "AIKOZHTS\n" + text)
        self.assertIsNotNone(stress_mem)
        events = runner._extract_events(observed)
        self.assertEqual(
            runner._event_token(events[0]),
            "checkpoint:EntryPreludePhase.Started",
        )

    def test_uses_stress_mem_text_with_interleaved_prompt_echo(self) -> None:
        text = (
            "checkpoint: EntryPreludePhase.Started\n"
            "checkpoint: PayloadPhase.Ready\n"
        )
        size = len(text.encode())
        encoded = text.encode().hex()
        split = len("checkpoint: EntryPrelude".encode().hex())
        line = (
            f"stress_mem: v=1 encoding=hex bytes={size} total={size} "
            f"overflow=0 dropped=0 data={encoded[:split]}~ # exit\n"
            f"{encoded[split:]}\n"
            "[    1.548514] ---[ end Kernel panic ]---\n"
        )
        observed, stress_mem = runner._observed_text(line)

        self.assertEqual(observed, text)
        self.assertIsNotNone(stress_mem)

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

    def test_capture_until_stress_mem_terminates_process(self) -> None:
        text = "checkpoint: EntryPreludePhase.Ready\n"
        line = (
            "stress_mem: v=1 encoding=hex "
            f"bytes={len(text.encode())} total={len(text.encode())} "
            f"overflow=0 dropped=0 data={text.encode().hex()}"
        )
        script = (
            "import sys, time\n"
            f"print({line!r}, flush=True)\n"
            "time.sleep(30)\n"
        )
        stdout, _returncode, timed_out, capture = runner._run_command_capture_until_stress_mem(
            [sys.executable, "-c", script],
            Path.cwd(),
            {},
            5,
        )
        self.assertFalse(timed_out)
        self.assertTrue(capture["terminated_after_stress_mem"])
        self.assertIn("stress_mem: v=1", stdout)

    def test_capture_until_stress_mem_waits_for_complete_hex_data(self) -> None:
        text = "checkpoint: A\ncheckpoint: B\n"
        encoded = text.encode().hex()
        split = len("checkpoint: A\n".encode().hex())
        prefix = (
            "stress_mem: v=1 encoding=hex "
            f"bytes={len(text.encode())} total={len(text.encode())} "
            "overflow=0 dropped=0 data="
        )
        script = (
            "import sys, time\n"
            f"sys.stdout.write({(prefix + encoded[:split])!r})\n"
            "sys.stdout.flush()\n"
            "time.sleep(0.2)\n"
            f"print({encoded[split:]!r}, flush=True)\n"
            "time.sleep(30)\n"
        )
        stdout, _returncode, timed_out, capture = runner._run_command_capture_until_stress_mem(
            [sys.executable, "-c", script],
            Path.cwd(),
            {},
            5,
        )
        observed, stress_mem = runner._observed_text(stdout)

        self.assertFalse(timed_out)
        self.assertTrue(capture["terminated_after_stress_mem"])
        self.assertEqual(observed, text)
        self.assertIsNotNone(stress_mem)

    def test_capture_until_stress_mem_supports_delayed_stdin(self) -> None:
        script = (
            "import sys, time\n"
            "print('/ #', flush=True)\n"
            "line = sys.stdin.readline().strip()\n"
            "text = f'checkpoint: {line}\\n'\n"
            "encoded = text.encode().hex()\n"
            "print(\n"
            "    f'stress_mem: v=1 encoding=hex bytes={len(text.encode())} '\n"
            "    f'total={len(text.encode())} overflow=0 dropped=0 data={encoded}',\n"
            "    flush=True,\n"
            ")\n"
            "time.sleep(30)\n"
        )
        stdout, _returncode, timed_out, capture = runner._run_command_capture_until_stress_mem(
            [sys.executable, "-c", script],
            Path.cwd(),
            {},
            5,
            runner.DelayedStdin(ready_marker="/ #", payload="PayloadPhase.Online\n"),
        )
        observed, stress_mem = runner._observed_text(stdout)

        self.assertFalse(timed_out)
        self.assertTrue(capture["terminated_after_stress_mem"])
        self.assertTrue(capture["stdin_sent"])
        self.assertEqual(capture["stdin_payload_bytes"], len("PayloadPhase.Online\n".encode()))
        self.assertEqual(observed, "checkpoint: PayloadPhase.Online\n")
        self.assertIsNotNone(stress_mem)

    def test_capture_until_stress_mem_tolerates_interleaved_prompt_echo(self) -> None:
        text = "checkpoint: PayloadPhase.Ready\n"
        encoded = text.encode().hex()
        split = len("checkpoint: Payload".encode().hex())
        prefix = (
            "stress_mem: v=1 encoding=hex "
            f"bytes={len(text.encode())} total={len(text.encode())} "
            "overflow=0 dropped=0 data="
        )
        script = (
            "import sys, time\n"
            f"sys.stdout.write({(prefix + encoded[:split] + '~ # exit\n' + encoded[split:])!r})\n"
            "sys.stdout.flush()\n"
            "time.sleep(30)\n"
        )
        stdout, _returncode, timed_out, capture = runner._run_command_capture_until_stress_mem(
            [sys.executable, "-c", script],
            Path.cwd(),
            {},
            5,
        )
        observed, stress_mem = runner._observed_text(stdout)

        self.assertFalse(timed_out)
        self.assertTrue(capture["terminated_after_stress_mem"])
        self.assertEqual(observed, text)
        self.assertIsNotNone(stress_mem)

    def test_paired_config_parses_linux_build_command(self) -> None:
        case = {
            "paired": {
                "checkpoint_scope": ["EntryPreludePhase.Started"],
                "checkpoint_scope_max_counts": {"UserExec.MainElfReady": 2},
                "arceos_ex": {"command": ["make", "run"]},
                "linux": {
                    "working_directory": "../linux-6.12",
                    "build_command": [
                        "make",
                        "ARCH=riscv",
                        "CROSS_COMPILE=riscv64-linux-gnu-",
                        "-j",
                        "$(nproc)",
                    ],
                    "command": ["qemu-system-riscv64"],
                    "stop_after_stress_mem": True,
                },
            }
        }
        config = runner._paired_config(case, Path("case.toml"), Path.cwd(), Path.cwd())
        self.assertEqual(config["checkpoint_scope"], ["EntryPreludePhase.Started"])
        self.assertEqual(config["checkpoint_scope_max_counts"], {"UserExec.MainElfReady": 2})
        self.assertIn("ARCH=riscv", config["linux"]["build_command"])
        self.assertTrue(config["linux"]["stop_after_stress_mem"])

    def test_rc_local_difftest_config_preserves_dotted_checkpoint_count_keys(self) -> None:
        arceos_ex_root = Path(__file__).resolve().parents[2]
        repo_root = arceos_ex_root.parents[1]
        case_path = arceos_ex_root / "tests/stress/cases/rc-local-difftest.toml"
        case = runner._load_toml(case_path)

        config = runner._paired_config(case, case_path, repo_root, repo_root)

        self.assertEqual(config["checkpoint_scope_max_counts"]["UserExec.MainElfReady"], 2)
        self.assertEqual(config["checkpoint_scope_max_counts"]["UserExec.TrapFrameReady"], 2)

    def test_paired_stress_mem_parses_both_sides(self) -> None:
        arceos_text = "AV9\ncheckpoint: TrampolineVm.Online\n"
        linux_text = (
            "checkpoint: EntryPreludePhase.Started\n"
            "checkpoint: EventStream.Prepared\n"
            "checkpoint: ExceptionStream.Prepared\n"
            "checkpoint: TrampolineVm.Online\n"
        )
        arceos_line = (
            "stress_mem: v=1 encoding=hex "
            f"bytes={len(arceos_text.encode())} total={len(arceos_text.encode())} "
            f"overflow=0 dropped=0 data={arceos_text.encode().hex()}"
        )
        linux_line = (
            "stress_mem: v=1 encoding=hex "
            f"bytes={len(linux_text.encode())} total={len(linux_text.encode())} "
            f"overflow=0 dropped=0 data={linux_text.encode().hex()}"
        )
        arceos_observed, _ = runner._observed_text(arceos_line)
        linux_observed, _ = runner._observed_text(linux_line)
        diff = runner._paired_checkpoint_diff(
            runner._extract_events(arceos_observed),
            runner._extract_events(linux_observed),
            [
                "EntryPreludePhase.Started",
                "EventStream.Prepared",
                "ExceptionStream.Prepared",
                "TrampolineVm.Online",
            ],
            left_label="arceos_ex",
            right_label="linux",
        )
        self.assertTrue(diff["passed"])

    def test_paired_checkpoint_diff_ignores_left_extra_outside_scope(self) -> None:
        left = runner._extract_events(
            "checkpoint: EntryPreludePhase.Started\n"
            "checkpoint: Internal.Only\n"
            "checkpoint: EntryPreludePhase.Ready\n"
        )
        right = runner._extract_events(
            "checkpoint: EntryPreludePhase.Started\n"
            "checkpoint: EntryPreludePhase.Ready\n"
        )
        diff = runner._paired_checkpoint_diff(
            left,
            right,
            ["EntryPreludePhase.Started", "EntryPreludePhase.Ready"],
            left_label="arceos_ex",
            right_label="linux",
        )
        self.assertTrue(diff["passed"])
        observed = diff["observed_but_not_compared"]
        self.assertEqual(observed["arceos_ex"][0]["name"], "Internal.Only")
        self.assertEqual(observed["arceos_ex"][0]["excluded_reason"], "outside_checkpoint_scope")
        self.assertEqual(observed["linux"], [])

    def test_paired_checkpoint_diff_counts_duplicate_outside_scope_events(self) -> None:
        left = runner._extract_events(
            "checkpoint: A\n"
            "checkpoint: SyscallTable.Read\n"
            "checkpoint: SyscallTable.Read\n"
        )
        right = runner._extract_events(
            "checkpoint: A\n"
            "checkpoint: SyscallTable.Write\n"
        )
        diff = runner._paired_checkpoint_diff(
            left,
            right,
            ["A"],
            left_label="arceos_ex",
            right_label="linux",
        )

        self.assertTrue(diff["passed"])
        self.assertEqual(
            diff["observed_but_not_compared"]["arceos_ex"],
            [
                {
                    "name": "SyscallTable.Read",
                    "count": 2,
                    "first_line": 2,
                    "excluded_reason": "outside_checkpoint_scope",
                }
            ],
        )
        self.assertEqual(diff["observed_but_not_compared"]["linux"][0]["count"], 1)

    def test_paired_checkpoint_diff_limits_scoped_checkpoint_counts(self) -> None:
        left = runner._extract_events(
            "checkpoint: A\n"
            "checkpoint: UserExec.MainElfReady\n"
            "checkpoint: UserExec.MainElfReady\n"
        )
        right = runner._extract_events(
            "checkpoint: A\n"
            "checkpoint: UserExec.MainElfReady\n"
            "checkpoint: UserExec.MainElfReady\n"
            "checkpoint: UserExec.MainElfReady\n"
        )
        diff = runner._paired_checkpoint_diff(
            left,
            right,
            ["A", "UserExec.MainElfReady", "UserExec.MainElfReady"],
            checkpoint_scope_max_counts={"UserExec.MainElfReady": 2},
            left_label="arceos_ex",
            right_label="linux",
        )

        self.assertTrue(diff["passed"])
        observed = diff["observed_but_not_compared"]
        self.assertEqual(observed["linux"][0]["name"], "UserExec.MainElfReady")
        self.assertEqual(observed["linux"][0]["excluded_reason"], "scope_count_limit")

    def test_report_lists_observed_but_not_compared_checkpoints(self) -> None:
        diff = runner._paired_checkpoint_diff(
            runner._extract_events("checkpoint: A\ncheckpoint: SyscallTable.Read\n"),
            runner._extract_events("checkpoint: A\n"),
            ["A"],
            left_label="arceos_ex",
            right_label="linux",
        )
        with tempfile.TemporaryDirectory() as tmp:
            report = Path(tmp) / "report.md"
            runner._write_report(
                report,
                "case",
                {
                    "dry_run": False,
                    "requested_runs": 1,
                    "completed_runs": 1,
                    "started_at": "now",
                    "ended_at": "later",
                    "total_seconds": 1.0,
                    "average_run_seconds": 1.0,
                    "totals": {"success": 1, "failure": 0},
                    "classes": [],
                    "failure_vs_success": [],
                    "paired_checkpoint_diff": [diff],
                },
            )
            text = report.read_text(encoding="utf-8")

        self.assertIn("observed_but_not_compared", text)
        self.assertIn("SyscallTable.Read x1 (outside_checkpoint_scope)", text)

    def test_paired_checkpoint_diff_reports_missing_extra_and_order(self) -> None:
        scope = ["A", "B"]
        missing = runner._paired_checkpoint_diff(
            runner._extract_events("checkpoint: A\ncheckpoint: B\n"),
            runner._extract_events("checkpoint: A\n"),
            scope,
            left_label="arceos_ex",
            right_label="linux",
        )
        self.assertFalse(missing["passed"])
        self.assertEqual(missing["missing_from_linux"], ["B"])

        extra = runner._paired_checkpoint_diff(
            runner._extract_events("checkpoint: A\n"),
            runner._extract_events("checkpoint: A\ncheckpoint: B\n"),
            scope,
            left_label="arceos_ex",
            right_label="linux",
        )
        self.assertFalse(extra["passed"])
        self.assertEqual(extra["extra_in_linux"], ["B"])

        order = runner._paired_checkpoint_diff(
            runner._extract_events("checkpoint: A\ncheckpoint: B\n"),
            runner._extract_events("checkpoint: B\ncheckpoint: A\n"),
            scope,
            left_label="arceos_ex",
            right_label="linux",
        )
        self.assertFalse(order["passed"])
        self.assertTrue(order["order_mismatch"])
        self.assertEqual(order["first_divergence"]["index"], 0)

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
