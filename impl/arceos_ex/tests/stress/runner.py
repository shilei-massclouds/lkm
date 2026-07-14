#!/usr/bin/env python3
"""Repeat stress cases and cluster observed event sequences."""

from __future__ import annotations

import argparse
from collections import Counter
from dataclasses import dataclass
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import selectors
import signal
import subprocess
import sys
import time
import tomllib
from typing import Any


SCHEMA_VERSION = 1
STRESS_DIR = Path(__file__).resolve().parent
DEFAULT_SUITE = (
    STRESS_DIR / "cases" / "df-0001-user-boot.toml",
    STRESS_DIR / "cases" / "df-0002-smoke-initcall.toml",
    STRESS_DIR / "cases" / "df-0003-distro-sh-ls.toml",
)
DEFAULT_OUT_ROOT = Path(__file__).resolve().parent / "out"
ANSI_RE = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")
CHECKPOINT_RE = re.compile(
    r"^(?P<prefix>checkpoint|trace): (?P<name>[^ ]+)(?: task=(?P<task>[^ ]+))?"
)
PHASE_ERROR_RE = re.compile(
    r"error=(?P<error>\S+) event=(?P<event>\S+) actual=(?P<actual>\S+) "
    r"expected=(?P<expected>\S+) target=(?P<target>\S+)"
)
READY_CHECK_FAILED_RE = re.compile(
    r"^ready_check_failed phase=(?P<phase>\S+) check=(?P<check>\S+) "
    r"first_failed=(?P<first_failed>\S+)"
)
FAILURE_DIAGNOSTIC_RE = re.compile(
    r"^failure_diagnostic phase=(?P<phase>\S+) step=(?P<step>\S+) "
    r"object=(?P<object>\S+) check=(?P<check>\S+) "
    r"first_failed=(?P<first_failed>\S+)"
)
SMOKE_RESULT_RE = re.compile(
    r"passed=(?P<passed>\d+) failed=(?P<failed>\d+) total=(?P<total>\d+)"
)
USER_EXIT_RE = re.compile(r"user exit status=(?P<status>-?\d+)")
STRESS_MEM_RE = re.compile(
    r"stress_mem: v=1 encoding=hex bytes=(?P<bytes>\d+) total=(?P<total>\d+) "
    r"overflow=(?P<overflow>[01]) dropped=(?P<dropped>\d+) data=(?P<data>[0-9a-f]*)$"
)
STRESS_MEM_HEADER_RE = re.compile(
    r"stress_mem: v=1 encoding=hex bytes=(?P<bytes>\d+) total=(?P<total>\d+) "
    r"overflow=(?P<overflow>[01]) dropped=(?P<dropped>\d+) data="
)
STRESS_MEM_PROMPT_ECHO_RE = re.compile(r"(?:~|/) # [^\r\n]*(?:\r?\n)?")
EARLY_CHECKPOINT_BYTES = {
    "R": "Kernel.Started",
    "B": "BootPhase.Started",
    "A": "EntryPreludePhase.Started",
    "I": "InterruptStream.Prepared",
    "K": "KernelImage.Prepared",
    "O": "RootStream.Prepared",
    "Z": "KernelImage.Ready",
    "H": "BootCPU.Prepared",
    "G": "CpuGroup.Prepared",
    "T": "InitTask.Prepared",
    "S": "InitStack.Prepared",
    "V": "EventStream.Prepared",
    "9": "ExceptionStream.Prepared",
    "Q": "TrampolineVm.Ready",
    "Y": "RawDtb.Prepared",
    "W": "RawDtb.Ready",
    "M": "FixMap.Ready",
    "N": "EarlyVm.Prepared",
    "J": "EarlyVm.Ready",
    "U": "Vm.Prepared",
}


@dataclass(frozen=True)
class DelayedStdin:
    ready_marker: str
    payload: str


class CheckpointCoverageError(Exception):
    def __init__(self, message: str, audit: dict[str, Any]) -> None:
        super().__init__(message)
        self.message = message
        self.audit = audit


def main(argv: list[str] | None = None) -> int:
    parser = _build_parser()
    args = parser.parse_args(argv)
    repo_root = _resolve_repo_root(args.repo_root)
    case_paths = _selected_case_paths(args.cases)
    out_root = args.out_dir.resolve() if args.out_dir else DEFAULT_OUT_ROOT

    results = [
        _run_case(
            case_path=case_path,
            repo_root=repo_root,
            out_root=out_root,
            runs_override=args.runs,
            timeout_override=args.timeout,
            dry_run=args.dry_run,
        )
        for case_path in case_paths
    ]
    if len(results) > 1:
        _print_suite_summary(results)
    return 1 if any(_case_failed(result) for result in results) else 0


def _run_case(
    *,
    case_path: Path,
    repo_root: Path,
    out_root: Path,
    runs_override: int | None,
    timeout_override: int | None,
    dry_run: bool,
) -> dict[str, Any]:
    case = _load_toml(case_path)
    mode = _optional_string(case, "mode")
    if mode is not None:
        if mode == "paired_checkpoint_diff":
            return _run_paired_case(
                case=case,
                case_path=case_path,
                repo_root=repo_root,
                out_root=out_root,
                runs_override=runs_override,
                timeout_override=timeout_override,
                dry_run=dry_run,
            )
        raise SystemExit(f"unknown stress case mode: {mode}")

    classifier_path = _resolve_case_path(case_path, _string(case, "classifier"))
    classifier = _load_toml(classifier_path)
    runs = runs_override if runs_override is not None else _integer(case, "default_runs")
    timeout = timeout_override if timeout_override is not None else _integer(case, "timeout_seconds")
    if runs < 0:
        raise SystemExit("--runs must be non-negative")
    if timeout <= 0:
        raise SystemExit("--timeout must be positive")

    case_name = _string(case, "name")
    output_dir = _unique_output_dir(out_root / _run_dir_name(case_name))
    output_dir.mkdir(parents=True, exist_ok=False)

    command = _string_list(case, "command")
    delayed_stdin = _delayed_stdin(case)
    workdir = _resolve_workdir(repo_root, case.get("working_directory", "."))
    rules = _classifier_rules(classifier)
    manifest = _manifest(
        case,
        case_path,
        classifier_path,
        command,
        delayed_stdin,
        runs,
        timeout,
        repo_root,
        workdir,
    )
    _write_json(output_dir / "manifest.json", manifest)

    sequences: dict[tuple[str, str, str], dict[str, Any]] = {}
    run_results: list[dict[str, Any]] = []
    suite_started = datetime.now(timezone.utc)
    suite_start_monotonic = time.monotonic()

    if runs == 0 or dry_run:
        suite_ended = datetime.now(timezone.utc)
        suite_duration = time.monotonic() - suite_start_monotonic
        summary = _build_summary(
            case_name,
            runs,
            run_results,
            sequences,
            dry_run=True,
            started=suite_started,
            ended=suite_ended,
            duration_seconds=suite_duration,
        )
        _write_json(output_dir / "summary.json", summary)
        _write_report(output_dir / "report.md", case_name, summary)
        print(f"stress dry-run wrote {output_dir}")
        return _case_result(case_name, case_path, output_dir, summary)

    setup_command = _optional_string_list(case, "setup_command")
    if setup_command:
        _run_setup_command(setup_command, repo_root, workdir, timeout, output_dir)

    for run_index in range(1, runs + 1):
        run_id = f"run-{run_index:04d}"
        run_dir = output_dir / "runs" / run_id
        run_dir.mkdir(parents=True)
        print(f"[stress] {case_name} {run_id}/{runs}")
        run_result = _execute_one_run(
            run_id=run_id,
            run_dir=run_dir,
            command=command,
            repo_root=repo_root,
            workdir=workdir,
            timeout=timeout,
            delayed_stdin=delayed_stdin,
            env_updates=_string_map(case.get("env", {}), "env"),
            rules=rules,
        )
        _record_sequence(sequences, run_result)
        _write_run_metadata(run_dir, run_result, repo_root)
        run_results.append(_persisted_run_result(run_result))

    _write_sequences(output_dir, sequences)
    suite_ended = datetime.now(timezone.utc)
    suite_duration = time.monotonic() - suite_start_monotonic
    summary = _build_summary(
        case_name,
        runs,
        run_results,
        sequences,
        dry_run=False,
        started=suite_started,
        ended=suite_ended,
        duration_seconds=suite_duration,
    )
    _write_json(output_dir / "summary.json", summary)
    _write_report(output_dir / "report.md", case_name, summary)
    print(f"stress report: {output_dir / 'report.md'}")
    return _case_result(case_name, case_path, output_dir, summary)


def _run_paired_case(
    *,
    case: dict[str, Any],
    case_path: Path,
    repo_root: Path,
    out_root: Path,
    runs_override: int | None,
    timeout_override: int | None,
    dry_run: bool,
) -> dict[str, Any]:
    runs = runs_override if runs_override is not None else _integer(case, "default_runs")
    timeout = timeout_override if timeout_override is not None else _integer(case, "timeout_seconds")
    if runs < 0:
        raise SystemExit("--runs must be non-negative")
    if timeout <= 0:
        raise SystemExit("--timeout must be positive")

    case_name = _string(case, "name")
    output_dir = _unique_output_dir(out_root / _run_dir_name(case_name))
    output_dir.mkdir(parents=True, exist_ok=False)
    base_workdir = _resolve_workdir(repo_root, case.get("working_directory", "."))
    try:
        paired = _paired_config(case, case_path, repo_root, base_workdir)
    except CheckpointCoverageError as error:
        _write_json(output_dir / "checkpoint_coverage_error.json", error.audit)
        print(error.message, file=sys.stderr)
        raise SystemExit(error.message) from error
    manifest = _paired_manifest(case, case_path, runs, timeout, repo_root, base_workdir, paired)
    _write_json(output_dir / "manifest.json", manifest)
    checkpoint_coverage = paired.get("checkpoint_coverage")

    sequences: dict[tuple[str, str, str], dict[str, Any]] = {}
    run_results: list[dict[str, Any]] = []
    suite_started = datetime.now(timezone.utc)
    suite_start_monotonic = time.monotonic()

    if runs == 0 or dry_run:
        suite_ended = datetime.now(timezone.utc)
        suite_duration = time.monotonic() - suite_start_monotonic
        summary = _build_summary(
            case_name,
            runs,
            run_results,
            sequences,
            dry_run=True,
            started=suite_started,
            ended=suite_ended,
            duration_seconds=suite_duration,
        )
        summary["paired_checkpoint_diff"] = []
        if checkpoint_coverage is not None:
            summary["checkpoint_coverage"] = _checkpoint_coverage_report(checkpoint_coverage)
        _write_json(output_dir / "summary.json", summary)
        _write_report(output_dir / "report.md", case_name, summary)
        print(f"stress dry-run wrote {output_dir}")
        return _case_result(case_name, case_path, output_dir, summary)

    setup_command = _optional_string_list(case, "setup_command")
    if setup_command:
        _run_setup_command(setup_command, repo_root, base_workdir, timeout, output_dir)

    linux_build = paired["linux"].get("build_command")
    if isinstance(linux_build, list) and linux_build:
        _run_setup_command(
            _expand_command_placeholders(_as_string_list(linux_build, "paired.linux.build_command")),
            repo_root,
            paired["linux"]["workdir"],
            timeout,
            output_dir,
            label="linux-build",
        )

    for run_index in range(1, runs + 1):
        run_id = f"run-{run_index:04d}"
        run_dir = output_dir / "runs" / run_id
        run_dir.mkdir(parents=True)
        print(f"[stress] {case_name} {run_id}/{runs}")
        started = datetime.now(timezone.utc)
        start_monotonic = time.monotonic()
        arceos = _execute_paired_side(
            side_id="arceos_ex",
            config=paired["arceos_ex"],
            run_dir=run_dir,
            timeout=timeout,
        )
        linux = _execute_paired_side(
            side_id="linux",
            config=paired["linux"],
            run_dir=run_dir,
            timeout=timeout,
        )
        ended = datetime.now(timezone.utc)
        duration = time.monotonic() - start_monotonic
        diff = _paired_checkpoint_diff(
            arceos["events_data"],
            linux["events_data"],
            paired["checkpoint_scope"],
            checkpoint_scope_max_counts=paired["checkpoint_scope_max_counts"],
            left_label="arceos_ex",
            right_label="linux",
        )
        if checkpoint_coverage is not None:
            diff["checkpoint_coverage"] = _checkpoint_coverage_report(checkpoint_coverage)
        side_failed = arceos["timed_out"] or linux["timed_out"] or linux["stress_mem"] is None
        passed = diff["passed"] and not side_failed
        sequence_tokens = [
            *(f"arceos_ex:{token}" for token in arceos["sequence_tokens"]),
            *(f"linux:{token}" for token in linux["sequence_tokens"]),
        ]
        result = {
            "schema_version": SCHEMA_VERSION,
            "run_id": run_id,
            "started_at": started.isoformat(),
            "ended_at": ended.isoformat(),
            "duration_seconds": round(duration, 3),
            "result": "success" if passed else "failure",
            "class_id": "paired-checkpoint-diff-ok" if passed else "paired-checkpoint-diff",
            "class_description": (
                "declared checkpoint intersection matched"
                if passed
                else "declared checkpoint intersection differed"
            ),
            "sequence_hash": _sequence_hash(sequence_tokens),
            "event_count": len(sequence_tokens),
            "arceos_ex": _persisted_side_result(arceos),
            "linux": _persisted_side_result(linux),
            "paired_diff": diff,
        }
        run = {
            **result,
            "events_data": _paired_events_for_sequence(arceos, linux, diff),
            "sequence_tokens": sequence_tokens,
        }
        _write_json(run_dir / "paired_diff.json", diff)
        _record_sequence(sequences, run)
        _write_run_metadata(run_dir, run, repo_root)
        run_results.append(_persisted_run_result(run))

    _write_sequences(output_dir, sequences)
    suite_ended = datetime.now(timezone.utc)
    suite_duration = time.monotonic() - suite_start_monotonic
    summary = _build_summary(
        case_name,
        runs,
        run_results,
        sequences,
        dry_run=False,
        started=suite_started,
        ended=suite_ended,
        duration_seconds=suite_duration,
    )
    summary["paired_checkpoint_diff"] = [run["paired_diff"] for run in run_results]
    if checkpoint_coverage is not None:
        summary["checkpoint_coverage"] = _checkpoint_coverage_report(checkpoint_coverage)
    _write_json(output_dir / "summary.json", summary)
    _write_report(output_dir / "report.md", case_name, summary)
    print(f"stress report: {output_dir / 'report.md'}")
    return _case_result(case_name, case_path, output_dir, summary)


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Run arceos_ex stress cases.")
    parser.add_argument(
        "cases",
        nargs="*",
        type=Path,
        help="stress case TOML(s); default: standard DF-0001 + DF-0002 + DF-0003 suite",
    )
    parser.add_argument("--repo-root", type=Path, help="repository root, auto-detected by default")
    parser.add_argument("--runs", type=int, help="override case default_runs")
    parser.add_argument("--timeout", type=int, help="override case timeout_seconds")
    parser.add_argument("--out-dir", type=Path, help="output root, default: tests/stress/out")
    parser.add_argument("--dry-run", action="store_true", help="validate config and write an empty report")
    parser.add_argument(
        "--fail-on-failure",
        action="store_true",
        help="compatibility no-op; failures already return non-zero",
    )
    return parser


def _selected_case_paths(cases: list[Path]) -> list[Path]:
    if cases:
        return [case.resolve() for case in cases]
    return [case.resolve() for case in DEFAULT_SUITE]


def _case_result(
    case_name: str, case_path: Path, output_dir: Path, summary: dict[str, Any]
) -> dict[str, Any]:
    return {
        "case_name": case_name,
        "case_path": str(case_path),
        "output_dir": str(output_dir),
        "report_path": str(output_dir / "report.md"),
        "summary": summary,
    }


def _case_failed(result: dict[str, Any]) -> bool:
    summary = result["summary"]
    return int(summary["totals"]["failure"]) > 0


def _print_suite_summary(results: list[dict[str, Any]]) -> None:
    print("")
    print("stress suite summary:")
    for result in results:
        totals = result["summary"]["totals"]
        print(
            "  "
            f"{result['case_name']}: "
            f"success={totals['success']} "
            f"failure={totals['failure']} "
            f"report={result['report_path']}"
        )


def _execute_one_run(
    *,
    run_id: str,
    run_dir: Path,
    command: list[str],
    repo_root: Path,
    workdir: Path,
    timeout: int,
    delayed_stdin: DelayedStdin | None,
    env_updates: dict[str, str],
    rules: list[dict[str, Any]],
) -> dict[str, Any]:
    started = datetime.now(timezone.utc)
    start_monotonic = time.monotonic()
    env = os.environ.copy()
    env.update(env_updates)
    stdout, returncode, timed_out, stdin_result = _run_command_capture(
        command, workdir, env, timeout, delayed_stdin
    )
    ended = datetime.now(timezone.utc)
    duration = time.monotonic() - start_monotonic

    observed_text, stress_mem = _observed_text(stdout)
    events = _extract_events(observed_text)
    sequence_tokens = [_event_token(event) for event in events]
    sequence_hash = _sequence_hash(sequence_tokens)
    classification = _classify(observed_text, returncode, timed_out, rules)
    result = {
        "schema_version": SCHEMA_VERSION,
        "run_id": run_id,
        "command": command,
        "working_directory": str(workdir),
        "started_at": started.isoformat(),
        "ended_at": ended.isoformat(),
        "duration_seconds": round(duration, 3),
        "returncode": returncode,
        "timed_out": timed_out,
        **stdin_result,
        "result": classification["result"],
        "class_id": classification["id"],
        "class_description": classification.get("description", ""),
        "sequence_hash": sequence_hash,
        "event_count": len(events),
    }
    if stress_mem is None:
        result["events"] = str(run_dir / "events.jsonl")
        result["events_saved"] = True
        (run_dir / "stdout.log").write_text(stdout, encoding="utf-8", errors="replace")
        (run_dir / "stderr.log").write_text(
            "stderr was merged into stdout.log to preserve event order.\n",
            encoding="utf-8",
        )
        result["stdout"] = str(run_dir / "stdout.log")
        result["stdout_saved"] = True
        _write_jsonl(run_dir / "events.jsonl", events)
    else:
        result["events_saved"] = False
        result["stdout_saved"] = False
        result["stress_mem"] = stress_mem
    run = {
        **result,
        "events_data": events,
        "sequence_tokens": sequence_tokens,
        "_run_dir": str(run_dir),
    }
    if stress_mem is not None:
        run["captured_stdout"] = stdout
        run["stress_mem_text"] = observed_text
        run["_stdout_candidate"] = str(run_dir / "stdout.first-seen.log")
        run["_events_candidate"] = str(run_dir / "events.first-seen.jsonl")
    return run


def _run_setup_command(
    command: list[str],
    repo_root: Path,
    workdir: Path,
    timeout: int,
    output_dir: Path,
    label: str = "setup",
) -> None:
    setup_dir = output_dir / _safe_path(label)
    setup_dir.mkdir(parents=True)
    stdout, returncode, timed_out, _ = _run_command_capture(
        command, workdir, os.environ.copy(), timeout, None
    )
    (setup_dir / "stdout.log").write_text(stdout, encoding="utf-8", errors="replace")
    _write_json(
        setup_dir / "result.json",
        {
            "schema_version": SCHEMA_VERSION,
            "command": command,
            "repo_root": str(repo_root),
            "working_directory": str(workdir),
            "returncode": returncode,
            "timed_out": timed_out,
        },
    )
    if timed_out:
        raise SystemExit("setup command timed out")
    if returncode != 0:
        raise SystemExit(f"setup command failed with return code {returncode}")


def _execute_paired_side(
    *,
    side_id: str,
    config: dict[str, Any],
    run_dir: Path,
    timeout: int,
) -> dict[str, Any]:
    side_dir = run_dir / side_id
    side_dir.mkdir(parents=True)
    command = _expand_command_placeholders(_as_string_list(config.get("command"), f"paired.{side_id}.command"))
    workdir = config["workdir"]
    env = os.environ.copy()
    env.update(_string_map(config.get("env", {}), f"paired.{side_id}.env"))
    delayed_stdin = _delayed_stdin(config)
    stop_after_stress_mem = bool(config.get("stop_after_stress_mem", False))

    started = datetime.now(timezone.utc)
    start_monotonic = time.monotonic()
    if stop_after_stress_mem:
        stdout, returncode, timed_out, capture_result = _run_command_capture_until_stress_mem(
            command, workdir, env, timeout, delayed_stdin
        )
    else:
        stdout, returncode, timed_out, capture_result = _run_command_capture(
            command, workdir, env, timeout, delayed_stdin
        )
    ended = datetime.now(timezone.utc)
    duration = time.monotonic() - start_monotonic

    observed_text, stress_mem = _observed_text(stdout)
    events = _extract_events(observed_text)
    sequence_tokens = [_event_token(event) for event in events]
    stdout_path = side_dir / "stdout.log"
    stdout_path.write_text(stdout, encoding="utf-8", errors="replace")
    decoded_path = side_dir / "stress-mem.txt"
    decoded_path.write_text(observed_text, encoding="utf-8", errors="replace")
    events_path = side_dir / "events.jsonl"
    _write_jsonl(events_path, events)

    result = {
        "schema_version": SCHEMA_VERSION,
        "side": side_id,
        "command": command,
        "working_directory": str(workdir),
        "started_at": started.isoformat(),
        "ended_at": ended.isoformat(),
        "duration_seconds": round(duration, 3),
        "returncode": returncode,
        "timed_out": timed_out,
        **capture_result,
        "stress_mem": stress_mem,
        "events": str(events_path),
        "stdout": str(stdout_path),
        "stress_mem_decoded": str(decoded_path),
        "event_count": len(events),
        "sequence_hash": _sequence_hash(sequence_tokens),
        "events_data": events,
        "sequence_tokens": sequence_tokens,
    }
    _write_json(side_dir / "result.json", _persisted_side_result(result))
    return result


def _persisted_side_result(side: dict[str, Any]) -> dict[str, Any]:
    return {
        key: value
        for key, value in side.items()
        if key not in {"events_data", "sequence_tokens"}
    }


def _paired_events_for_sequence(
    arceos: dict[str, Any],
    linux: dict[str, Any],
    diff: dict[str, Any],
) -> list[dict[str, Any]]:
    events: list[dict[str, Any]] = []
    for side_id, side in (("arceos_ex", arceos), ("linux", linux)):
        for event in side["events_data"]:
            events.append(
                {
                    "i": len(events),
                    "line": event.get("line", 0),
                    "kind": f"{side_id}_{event.get('kind', 'unknown')}",
                    "name": event.get("name", "unknown"),
                    "raw": event.get("raw", ""),
                }
            )
    if not diff["passed"]:
        events.append(
            {
                "i": len(events),
                "line": 0,
                "kind": "paired_diff",
                "name": "PairedCheckpointDiffMismatch",
                "raw": json.dumps(diff, ensure_ascii=True, sort_keys=True),
            }
        )
    return events


def _run_command_capture(
    command: list[str],
    workdir: Path,
    env: dict[str, str],
    timeout: int,
    delayed_stdin: DelayedStdin | None,
) -> tuple[str, int | None, bool, dict[str, Any]]:
    if delayed_stdin is not None:
        return _run_command_capture_with_delayed_stdin(
            command, workdir, env, timeout, delayed_stdin
        )

    process = subprocess.Popen(
        command,
        cwd=workdir,
        env=env,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        start_new_session=True,
    )
    try:
        stdout, _ = process.communicate(timeout=timeout)
        return stdout, process.returncode, False, {}
    except subprocess.TimeoutExpired:
        _kill_process_group(process)
        stdout, _ = process.communicate()
        return stdout, process.returncode, True, {}


def _run_command_capture_until_stress_mem(
    command: list[str],
    workdir: Path,
    env: dict[str, str],
    timeout: int,
    delayed_stdin: DelayedStdin | None = None,
) -> tuple[str, int | None, bool, dict[str, Any]]:
    marker = delayed_stdin.ready_marker.encode() if delayed_stdin is not None else None
    payload = delayed_stdin.payload.encode() if delayed_stdin is not None else None
    process = subprocess.Popen(
        command,
        cwd=workdir,
        env=env,
        stdin=subprocess.PIPE if delayed_stdin is not None else subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        start_new_session=True,
    )
    assert process.stdout is not None
    selector = selectors.DefaultSelector()
    selector.register(process.stdout, selectors.EVENT_READ)
    stdout_parts: list[bytes] = []
    tail = b""
    deadline = time.monotonic() + timeout
    timed_out = False
    terminated_after_stress_mem = False
    stdin_sent = False

    while True:
        now = time.monotonic()
        if now >= deadline:
            timed_out = True
            _kill_process_group(process)
            break
        if process.poll() is not None:
            break

        wait_time = min(0.25, max(0.0, deadline - now))
        events = selector.select(wait_time)
        if not events:
            continue
        for key, _ in events:
            chunk = os.read(key.fd, 4096)
            if not chunk:
                continue
            stdout_parts.append(chunk)
            tail = (tail + chunk)[-131072:]
            if marker is not None and payload is not None and not stdin_sent and marker in tail:
                try:
                    assert process.stdin is not None
                    process.stdin.write(payload)
                    process.stdin.flush()
                except BrokenPipeError:
                    pass
                stdin_sent = True
            if _parse_stress_mem(tail.decode("utf-8", errors="replace")) is not None:
                terminated_after_stress_mem = True
                _kill_process_group(process)
                break
        if terminated_after_stress_mem:
            break

    try:
        rest, _ = process.communicate(timeout=3)
    except subprocess.TimeoutExpired:
        _kill_process_group(process)
        rest, _ = process.communicate()
    if rest:
        stdout_parts.append(rest)
    selector.close()
    stdout = b"".join(stdout_parts).decode("utf-8", errors="replace")
    return (
        stdout,
        process.returncode,
        timed_out,
        {
            "terminated_after_stress_mem": terminated_after_stress_mem,
            **(
                {}
                if delayed_stdin is None
                else {
                    "stdin_ready_marker": delayed_stdin.ready_marker,
                    "stdin_payload_bytes": len(payload or b""),
                    "stdin_sent": stdin_sent,
                }
            ),
        },
    )


def _run_command_capture_with_delayed_stdin(
    command: list[str],
    workdir: Path,
    env: dict[str, str],
    timeout: int,
    delayed_stdin: DelayedStdin,
) -> tuple[str, int | None, bool, dict[str, Any]]:
    marker = delayed_stdin.ready_marker.encode()
    payload = delayed_stdin.payload.encode()
    process = subprocess.Popen(
        command,
        cwd=workdir,
        env=env,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        start_new_session=True,
    )
    assert process.stdout is not None
    selector = selectors.DefaultSelector()
    selector.register(process.stdout, selectors.EVENT_READ)
    stdout_parts: list[bytes] = []
    tail = b""
    stdin_sent = False
    stdout_eof = False
    deadline = time.monotonic() + timeout
    timed_out = False

    while True:
        now = time.monotonic()
        if now >= deadline:
            timed_out = True
            _kill_process_group(process)
            break
        if stdout_eof and process.poll() is not None:
            break

        wait_time = min(0.25, max(0.0, deadline - now))
        events = selector.select(wait_time)
        if not events:
            continue
        for key, _ in events:
            chunk = os.read(key.fd, 4096)
            if not chunk:
                stdout_eof = True
                continue
            stdout_parts.append(chunk)
            tail = (tail + chunk)[-4096:]
            if not stdin_sent and marker in tail:
                try:
                    assert process.stdin is not None
                    process.stdin.write(payload)
                    process.stdin.flush()
                except BrokenPipeError:
                    pass
                stdin_sent = True

    try:
        rest, _ = process.communicate(timeout=3)
    except subprocess.TimeoutExpired:
        _kill_process_group(process)
        rest, _ = process.communicate()
    if rest:
        stdout_parts.append(rest)
    selector.close()
    stdout = b"".join(stdout_parts).decode("utf-8", errors="replace")
    return (
        stdout,
        process.returncode,
        timed_out,
        {
            "stdin_ready_marker": delayed_stdin.ready_marker,
            "stdin_payload_bytes": len(payload),
            "stdin_sent": stdin_sent,
        },
    )


def _kill_process_group(process: subprocess.Popen[str]) -> None:
    if hasattr(os, "killpg"):
        try:
            os.killpg(process.pid, signal.SIGKILL)
            return
        except ProcessLookupError:
            return
    process.kill()


def _extract_events(text: str) -> list[dict[str, Any]]:
    events: list[dict[str, Any]] = []
    for line_no, raw_line in enumerate(text.splitlines(), 1):
        line = _normalize_line(raw_line)
        if not line:
            continue
        early_events = _early_byte_events_from_line(line, line_no, len(events))
        if early_events:
            events.extend(early_events)
            continue
        event = _event_from_line(line, line_no)
        if event is not None:
            event["i"] = len(events)
            events.append(event)
    if not events:
        events.append({"i": 0, "line": 0, "kind": "run", "name": "NoObservedEvents"})
    return events


def _observed_text(stdout: str) -> tuple[str, dict[str, Any] | None]:
    parsed = _parse_stress_mem(stdout)
    if parsed is None:
        return stdout, None
    text, metadata = parsed
    return text, metadata


def _parse_stress_mem(stdout: str) -> tuple[str, dict[str, Any]] | None:
    for raw_line in reversed(stdout.splitlines()):
        line = _normalize_line(raw_line)
        match = STRESS_MEM_RE.search(line)
        if not match:
            continue
        expected_bytes = int(match.group("bytes"))
        hex_data = match.group("data")
        if len(hex_data) != expected_bytes * 2:
            continue
        data = bytes.fromhex(hex_data)
        text = data.decode("utf-8", errors="replace")
        early_prefix = _stress_mem_early_prefix(line[: match.start()])
        if early_prefix:
            text = early_prefix + "\n" + text
        return text, {
            "bytes": expected_bytes,
            "total": int(match.group("total")),
            "overflow": match.group("overflow") == "1",
            "dropped": int(match.group("dropped")),
        }
    parsed = _parse_stress_mem_stream(stdout)
    if parsed is not None:
        return parsed
    return None


def _parse_stress_mem_stream(stdout: str) -> tuple[str, dict[str, Any]] | None:
    normalized = ANSI_RE.sub("", stdout)
    matches = list(STRESS_MEM_HEADER_RE.finditer(normalized))
    for match in reversed(matches):
        expected_bytes = int(match.group("bytes"))
        expected_hex_len = expected_bytes * 2
        data_segment = STRESS_MEM_PROMPT_ECHO_RE.sub("", normalized[match.end() :])
        hex_data = _take_hex_payload(data_segment, expected_hex_len)
        if hex_data is None:
            continue
        data = bytes.fromhex(hex_data)
        text = data.decode("utf-8", errors="replace")
        line_start = normalized.rfind("\n", 0, match.start()) + 1
        early_prefix = _stress_mem_early_prefix(normalized[line_start : match.start()].strip())
        if early_prefix:
            text = early_prefix + "\n" + text
        return text, {
            "bytes": expected_bytes,
            "total": int(match.group("total")),
            "overflow": match.group("overflow") == "1",
            "dropped": int(match.group("dropped")),
        }
    return None


def _take_hex_payload(data_segment: str, expected_hex_len: int) -> str | None:
    chars: list[str] = []
    for char in data_segment:
        if char in "0123456789abcdef":
            chars.append(char)
            if len(chars) == expected_hex_len:
                return "".join(chars)
            continue
        if char.isspace():
            continue
        return None
    return None


def _stress_mem_early_prefix(prefix: str) -> str:
    allowed = set(EARLY_CHECKPOINT_BYTES) | {"?"}
    index = len(prefix)
    while index > 0 and prefix[index - 1] in allowed:
        index -= 1
    return prefix[index:]


def _normalize_line(line: str) -> str:
    return ANSI_RE.sub("", line).strip()


def _normalize_text(text: str) -> str:
    return "\n".join(
        line for raw_line in text.splitlines() if (line := _normalize_line(raw_line))
    )


def _event_from_line(line: str, line_no: int) -> dict[str, Any] | None:
    if match := CHECKPOINT_RE.match(line):
        event = {
            "line": line_no,
            "kind": "checkpoint",
            "name": match.group("name"),
            "source": (
                "announce" if match.group("prefix") == "checkpoint" else "legacy-trace"
            ),
            "raw": line,
        }
        if match.group("task"):
            event["task"] = match.group("task")
        return event
    if match := READY_CHECK_FAILED_RE.match(line):
        return {
            "line": line_no,
            "kind": "ready_check_failed",
            "name": "ReadyCheckFailed",
            "phase": match.group("phase"),
            "check": match.group("check"),
            "first_failed": match.group("first_failed"),
            "raw": line,
        }
    if match := FAILURE_DIAGNOSTIC_RE.match(line):
        return {
            "line": line_no,
            "kind": "failure_diagnostic",
            "name": "FailureDiagnostic",
            "phase": match.group("phase"),
            "step": match.group("step"),
            "object": match.group("object"),
            "check": match.group("check"),
            "first_failed": match.group("first_failed"),
            "raw": line,
        }
    if match := PHASE_ERROR_RE.search(line):
        return {
            "line": line_no,
            "kind": "phase_error",
            "name": "PhaseEventError",
            "error": match.group("error"),
            "event": match.group("event"),
            "actual": match.group("actual"),
            "expected": match.group("expected"),
            "target": match.group("target"),
            "raw": line,
        }
    if line.startswith("# checkpoint fail: "):
        return {
            "line": line_no,
            "kind": "checkpoint_failure",
            "name": line.removeprefix("# checkpoint fail: "),
            "raw": line,
        }
    if line.startswith("# checkpoint stop: "):
        return {
            "line": line_no,
            "kind": "checkpoint_stop",
            "name": line.removeprefix("# checkpoint stop: "),
            "raw": line,
        }
    if "read user ELF failed" in line:
        return {"line": line_no, "kind": "symptom", "name": "ReadUserElfFailed", "raw": line}
    if "arceos_ex initcall event failed" in line:
        return {"line": line_no, "kind": "symptom", "name": "InitcallEventFailed", "raw": line}
    if "arceos_ex panic" in line:
        return {"line": line_no, "kind": "symptom", "name": "KernelPanic", "raw": line}
    if "arceos_ex allocation error" in line:
        return {"line": line_no, "kind": "symptom", "name": "AllocationError", "raw": line}
    if line.startswith("wait4 child handoff"):
        return {"line": line_no, "kind": "boundary", "name": "Wait4ChildHandoff", "raw": line}
    if "lost+found" in line:
        return {"line": line_no, "kind": "user_output", "name": "DistroLsRootListing", "raw": line}
    if "user hello" in line:
        return {"line": line_no, "kind": "user_output", "name": "UserHello", "raw": line}
    if match := USER_EXIT_RE.search(line):
        return {
            "line": line_no,
            "kind": "user_exit",
            "name": "UserExitStatus",
            "status": match.group("status"),
            "raw": line,
        }
    if match := SMOKE_RESULT_RE.search(line):
        return {
            "line": line_no,
            "kind": "smoke_result",
            "name": "SmokeResult",
            "passed": match.group("passed"),
            "failed": match.group("failed"),
            "total": match.group("total"),
            "raw": line,
        }
    return None


def _early_byte_events_from_line(
    line: str,
    line_no: int,
    start_index: int,
) -> list[dict[str, Any]]:
    if any(char not in EARLY_CHECKPOINT_BYTES and char != "?" for char in line):
        return []
    if not any(char in EARLY_CHECKPOINT_BYTES for char in line):
        return []

    events: list[dict[str, Any]] = []
    for char in line:
        name = EARLY_CHECKPOINT_BYTES.get(char)
        if name is None:
            continue
        events.append(
            {
                "i": start_index + len(events),
                "line": line_no,
                "kind": "checkpoint",
                "name": name,
                "source": "early-byte",
                "raw": char,
            }
        )
    return events


def _event_token(event: dict[str, Any]) -> str:
    kind = str(event.get("kind", "unknown"))
    name = str(event.get("name", "unknown"))
    if kind == "ready_check_failed":
        return (
            f"{kind}:{name}:phase={event.get('phase')}:check={event.get('check')}:"
            f"first_failed={event.get('first_failed')}"
        )
    if kind == "failure_diagnostic":
        return (
            f"{kind}:{name}:phase={event.get('phase')}:step={event.get('step')}:"
            f"object={event.get('object')}:check={event.get('check')}:"
            f"first_failed={event.get('first_failed')}"
        )
    if kind == "phase_error":
        return (
            f"{kind}:{name}:error={event.get('error')}:event={event.get('event')}:"
            f"actual={event.get('actual')}:expected={event.get('expected')}:target={event.get('target')}"
        )
    if kind == "user_exit":
        return f"{kind}:{name}:status={event.get('status')}"
    if kind == "smoke_result":
        return (
            f"{kind}:{name}:passed={event.get('passed')}:failed={event.get('failed')}:"
            f"total={event.get('total')}"
        )
    return f"{kind}:{name}"


def _classify(
    text: str, returncode: int | None, timed_out: bool, rules: list[dict[str, Any]]
) -> dict[str, str]:
    if timed_out:
        return {"id": "timeout", "result": "failure", "description": "command timed out"}
    normalized_text = _normalize_text(text)
    for rule in rules:
        if rule["result"] != "failure":
            continue
        if _rule_matches(rule, normalized_text):
            return {
                "id": str(rule["id"]),
                "result": str(rule["result"]),
                "description": str(rule.get("description", "")),
            }
    if returncode not in (0, None):
        return {
            "id": "nonzero-exit",
            "result": "failure",
            "description": f"command returned {returncode}",
        }
    for rule in rules:
        if rule["result"] != "success":
            continue
        if _rule_matches(rule, normalized_text):
            return {
                "id": str(rule["id"]),
                "result": str(rule["result"]),
                "description": str(rule.get("description", "")),
            }
    return {
        "id": "unknown-failure",
        "result": "failure",
        "description": "no success rule matched",
    }


def _rule_matches(rule: dict[str, Any], text: str) -> bool:
    contains = _as_string_list(rule.get("contains", []), "contains")
    regex = _as_string_list(rule.get("regex", []), "regex")
    return all(item in text for item in contains) and all(re.search(item, text) for item in regex)


def _record_sequence(sequences: dict[tuple[str, str, str], dict[str, Any]], run: dict[str, Any]) -> None:
    key = (str(run["result"]), str(run["class_id"]), str(run["sequence_hash"]))
    entry = sequences.get(key)
    if entry is None:
        _write_first_seen_artifacts(run)
        entry = {
            "schema_version": SCHEMA_VERSION,
            "result": run["result"],
            "class_id": run["class_id"],
            "sequence_hash": run["sequence_hash"],
            "first_run": run["run_id"],
            "count": 0,
            "run_ids": [],
            "tokens": run["sequence_tokens"],
            "events": run["events_data"],
            "features": _sequence_features(run["sequence_tokens"]),
        }
        sequences[key] = entry
    entry["count"] += 1
    entry["run_ids"].append(run["run_id"])


def _write_first_seen_artifacts(run: dict[str, Any]) -> None:
    events_path = run.get("_events_candidate")
    events_data = run.get("events_data")
    if isinstance(events_path, str) and isinstance(events_data, list):
        _write_jsonl(Path(events_path), events_data)
        run["events"] = events_path
        run["events_saved"] = True

    stdout_path = run.get("_stdout_candidate")
    captured_stdout = run.get("captured_stdout")
    if isinstance(stdout_path, str) and isinstance(captured_stdout, str):
        Path(stdout_path).write_text(captured_stdout, encoding="utf-8", errors="replace")
        run["stdout"] = stdout_path
        run["stdout_saved"] = True
    stress_mem_text = run.get("stress_mem_text")
    if isinstance(stdout_path, str) and isinstance(stress_mem_text, str):
        decoded_path = Path(stdout_path).with_suffix(".stress-mem.txt")
        decoded_path.write_text(
            stress_mem_text,
            encoding="utf-8",
            errors="replace",
        )
        run["stress_mem_decoded"] = str(decoded_path)


def _write_run_metadata(run_dir: Path, run: dict[str, Any], repo_root: Path) -> None:
    result = _persisted_run_result(run)
    _write_json(run_dir / "result.json", result)
    _write_json(run_dir / "meta.json", {**result, "repo_root": str(repo_root)})


def _persisted_run_result(run: dict[str, Any]) -> dict[str, Any]:
    internal_keys = {
        "events_data",
        "sequence_tokens",
        "captured_stdout",
        "stress_mem_text",
        "_run_dir",
        "_stdout_candidate",
        "_events_candidate",
    }
    return {key: value for key, value in run.items() if key not in internal_keys}


def _write_sequences(output_dir: Path, sequences: dict[tuple[str, str, str], dict[str, Any]]) -> None:
    for (result, class_id, sequence_hash), entry in sorted(sequences.items()):
        path = output_dir / "sequences" / _safe_path(result) / _safe_path(class_id)
        path.mkdir(parents=True, exist_ok=True)
        _write_json(path / f"{sequence_hash}.json", entry)


def _build_summary(
    case_name: str,
    requested_runs: int,
    run_results: list[dict[str, Any]],
    sequences: dict[tuple[str, str, str], dict[str, Any]],
    *,
    dry_run: bool,
    started: datetime,
    ended: datetime,
    duration_seconds: float,
) -> dict[str, Any]:
    totals = Counter(str(run["result"]) for run in run_results)
    class_counts = Counter((str(run["result"]), str(run["class_id"])) for run in run_results)
    sequence_counts = Counter((result, class_id) for result, class_id, _ in sequences)
    class_features = _class_features(sequences)
    comparisons = _failure_success_comparisons(sequences)
    completed_runs = len(run_results)
    average_run_seconds = (
        round(sum(float(run["duration_seconds"]) for run in run_results) / completed_runs, 3)
        if completed_runs
        else None
    )
    return {
        "schema_version": SCHEMA_VERSION,
        "case": case_name,
        "dry_run": dry_run,
        "requested_runs": requested_runs,
        "completed_runs": completed_runs,
        "started_at": started.isoformat(),
        "ended_at": ended.isoformat(),
        "total_seconds": round(duration_seconds, 3),
        "average_run_seconds": average_run_seconds,
        "totals": {
            "success": totals.get("success", 0),
            "failure": totals.get("failure", 0),
        },
        "classes": [
            {
                "result": result,
                "class_id": class_id,
                "runs": count,
                "sequences": sequence_counts.get((result, class_id), 0),
                "features": class_features.get(f"{result}/{class_id}", {}),
            }
            for (result, class_id), count in sorted(class_counts.items())
        ],
        "sequences": [
            {
                "result": result,
                "class_id": class_id,
                "sequence_hash": sequence_hash,
                "count": entry["count"],
                "first_run": entry["first_run"],
            }
            for (result, class_id, sequence_hash), entry in sorted(sequences.items())
        ],
        "failure_vs_success": comparisons,
    }


def _sequence_features(tokens: list[str]) -> dict[str, Any]:
    return {
        "event_count": len(tokens),
        "first_event": tokens[0] if tokens else None,
        "last_event": tokens[-1] if tokens else None,
        "unique_events": sorted(set(tokens)),
    }


def _class_features(sequences: dict[tuple[str, str, str], dict[str, Any]]) -> dict[str, Any]:
    by_class: dict[tuple[str, str], list[list[str]]] = {}
    for (result, class_id, _), entry in sequences.items():
        by_class.setdefault((result, class_id), []).append(list(entry["tokens"]))
    features: dict[str, Any] = {}
    for (result, class_id), token_lists in sorted(by_class.items()):
        sets = [set(tokens) for tokens in token_lists]
        union = set().union(*sets) if sets else set()
        intersection = set.intersection(*sets) if sets else set()
        features[f"{result}/{class_id}"] = {
            "common_prefix": _common_prefix(token_lists),
            "always_events": sorted(intersection),
            "event_union": sorted(union),
            "representative_sequence_count": len(token_lists),
        }
    return features


def _failure_success_comparisons(
    sequences: dict[tuple[str, str, str], dict[str, Any]]
) -> list[dict[str, Any]]:
    success_entries = [
        entry for (result, _, _), entry in sequences.items() if result == "success"
    ]
    comparisons: list[dict[str, Any]] = []
    for (result, class_id, sequence_hash), entry in sorted(sequences.items()):
        if result != "failure":
            continue
        if not success_entries:
            comparisons.append(
                {
                    "failure_class": class_id,
                    "failure_sequence": sequence_hash,
                    "status": "no_success_baseline",
                }
            )
            continue
        best = max(
            success_entries,
            key=lambda success: _common_prefix_len(entry["tokens"], success["tokens"]),
        )
        prefix_len = _common_prefix_len(entry["tokens"], best["tokens"])
        comparisons.append(
            {
                "failure_class": class_id,
                "failure_sequence": sequence_hash,
                "closest_success_sequence": best["sequence_hash"],
                "common_prefix_length": prefix_len,
                "failure_event_at_divergence": _token_at(entry["tokens"], prefix_len),
                "success_event_at_divergence": _token_at(best["tokens"], prefix_len),
            }
        )
    return comparisons


def _checkpoint_sequence(events: list[dict[str, Any]], scope: list[str]) -> list[str]:
    return _checkpoint_sequence_limited(events, scope, {})


def _checkpoint_sequence_limited(
    events: list[dict[str, Any]],
    scope: list[str],
    max_counts: dict[str, int],
) -> list[str]:
    scope_set = set(scope)
    counts: Counter[str] = Counter()
    sequence: list[str] = []
    for event in events:
        if event.get("kind") != "checkpoint":
            continue
        name = str(event.get("name"))
        if name not in scope_set:
            continue
        limit = max_counts.get(name)
        if limit is not None and counts[name] >= limit:
            continue
        counts[name] += 1
        sequence.append(name)
    return sequence


def _checkpoint_observed_but_not_compared(
    events: list[dict[str, Any]], scope: list[str]
) -> list[dict[str, Any]]:
    return _checkpoint_observed_but_not_compared_limited(events, scope, {})


def _checkpoint_observed_but_not_compared_limited(
    events: list[dict[str, Any]],
    scope: list[str],
    max_counts: dict[str, int],
) -> list[dict[str, Any]]:
    scope_set = set(scope)
    counts: Counter[str] = Counter()
    observed: dict[str, dict[str, Any]] = {}
    for event in events:
        if event.get("kind") != "checkpoint":
            continue
        name = str(event.get("name"))
        excluded_reason = "outside_checkpoint_scope"
        if name in scope_set:
            counts[name] += 1
            limit = max_counts.get(name)
            if limit is None or counts[name] <= limit:
                continue
            excluded_reason = "scope_count_limit"
        entry = observed.setdefault(
            name,
            {
                "name": name,
                "count": 0,
                "first_line": event.get("line", 0),
                "excluded_reason": excluded_reason,
            },
        )
        entry["count"] += 1
    return list(observed.values())


def _ordered_missing(expected: list[str], actual: list[str]) -> list[str]:
    remaining = Counter(actual)
    missing: list[str] = []
    for item in expected:
        if remaining[item] > 0:
            remaining[item] -= 1
        else:
            missing.append(item)
    return missing


def _first_divergence(left: list[str], right: list[str]) -> dict[str, Any] | None:
    for index, (left_item, right_item) in enumerate(zip(left, right)):
        if left_item != right_item:
            return {"index": index, "left": left_item, "right": right_item}
    if len(left) == len(right):
        return None
    index = min(len(left), len(right))
    return {
        "index": index,
        "left": _token_at(left, index),
        "right": _token_at(right, index),
    }


def _paired_checkpoint_diff(
    left_events: list[dict[str, Any]],
    right_events: list[dict[str, Any]],
    checkpoint_scope: list[str],
    *,
    checkpoint_scope_max_counts: dict[str, int] | None = None,
    left_label: str = "left",
    right_label: str = "right",
) -> dict[str, Any]:
    max_counts = checkpoint_scope_max_counts or {}
    left_sequence = _checkpoint_sequence_limited(left_events, checkpoint_scope, max_counts)
    right_sequence = _checkpoint_sequence_limited(right_events, checkpoint_scope, max_counts)
    missing_from_left = _ordered_missing(checkpoint_scope, left_sequence)
    missing_from_right = _ordered_missing(checkpoint_scope, right_sequence)
    extra_in_left = _ordered_missing(left_sequence, right_sequence)
    extra_in_right = _ordered_missing(right_sequence, left_sequence)
    first_divergence = _first_divergence(left_sequence, right_sequence)
    order_mismatch = (
        first_divergence is not None
        and not missing_from_left
        and not missing_from_right
        and not extra_in_left
        and not extra_in_right
    )
    passed = (
        not missing_from_left
        and not missing_from_right
        and not extra_in_left
        and not extra_in_right
        and first_divergence is None
    )
    return {
        "left_label": left_label,
        "right_label": right_label,
        "checkpoint_scope": checkpoint_scope,
        "checkpoint_scope_max_counts": max_counts,
        "left_sequence": left_sequence,
        "right_sequence": right_sequence,
        f"missing_from_{left_label}": missing_from_left,
        f"missing_from_{right_label}": missing_from_right,
        f"extra_in_{left_label}": extra_in_left,
        f"extra_in_{right_label}": extra_in_right,
        "observed_but_not_compared": {
            left_label: _checkpoint_observed_but_not_compared_limited(
                left_events, checkpoint_scope, max_counts
            ),
            right_label: _checkpoint_observed_but_not_compared_limited(
                right_events, checkpoint_scope, max_counts
            ),
        },
        "order_mismatch": order_mismatch,
        "first_divergence": first_divergence,
        "passed": passed,
    }


def _write_report(path: Path, case_name: str, summary: dict[str, Any]) -> None:
    lines = [
        f"# Stress Report: {case_name}",
        "",
        f"- dry_run: {summary['dry_run']}",
        f"- requested_runs: {summary['requested_runs']}",
        f"- completed_runs: {summary['completed_runs']}",
        f"- started_at: {summary['started_at']}",
        f"- ended_at: {summary['ended_at']}",
        f"- total_seconds: {summary['total_seconds']}",
        f"- average_run_seconds: {_report_scalar(summary['average_run_seconds'])}",
        f"- success: {summary['totals']['success']}",
        f"- failure: {summary['totals']['failure']}",
        "",
        "## Classes",
        "",
        "| result | class | runs | sequences |",
        "| --- | --- | ---: | ---: |",
    ]
    for item in summary["classes"]:
        lines.append(
            f"| {item['result']} | {item['class_id']} | {item['runs']} | {item['sequences']} |"
        )
    if not summary["classes"]:
        lines.append("| none | none | 0 | 0 |")
    lines.extend(["", "## Failure Vs Success", ""])
    if not summary["failure_vs_success"]:
        lines.append("No failure sequences to compare.")
    else:
        for item in summary["failure_vs_success"]:
            if item.get("status") == "no_success_baseline":
                lines.append(
                    f"- {item['failure_class']} {item['failure_sequence']}: no success baseline."
                )
                continue
            lines.append(
                "- "
                f"{item['failure_class']} {item['failure_sequence']} diverges after "
                f"{item['common_prefix_length']} events; "
                f"failure={item['failure_event_at_divergence']} "
                f"success={item['success_event_at_divergence']}."
            )
    paired_diffs = summary.get("paired_checkpoint_diff")
    if isinstance(paired_diffs, list):
        lines.extend(["", "## Paired Checkpoint Diff", ""])
        if not paired_diffs:
            lines.append("No paired runs completed.")
        for index, diff in enumerate(paired_diffs, 1):
            status = "passed" if diff.get("passed") else "failed"
            lines.append(f"- run {index}: {status}")
            lines.append(
                f"  - {diff.get('left_label')}: "
                f"{', '.join(diff.get('left_sequence', [])) or 'none'}"
            )
            lines.append(
                f"  - {diff.get('right_label')}: "
                f"{', '.join(diff.get('right_sequence', [])) or 'none'}"
            )
            if diff.get("first_divergence") is not None:
                lines.append(f"  - first_divergence: {diff['first_divergence']}")
            observed = diff.get("observed_but_not_compared")
            if isinstance(observed, dict):
                for side_label, items in observed.items():
                    if not isinstance(items, list) or not items:
                        continue
                    rendered = ", ".join(
                        f"{item.get('name')} x{item.get('count')} "
                        f"({item.get('excluded_reason')})"
                        for item in items
                        if isinstance(item, dict)
                    )
                    if rendered:
                        lines.append(
                            f"  - {side_label} observed_but_not_compared: {rendered}"
                        )
            coverage = diff.get("checkpoint_coverage")
            if isinstance(coverage, dict):
                lines.append(
                    "  - checkpoint_coverage: "
                    f"required_total={coverage.get('required_total')} "
                    f"in_scope={coverage.get('in_scope')} "
                    f"accounted_outside_scope={coverage.get('accounted_outside_scope')} "
                    f"unaccounted={coverage.get('unaccounted')}"
                )
    coverage = summary.get("checkpoint_coverage")
    if isinstance(coverage, dict):
        lines.extend(["", "## Checkpoint Coverage Audit", ""])
        lines.append(f"- mapping_path: {coverage.get('mapping_path')}")
        lines.append(
            f"- required_mapping_kinds: {', '.join(coverage.get('required_mapping_kinds', []))}"
        )
        lines.append(f"- mode: {coverage.get('mode')}")
        lines.append(f"- required_total: {coverage.get('required_total')}")
        lines.append(f"- in_scope: {coverage.get('in_scope')}")
        lines.append(
            f"- accounted_outside_scope: {coverage.get('accounted_outside_scope')}"
        )
        lines.append(f"- unaccounted: {coverage.get('unaccounted')}")
        unaccounted = coverage.get("unaccounted_checkpoints")
        if isinstance(unaccounted, list) and unaccounted:
            lines.append(f"- unaccounted_checkpoints: {', '.join(map(str, unaccounted))}")
        else:
            lines.append("- unaccounted_checkpoints: none")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def _report_scalar(value: object) -> str:
    if value is None:
        return "null"
    return str(value)


def _manifest(
    case: dict[str, Any],
    case_path: Path,
    classifier_path: Path,
    command: list[str],
    delayed_stdin: DelayedStdin | None,
    runs: int,
    timeout: int,
    repo_root: Path,
    workdir: Path,
) -> dict[str, Any]:
    return {
        "schema_version": SCHEMA_VERSION,
        "case": _string(case, "name"),
        "description": str(case.get("description", "")),
        "case_path": str(case_path),
        "classifier_path": str(classifier_path),
        "command": command,
        "delayed_stdin": (
            None
            if delayed_stdin is None
            else {
                "ready_marker": delayed_stdin.ready_marker,
                "payload": delayed_stdin.payload,
                "payload_bytes": len(delayed_stdin.payload.encode()),
            }
        ),
        "requested_runs": runs,
        "timeout_seconds": timeout,
        "repo_root": str(repo_root),
        "working_directory": str(workdir),
        "metadata": case.get("metadata", {}),
        "git": {
            "head": _git_output(repo_root, ["rev-parse", "--short", "HEAD"]),
            "status_short": _git_output(repo_root, ["status", "--short"]),
        },
    }


def _paired_manifest(
    case: dict[str, Any],
    case_path: Path,
    runs: int,
    timeout: int,
    repo_root: Path,
    workdir: Path,
    paired: dict[str, Any],
) -> dict[str, Any]:
    paired_entry = {
        "schema_version": SCHEMA_VERSION,
        "case": _string(case, "name"),
        "mode": "paired_checkpoint_diff",
        "description": str(case.get("description", "")),
        "case_path": str(case_path),
        "requested_runs": runs,
        "timeout_seconds": timeout,
        "repo_root": str(repo_root),
        "working_directory": str(workdir),
        "metadata": case.get("metadata", {}),
        "paired": {
            "checkpoint_scope": paired["checkpoint_scope"],
            "checkpoint_scope_max_counts": paired["checkpoint_scope_max_counts"],
            "arceos_ex": {
                "command": paired["arceos_ex"]["command"],
                "working_directory": str(paired["arceos_ex"]["workdir"]),
                "delayed_stdin": _delayed_stdin_summary(_delayed_stdin(paired["arceos_ex"])),
            },
            "linux": {
                "build_command": paired["linux"].get("build_command"),
                "command": paired["linux"]["command"],
                "working_directory": str(paired["linux"]["workdir"]),
                "stop_after_stress_mem": paired["linux"].get("stop_after_stress_mem", False),
                "delayed_stdin": _delayed_stdin_summary(_delayed_stdin(paired["linux"])),
            },
        },
        "git": {
            "head": _git_output(repo_root, ["rev-parse", "--short", "HEAD"]),
            "status_short": _git_output(repo_root, ["status", "--short"]),
        },
    }
    if paired.get("checkpoint_coverage") is not None:
        paired_entry["paired"]["checkpoint_coverage"] = _checkpoint_coverage_report(
            paired["checkpoint_coverage"]
        )
    return paired_entry


def _delayed_stdin_summary(delayed_stdin: DelayedStdin | None) -> dict[str, Any] | None:
    if delayed_stdin is None:
        return None
    return {
        "ready_marker": delayed_stdin.ready_marker,
        "payload": delayed_stdin.payload,
        "payload_bytes": len(delayed_stdin.payload.encode()),
    }


def _paired_config(
    case: dict[str, Any],
    case_path: Path,
    repo_root: Path,
    base_workdir: Path,
) -> dict[str, Any]:
    raw = case.get("paired")
    if not isinstance(raw, dict):
        raise SystemExit("paired_checkpoint_diff cases require a [paired] table")
    scope = _as_string_list(raw.get("checkpoint_scope"), "paired.checkpoint_scope")
    if not scope:
        raise SystemExit("paired.checkpoint_scope must not be empty")
    max_counts = _optional_integer_map(
        raw.get("checkpoint_scope_max_counts"),
        "paired.checkpoint_scope_max_counts",
    )
    checkpoint_coverage = _checkpoint_coverage_config(raw, repo_root, scope)
    arceos = _paired_side_config(raw, "arceos_ex", case_path, repo_root, base_workdir)
    linux = _paired_side_config(raw, "linux", case_path, repo_root, base_workdir)
    config = {
        "checkpoint_scope": scope,
        "checkpoint_scope_max_counts": max_counts,
        "arceos_ex": arceos,
        "linux": linux,
    }
    if checkpoint_coverage is not None:
        config["checkpoint_coverage"] = checkpoint_coverage
    return config


def _checkpoint_coverage_config(
    paired: dict[str, Any],
    repo_root: Path,
    checkpoint_scope: list[str],
) -> dict[str, Any] | None:
    raw = paired.get("checkpoint_coverage")
    if raw is None:
        return None
    if not isinstance(raw, dict):
        raise SystemExit("expected table field: paired.checkpoint_coverage")
    mapping_path = _resolve_repo_path(
        repo_root,
        _string(raw, "mapping_path"),
    )
    required_mapping_kinds = _as_string_list(
        raw.get("required_mapping_kinds"),
        "paired.checkpoint_coverage.required_mapping_kinds",
    )
    if not required_mapping_kinds:
        raise SystemExit("paired.checkpoint_coverage.required_mapping_kinds must not be empty")
    mode = _string(raw, "mode")
    if mode != "explicit-accounting":
        raise SystemExit(f"unsupported paired.checkpoint_coverage.mode: {mode}")
    accounted_outside_scope = _non_empty_string_map(
        raw.get("accounted_outside_scope", {}),
        "paired.checkpoint_coverage.accounted_outside_scope",
    )
    audit = _audit_checkpoint_coverage(
        mapping_path=mapping_path,
        required_mapping_kinds=required_mapping_kinds,
        checkpoint_scope=checkpoint_scope,
        accounted_outside_scope=accounted_outside_scope,
        mode=mode,
    )
    return {
        "mapping_path": mapping_path,
        "required_mapping_kinds": required_mapping_kinds,
        "mode": mode,
        "accounted_outside_scope": accounted_outside_scope,
        "audit": audit,
    }


def _audit_checkpoint_coverage(
    *,
    mapping_path: Path,
    required_mapping_kinds: list[str],
    checkpoint_scope: list[str],
    accounted_outside_scope: dict[str, str],
    mode: str,
) -> dict[str, Any]:
    required_kind_set = set(required_mapping_kinds)
    mapping = _load_checkpoint_mapping(mapping_path)
    required_checkpoints: list[str] = []
    seen_required: set[str] = set()
    for row_number, row in enumerate(mapping, 1):
        kind = _mapping_row_string(row, row_number, "mapping_kind")
        if kind not in required_kind_set:
            continue
        name = _mapping_row_string(row, row_number, "checkpoint_name")
        if name in seen_required:
            raise SystemExit(f"duplicate required checkpoint mapping name: {name}")
        seen_required.add(name)
        required_checkpoints.append(name)

    scope_set = set(checkpoint_scope)
    required_set = set(required_checkpoints)
    outside_scope = [name for name in required_checkpoints if name not in scope_set]
    accounted_names = [
        name for name in required_checkpoints if name in accounted_outside_scope and name not in scope_set
    ]
    unaccounted = [name for name in outside_scope if name not in accounted_outside_scope]
    accounting_for_unknown = sorted(
        name for name in accounted_outside_scope if name not in required_set
    )
    accounting_for_in_scope = sorted(
        name for name in accounted_outside_scope if name in scope_set
    )
    audit = {
        "mapping_path": str(mapping_path),
        "required_mapping_kinds": list(required_mapping_kinds),
        "mode": mode,
        "required_total": len(required_checkpoints),
        "in_scope": sum(1 for name in required_checkpoints if name in scope_set),
        "accounted_outside_scope": len(accounted_names),
        "unaccounted": len(unaccounted),
        "required_checkpoints": required_checkpoints,
        "in_scope_checkpoints": [
            name for name in required_checkpoints if name in scope_set
        ],
        "accounted_outside_scope_checkpoints": [
            {"name": name, "reason": accounted_outside_scope[name]}
            for name in accounted_names
        ],
        "unaccounted_checkpoints": unaccounted,
        "invalid_accounting": {
            "unknown_or_not_required": accounting_for_unknown,
            "already_in_scope": accounting_for_in_scope,
        },
    }
    if unaccounted or accounting_for_unknown or accounting_for_in_scope:
        problems: list[str] = []
        if unaccounted:
            problems.append(
                "unaccounted required checkpoints: " + ", ".join(unaccounted)
            )
        if accounting_for_unknown:
            problems.append(
                "accounted checkpoints are not required mappings: "
                + ", ".join(accounting_for_unknown)
            )
        if accounting_for_in_scope:
            problems.append(
                "accounted checkpoints are already in checkpoint_scope: "
                + ", ".join(accounting_for_in_scope)
            )
        raise CheckpointCoverageError(
            "checkpoint coverage audit failed; " + "; ".join(problems),
            audit,
        )
    return audit


def _load_checkpoint_mapping(path: Path) -> list[dict[str, Any]]:
    try:
        with path.open("r", encoding="utf-8") as fh:
            data = json.load(fh)
    except FileNotFoundError as error:
        raise SystemExit(f"checkpoint coverage mapping not found: {path}") from error
    except json.JSONDecodeError as error:
        raise SystemExit(f"checkpoint coverage mapping is not valid JSON: {path}") from error
    if not isinstance(data, list):
        raise SystemExit(f"checkpoint coverage mapping must be a JSON array: {path}")
    rows: list[dict[str, Any]] = []
    for row_number, row in enumerate(data, 1):
        if not isinstance(row, dict):
            raise SystemExit(f"checkpoint coverage mapping row {row_number} must be an object")
        rows.append(row)
    return rows


def _mapping_row_string(row: dict[str, Any], row_number: int, key: str) -> str:
    value = row.get(key)
    if not isinstance(value, str) or not value:
        raise SystemExit(f"checkpoint coverage mapping row {row_number} missing string {key}")
    return value


def _checkpoint_coverage_report(config: dict[str, Any]) -> dict[str, Any]:
    audit = config["audit"]
    return {
        "mapping_path": audit["mapping_path"],
        "required_mapping_kinds": list(audit["required_mapping_kinds"]),
        "mode": audit["mode"],
        "required_total": audit["required_total"],
        "in_scope": audit["in_scope"],
        "accounted_outside_scope": audit["accounted_outside_scope"],
        "unaccounted": audit["unaccounted"],
        "unaccounted_checkpoints": list(audit["unaccounted_checkpoints"]),
    }


def _paired_side_config(
    paired: dict[str, Any],
    side_id: str,
    case_path: Path,
    repo_root: Path,
    base_workdir: Path,
) -> dict[str, Any]:
    raw = paired.get(side_id)
    if not isinstance(raw, dict):
        raise SystemExit(f"paired_checkpoint_diff cases require [paired.{side_id}]")
    workdir = _resolve_workdir(repo_root, raw.get("working_directory", str(base_workdir)))
    command = _as_string_list(raw.get("command"), f"paired.{side_id}.command")
    config = {
        **raw,
        "command": command,
        "workdir": workdir,
        "case_dir": case_path.parent,
    }
    if "build_command" in raw:
        config["build_command"] = _as_string_list(
            raw.get("build_command"),
            f"paired.{side_id}.build_command",
        )
    return config


def _load_toml(path: Path) -> dict[str, Any]:
    with path.open("rb") as fh:
        data = tomllib.load(fh)
    if not isinstance(data, dict):
        raise SystemExit(f"TOML root must be a table: {path}")
    return data


def _classifier_rules(data: dict[str, Any]) -> list[dict[str, Any]]:
    raw_rules = data.get("rules", [])
    if not isinstance(raw_rules, list):
        raise SystemExit("classifier rules must be a list")
    rules = []
    for item in raw_rules:
        if not isinstance(item, dict):
            raise SystemExit("classifier rule must be a table")
        rule_id = _string(item, "id")
        result = _string(item, "result")
        if result not in ("success", "failure"):
            raise SystemExit(f"classifier rule {rule_id} has invalid result: {result}")
        rules.append(item)
    return rules


def _delayed_stdin(case: dict[str, Any]) -> DelayedStdin | None:
    raw = case.get("delayed_stdin")
    if raw is None:
        return None
    if not isinstance(raw, dict):
        raise SystemExit("expected table field: delayed_stdin")
    return DelayedStdin(
        ready_marker=_string(raw, "ready_marker"),
        payload=_string(raw, "payload"),
    )


def _resolve_repo_root(value: Path | None) -> Path:
    if value is not None:
        return value.resolve()
    current = Path(__file__).resolve()
    for parent in [current, *current.parents]:
        if (parent / ".git").exists():
            return parent
    raise SystemExit("could not auto-detect repository root")


def _resolve_case_path(case_path: Path, value: str) -> Path:
    path = Path(value)
    if not path.is_absolute():
        path = case_path.parent / path
    return path.resolve()


def _resolve_repo_path(repo_root: Path, value: str) -> Path:
    path = Path(value)
    if not path.is_absolute():
        path = repo_root / path
    return path.resolve()


def _resolve_workdir(repo_root: Path, value: object) -> Path:
    text = str(value)
    path = Path(text)
    if not path.is_absolute():
        path = repo_root / path
    return path.resolve()


def _run_dir_name(case_name: str) -> str:
    timestamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    return f"{timestamp}-{_safe_path(case_name)}"


def _unique_output_dir(base: Path) -> Path:
    if not base.exists():
        return base
    for index in range(2, 1000):
        candidate = base.with_name(f"{base.name}-{index}")
        if not candidate.exists():
            return candidate
    raise SystemExit(f"could not allocate unique output directory under {base.parent}")


def _sequence_hash(tokens: list[str]) -> str:
    payload = json.dumps(tokens, ensure_ascii=True, separators=(",", ":"))
    return hashlib.sha256(payload.encode("utf-8")).hexdigest()[:16]


def _common_prefix(items: list[list[str]]) -> list[str]:
    if not items:
        return []
    prefix = list(items[0])
    for tokens in items[1:]:
        count = _common_prefix_len(prefix, tokens)
        prefix = prefix[:count]
    return prefix


def _common_prefix_len(left: list[str], right: list[str]) -> int:
    count = 0
    for left_item, right_item in zip(left, right):
        if left_item != right_item:
            break
        count += 1
    return count


def _token_at(tokens: list[str], index: int) -> str | None:
    if index < len(tokens):
        return tokens[index]
    return None


def _safe_path(value: str) -> str:
    return re.sub(r"[^A-Za-z0-9_.-]+", "-", value).strip("-") or "unnamed"


def _git_output(repo_root: Path, args: list[str]) -> str:
    completed = subprocess.run(
        ["git", *args],
        cwd=repo_root,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
        check=False,
    )
    if completed.returncode != 0:
        return ""
    return completed.stdout.strip()


def _write_json(path: Path, data: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def _write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as fh:
        for row in rows:
            fh.write(json.dumps(row, ensure_ascii=False, separators=(",", ":")) + "\n")


def _string(data: dict[str, Any], key: str) -> str:
    value = data.get(key)
    if not isinstance(value, str) or not value:
        raise SystemExit(f"expected non-empty string field: {key}")
    return value


def _optional_string(data: dict[str, Any], key: str) -> str | None:
    if key not in data:
        return None
    value = data.get(key)
    if not isinstance(value, str) or not value:
        raise SystemExit(f"expected non-empty string field: {key}")
    return value


def _integer(data: dict[str, Any], key: str) -> int:
    value = data.get(key)
    if not isinstance(value, int):
        raise SystemExit(f"expected integer field: {key}")
    return value


def _string_list(data: dict[str, Any], key: str) -> list[str]:
    return _as_string_list(data.get(key), key)


def _optional_string_list(data: dict[str, Any], key: str) -> list[str] | None:
    if key not in data:
        return None
    return _as_string_list(data.get(key), key)


def _as_string_list(value: object, name: str) -> list[str]:
    if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
        raise SystemExit(f"expected string list field: {name}")
    return list(value)


def _string_map(value: object, name: str) -> dict[str, str]:
    if not isinstance(value, dict):
        raise SystemExit(f"expected table field: {name}")
    result: dict[str, str] = {}
    for key, item in value.items():
        if not isinstance(key, str) or not isinstance(item, str):
            raise SystemExit(f"expected string map entries in field: {name}")
        result[key] = item
    return result


def _non_empty_string_map(value: object, name: str) -> dict[str, str]:
    result = _string_map(value, name)
    for key, item in result.items():
        if not key or not item:
            raise SystemExit(f"expected non-empty string map entries in field: {name}")
    return result


def _optional_integer_map(value: object, name: str) -> dict[str, int]:
    if value is None:
        return {}
    if not isinstance(value, dict):
        raise SystemExit(f"expected table field: {name}")
    result: dict[str, int] = {}
    for key, item in value.items():
        if not isinstance(key, str) or not isinstance(item, int) or item <= 0:
            raise SystemExit(f"expected positive integer map entries in field: {name}")
        result[key] = item
    return result


def _expand_command_placeholders(command: list[str]) -> list[str]:
    nproc = str(os.cpu_count() or 1)
    return [item.replace("$(nproc)", nproc) for item in command]


if __name__ == "__main__":
    raise SystemExit(main())
