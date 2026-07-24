#!/usr/bin/env python3
"""Orchestrate basic tests for stress classification and checkpoint difftest."""

from __future__ import annotations

import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time
import tomllib
from typing import Any


SCHEMA_VERSION = 2
STRESS_DIR = Path(__file__).resolve().parent
DEFAULT_SUITE = (
    STRESS_DIR / "cases" / "df-0001-user-boot.toml",
    STRESS_DIR / "cases" / "df-0002-smoke-initcall.toml",
    STRESS_DIR / "cases" / "df-0003-distro-sh-ls.toml",
)
DEFAULT_OUT_ROOT = STRESS_DIR / "out"
BASIC_DIR = STRESS_DIR.parent / "basic"
BASIC_RUNNER = BASIC_DIR / "runner.py"
BASIC_CASES = BASIC_DIR / "cases"
TEST_NAME_RE = re.compile(r"^[a-z0-9][a-z0-9-]*$")
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
STRESS_MEM_HEADER_RE = re.compile(
    r"stress_mem: v=1 encoding=hex bytes=(?P<bytes>\d+) total=(?P<total>\d+) "
    r"overflow=(?P<overflow>[01]) dropped=(?P<dropped>\d+) data="
)
STRESS_MEM_PROMPT_ECHO_RE = re.compile(r"(?:~|/) # [^\r\n]*(?:\r?\n)?")
EARLY_CHECKPOINT_BYTES = {
    "R": "Kernel.Started",
    "T": "BootTask.OnCpu",
    "O": "BootInitFlow.Started",
    "I": "InterruptStream.Prepared",
    "K": "KernelImage.Prepared",
    "Z": "KernelImage.Ready",
    "H": "BootCPU.Prepared",
    "G": "CpuGroup.Prepared",
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


class CompositeConfigError(Exception):
    pass


class CheckpointCoverageError(CompositeConfigError):
    def __init__(self, message: str, audit: dict[str, Any]) -> None:
        super().__init__(message)
        self.audit = audit


def main(argv: list[str] | None = None) -> int:
    args = _build_parser().parse_args(argv)
    try:
        if args.runs is not None and args.runs < 0:
            raise CompositeConfigError("--runs must be non-negative")
        repo_root = _resolve_repo_root(args.repo_root)
        paths = _selected_case_paths(args.cases)
        cases = [_load_case(path, repo_root) for path in paths]
        if args.baseline is not None:
            if len(cases) != 1 or cases[0]["mode"] != "stress":
                raise CompositeConfigError("--baseline requires exactly one stress case")
            _validate_baseline(args.baseline, cases[0])
        effective_runs = [0 if args.dry_run else _case_runs(case, args.runs) for case in cases]
        if any(runs > 0 for runs in effective_runs):
            _prepare_canonical_disk(repo_root)
        out_root = args.out_dir.resolve() if args.out_dir else DEFAULT_OUT_ROOT
        results = [
            _run_case(
                case=case,
                repo_root=repo_root,
                out_root=out_root,
                runs=runs,
                baseline=args.baseline if case["mode"] == "stress" else None,
            )
            for case, runs in zip(cases, effective_runs)
        ]
    except CompositeConfigError as error:
        print(f"composite test configuration error: {error}", file=sys.stderr)
        return 2
    except RuntimeError as error:
        print(f"composite test failed: {error}", file=sys.stderr)
        return 1
    _print_suite_summary(results)
    return 1 if any(_case_failed(result) for result in results) else 0


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("cases", nargs="*", type=Path)
    parser.add_argument("--repo-root", type=Path)
    parser.add_argument("--runs", type=int)
    parser.add_argument("--out-dir", type=Path)
    parser.add_argument("--baseline", type=Path)
    parser.add_argument("--dry-run", action="store_true", help="compatibility alias for --runs 0")
    return parser


def _selected_case_paths(cases: list[Path]) -> list[Path]:
    return [path.resolve() for path in cases] if cases else [path.resolve() for path in DEFAULT_SUITE]


def _load_case(path: Path, repo_root: Path) -> dict[str, Any]:
    raw = _load_toml(path)
    version = raw.get("schema_version")
    if version != SCHEMA_VERSION:
        raise CompositeConfigError(
            f"{path}: unsupported schema_version {version!r}; composite cases require schema v2"
        )
    mode = raw.get("mode")
    if mode == "stress":
        allowed = {
            "schema_version", "name", "description", "mode", "test", "runs",
            "classifier", "metadata",
        }
    elif mode == "difftest":
        allowed = {
            "schema_version", "name", "description", "mode", "runs",
            "left_test", "left_label", "right_test", "right_label",
            "checkpoint_scope", "checkpoint_scope_max_counts", "checkpoint_coverage",
            "metadata",
        }
    else:
        raise CompositeConfigError(f"{path}: mode must be 'stress' or 'difftest'")
    unknown = sorted(set(raw) - allowed)
    if unknown:
        raise CompositeConfigError(f"{path}: forbidden or unknown field(s): {', '.join(unknown)}")
    name = _required_string(raw, "name", str(path))
    if path.stem != name or not TEST_NAME_RE.fullmatch(name):
        raise CompositeConfigError(f"{path}: name must be a valid test name matching the filename")
    description = raw.get("description", "")
    if not isinstance(description, str):
        raise CompositeConfigError(f"{path}: description must be a string")
    runs = raw.get("runs")
    if not isinstance(runs, int) or isinstance(runs, bool) or runs < 0:
        raise CompositeConfigError(f"{path}: runs must be a non-negative integer")
    metadata = raw.get("metadata", {})
    if not isinstance(metadata, dict):
        raise CompositeConfigError(f"{path}: metadata must be a table")
    config: dict[str, Any] = {
        "schema_version": SCHEMA_VERSION,
        "name": name,
        "description": description,
        "mode": mode,
        "runs": runs,
        "metadata": metadata,
        "case_path": path.resolve(),
        "case_sha256": _sha256_file(path),
    }
    if mode == "stress":
        test = _required_string(raw, "test", str(path))
        classifier_path = _resolve_case_path(path, _required_string(raw, "classifier", str(path)))
        classifier = _load_toml(classifier_path)
        config.update(
            test=test,
            basic=_validate_basic(test, repo_root),
            classifier_path=classifier_path,
            classifier_sha256=_sha256_file(classifier_path),
            rules=_classifier_rules(classifier),
        )
    else:
        scope = _string_list(raw.get("checkpoint_scope"), "checkpoint_scope")
        if not scope:
            raise CompositeConfigError(f"{path}: checkpoint_scope must not be empty")
        left_test = _required_string(raw, "left_test", str(path))
        right_test = _required_string(raw, "right_test", str(path))
        left_label = _required_string(raw, "left_label", str(path))
        right_label = _required_string(raw, "right_label", str(path))
        if left_label == right_label:
            raise CompositeConfigError(f"{path}: left_label and right_label must differ")
        max_counts = _positive_integer_map(
            raw.get("checkpoint_scope_max_counts"), "checkpoint_scope_max_counts"
        )
        if any(name not in set(scope) for name in max_counts):
            raise CompositeConfigError(f"{path}: checkpoint count limit names must be in scope")
        coverage = _checkpoint_coverage_config(raw.get("checkpoint_coverage"), repo_root, scope)
        config.update(
            left_test=left_test,
            left_label=left_label,
            right_test=right_test,
            right_label=right_label,
            left_basic=_validate_basic(left_test, repo_root),
            right_basic=_validate_basic(right_test, repo_root),
            checkpoint_scope=scope,
            checkpoint_scope_max_counts=max_counts,
            checkpoint_coverage=coverage,
        )
    config["config_fingerprint"] = _config_fingerprint(config)
    return config


def _validate_basic(test: str, repo_root: Path) -> dict[str, Any]:
    if not TEST_NAME_RE.fullmatch(test):
        raise CompositeConfigError(f"invalid basic test name: {test!r}")
    path = (repo_root / "impl" / "arceos_ex" / "tests" / "basic" / "cases" / f"{test}.toml").resolve()
    if not path.is_file():
        raise CompositeConfigError(f"referenced basic test does not exist: {test}")
    try:
        module = _basic_runner_module()
        loaded = module.load_config(path, repo_root)
    except Exception as error:
        raise CompositeConfigError(f"invalid referenced basic test {test}: {error}") from error
    if loaded.get("source_schema_version") != 2 or loaded.get("name") != test:
        raise CompositeConfigError(f"referenced basic test must be canonical schema v2: {test}")
    return {
        "test": test,
        "config_path": str(path),
        "config_sha256": _sha256_file(path),
        "purpose": loaded["purpose"],
        "kernel_target": loaded["kernel"]["target"],
    }


_BASIC_RUNNER_MODULE: Any | None = None


def _basic_runner_module() -> Any:
    global _BASIC_RUNNER_MODULE
    if _BASIC_RUNNER_MODULE is not None:
        return _BASIC_RUNNER_MODULE
    sys.path.insert(0, str(BASIC_DIR))
    spec = importlib.util.spec_from_file_location("lkm_basic_runner", BASIC_RUNNER)
    if spec is None or spec.loader is None:
        raise CompositeConfigError(f"cannot load basic runner: {BASIC_RUNNER}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    _BASIC_RUNNER_MODULE = module
    return module


def _case_runs(case: dict[str, Any], override: int | None) -> int:
    return int(case["runs"] if override is None else override)


def _prepare_canonical_disk(repo_root: Path) -> None:
    print("[composite] preparing canonical disk", flush=True)
    completed = subprocess.run(["make", "disk"], cwd=repo_root, check=False)
    if completed.returncode != 0:
        raise RuntimeError(f"canonical make disk exited with status {completed.returncode}")


def _run_case(
    *,
    case: dict[str, Any],
    repo_root: Path,
    out_root: Path,
    runs: int,
    baseline: Path | None,
) -> dict[str, Any]:
    output_dir = _unique_output_dir(out_root / _run_dir_name(case["name"]))
    output_dir.mkdir(parents=True, exist_ok=False)
    baseline_manifest = _baseline_manifest(baseline) if baseline is not None else None
    manifest = _manifest(case, runs, repo_root, baseline_manifest)
    _write_json(output_dir / "manifest.json", manifest)
    started = datetime.now(timezone.utc)
    start_monotonic = time.monotonic()
    sequences: dict[tuple[str, str, str], dict[str, Any]] = {}
    run_results: list[dict[str, Any]] = []
    for index in range(1, runs + 1):
        run_id = f"run-{index:04d}"
        run_dir = output_dir / "runs" / run_id
        run_dir.mkdir(parents=True)
        print(f"[composite] {case['name']} {run_id}/{runs}", flush=True)
        if case["mode"] == "stress":
            run = _execute_stress_run(case, run_id, run_dir, repo_root)
        else:
            run = _execute_difftest_run(case, run_id, run_dir, repo_root)
        _record_sequence(sequences, run)
        _write_json(run_dir / "result.json", _persisted_run_result(run))
        _write_json(run_dir / "events.json", run["events_data"])
        run_results.append(_persisted_run_result(run))
    _write_sequences(output_dir, sequences)
    ended = datetime.now(timezone.utc)
    summary = _build_summary(
        case["name"], runs, run_results, sequences,
        dry_run=runs == 0, started=started, ended=ended,
        duration_seconds=time.monotonic() - start_monotonic,
    )
    summary["mode"] = case["mode"]
    if case["mode"] == "difftest":
        summary["paired_checkpoint_diff"] = [run["paired_diff"] for run in run_results]
        if case["checkpoint_coverage"] is not None:
            summary["checkpoint_coverage"] = _checkpoint_coverage_report(case["checkpoint_coverage"])
    if baseline is not None:
        summary["historical_baseline"] = _compare_baseline(baseline, case, summary, run_results)
    _write_json(output_dir / "summary.json", summary)
    _write_report(output_dir / "report.md", case["name"], summary)
    print(f"composite report: {output_dir / 'report.md'}", flush=True)
    return _case_result(case["name"], case["case_path"], output_dir, summary)


def _execute_basic(test: str, artifact_dir: Path, repo_root: Path) -> dict[str, Any]:
    command = [
        sys.executable, str(repo_root / "impl" / "arceos_ex" / "tests" / "basic" / "runner.py"),
        "run", test, "--repo-root", str(repo_root), "--output-dir", str(artifact_dir),
    ]
    started = time.monotonic()
    completed = subprocess.run(command, cwd=repo_root, check=False)
    duration = time.monotonic() - started
    result_path = artifact_dir / "result.json"
    log_path = artifact_dir / "qemu.log"
    if not result_path.is_file() or not log_path.is_file():
        raise RuntimeError(f"basic test {test} did not produce result.json and qemu.log in {artifact_dir}")
    try:
        result = json.loads(result_path.read_text())
    except json.JSONDecodeError as error:
        raise RuntimeError(f"basic test {test} produced invalid result.json: {error}") from error
    if result.get("schema_version") != 2 or result.get("test") != test:
        raise RuntimeError(f"basic test {test} produced an incompatible result")
    return {
        "command_exit_code": completed.returncode,
        "duration_seconds": duration,
        "artifact_dir": str(artifact_dir),
        "result_path": str(result_path),
        "qemu_log_path": str(log_path),
        "result": result,
        "qemu_log": log_path.read_text(errors="replace"),
    }


def _basic_gate(execution: dict[str, Any]) -> tuple[bool, str | None]:
    result = execution["result"]
    if execution["command_exit_code"] != 0:
        return False, "basic-command-failed"
    if result.get("execution_status") != "completed":
        return False, "basic-execution-failed"
    expectations = result.get("expectations")
    if not isinstance(expectations, dict) or expectations.get("passed") is not True:
        return False, "basic-expectations-failed"
    return True, None


def _execute_basic_side(test: str, artifact_dir: Path, repo_root: Path) -> dict[str, Any]:
    try:
        return _execute_basic(test, artifact_dir, repo_root)
    except Exception as error:
        log_path = artifact_dir / "qemu.log"
        return {
            "command_exit_code": 1,
            "duration_seconds": 0.0,
            "artifact_dir": str(artifact_dir),
            "result_path": str(artifact_dir / "result.json"),
            "qemu_log_path": str(log_path),
            "qemu_log": log_path.read_text(errors="replace") if log_path.is_file() else "",
            "result": {
                "schema_version": 2,
                "test": test,
                "execution_status": "failed",
                "verdict": "inconclusive",
                "expectations": None,
                "errors": [str(error)],
                "qemu": None,
                "cleanup": None,
            },
        }


def _execute_stress_run(
    case: dict[str, Any], run_id: str, run_dir: Path, repo_root: Path
) -> dict[str, Any]:
    started = datetime.now(timezone.utc)
    start_monotonic = time.monotonic()
    execution = _execute_basic(case["test"], run_dir / "basic", repo_root)
    observed, stress_mem = _observed_text(execution["qemu_log"])
    events = _extract_events(observed)
    tokens = [_event_token(event) for event in events]
    gate_passed, gate_failure = _basic_gate(execution)
    classification = _classify(
        observed,
        execution["command_exit_code"] if gate_passed else 1,
        bool(execution["result"].get("qemu", {}).get("timed_out")),
        case["rules"],
    )
    if not gate_passed and classification["result"] == "success":
        classification = {
            "id": gate_failure or "basic-failed",
            "result": "failure",
            "description": "referenced basic test did not satisfy its standalone gate",
        }
    ended = datetime.now(timezone.utc)
    return {
        "schema_version": SCHEMA_VERSION,
        "run_id": run_id,
        "started_at": started.isoformat(),
        "ended_at": ended.isoformat(),
        "duration_seconds": round(time.monotonic() - start_monotonic, 3),
        "result": classification["result"],
        "class_id": classification["id"],
        "class_description": classification["description"],
        "sequence_hash": _sequence_hash(tokens),
        "sequence_tokens": tokens,
        "event_count": len(events),
        "stress_mem": stress_mem,
        "basic": _persisted_basic(execution),
        "events_data": events,
    }


def _execute_difftest_run(
    case: dict[str, Any], run_id: str, run_dir: Path, repo_root: Path
) -> dict[str, Any]:
    started = datetime.now(timezone.utc)
    start_monotonic = time.monotonic()
    left = _execute_basic_side(case["left_test"], run_dir / "left", repo_root)
    right = _execute_basic_side(case["right_test"], run_dir / "right", repo_root)
    left_text, left_stress = _observed_text(left["qemu_log"])
    right_text, right_stress = _observed_text(right["qemu_log"])
    left_events = _extract_events(left_text)
    right_events = _extract_events(right_text)
    diff = _paired_checkpoint_diff(
        left_events,
        right_events,
        case["checkpoint_scope"],
        checkpoint_scope_max_counts=case["checkpoint_scope_max_counts"],
        left_label=case["left_label"],
        right_label=case["right_label"],
    )
    if case["checkpoint_coverage"] is not None:
        diff["checkpoint_coverage"] = _checkpoint_coverage_report(case["checkpoint_coverage"])
    left_ok, left_failure = _basic_gate(left)
    right_ok, right_failure = _basic_gate(right)
    passed = left_ok and right_ok and diff["passed"]
    tokens = [
        *(f"{case['left_label']}:{_event_token(event)}" for event in left_events),
        *(f"{case['right_label']}:{_event_token(event)}" for event in right_events),
    ]
    ended = datetime.now(timezone.utc)
    return {
        "schema_version": SCHEMA_VERSION,
        "run_id": run_id,
        "started_at": started.isoformat(),
        "ended_at": ended.isoformat(),
        "duration_seconds": round(time.monotonic() - start_monotonic, 3),
        "result": "success" if passed else "failure",
        "class_id": "paired-checkpoint-diff-ok" if passed else "paired-checkpoint-diff",
        "class_description": (
            "both basic tests completed and the checkpoint scope matched"
            if passed else "a basic side failed its standalone gate or checkpoint scope differed"
        ),
        "sequence_hash": _sequence_hash(tokens),
        "sequence_tokens": tokens,
        "event_count": len(tokens),
        "left": {**_persisted_basic(left), "label": case["left_label"], "gate_passed": left_ok, "gate_failure": left_failure, "stress_mem": left_stress},
        "right": {**_persisted_basic(right), "label": case["right_label"], "gate_passed": right_ok, "gate_failure": right_failure, "stress_mem": right_stress},
        "paired_diff": diff,
        "events_data": {
            case["left_label"]: left_events,
            case["right_label"]: right_events,
        },
    }


def _persisted_basic(execution: dict[str, Any]) -> dict[str, Any]:
    result = execution["result"]
    expectations = result.get("expectations")
    return {
        "artifact_dir": execution["artifact_dir"],
        "result_path": execution["result_path"],
        "qemu_log_path": execution["qemu_log_path"],
        "command_exit_code": execution["command_exit_code"],
        "duration_seconds": round(execution["duration_seconds"], 3),
        "test": result.get("test"),
        "execution_status": result.get("execution_status"),
        "verdict": result.get("verdict"),
        "expectations_passed": expectations.get("passed") if isinstance(expectations, dict) else None,
        "errors": result.get("errors", []),
        "qemu": result.get("qemu"),
        "cleanup": result.get("cleanup"),
    }


def _manifest(
    case: dict[str, Any], runs: int, repo_root: Path, baseline: dict[str, Any] | None
) -> dict[str, Any]:
    manifest: dict[str, Any] = {
        "schema_version": SCHEMA_VERSION,
        "case": case["name"],
        "mode": case["mode"],
        "description": case["description"],
        "case_path": str(case["case_path"]),
        "case_sha256": case["case_sha256"],
        "config_fingerprint": case["config_fingerprint"],
        "requested_runs": runs,
        "repo_root": str(repo_root),
        "metadata": case["metadata"],
        "git": {
            "head": _git_output(repo_root, ["rev-parse", "--short", "HEAD"]),
            "status_short": _git_output(repo_root, ["status", "--short"]),
        },
    }
    if case["mode"] == "stress":
        manifest.update(
            test=case["test"],
            basic=case["basic"],
            classifier_path=str(case["classifier_path"]),
            classifier_sha256=case["classifier_sha256"],
        )
        if baseline is not None:
            manifest["historical_baseline"] = baseline
    else:
        manifest["left"] = {"label": case["left_label"], **case["left_basic"]}
        manifest["right"] = {"label": case["right_label"], **case["right_basic"]}
        manifest["checkpoint_scope"] = case["checkpoint_scope"]
        manifest["checkpoint_scope_max_counts"] = case["checkpoint_scope_max_counts"]
        if case["checkpoint_coverage"] is not None:
            manifest["checkpoint_coverage"] = _checkpoint_coverage_report(case["checkpoint_coverage"])
    return manifest


def _config_fingerprint(case: dict[str, Any]) -> str:
    if case["mode"] == "stress":
        value = {
            "mode": "stress",
            "case": case["name"],
            "test": case["basic"]["config_sha256"],
            "classifier": case["classifier_sha256"],
            "metadata": case["metadata"],
        }
    else:
        value = {
            "mode": "difftest",
            "case": case["name"],
            "left": case["left_basic"]["config_sha256"],
            "right": case["right_basic"]["config_sha256"],
            "scope": case["checkpoint_scope"],
            "max_counts": case["checkpoint_scope_max_counts"],
            "metadata": case["metadata"],
        }
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def _validate_baseline(path: Path, case: dict[str, Any]) -> None:
    directory = path.resolve()
    try:
        manifest = json.loads((directory / "manifest.json").read_text())
        summary = json.loads((directory / "summary.json").read_text())
    except (OSError, json.JSONDecodeError) as error:
        raise CompositeConfigError(f"invalid stress baseline {directory}: {error}") from error
    if manifest.get("schema_version") != 2 or summary.get("schema_version") != 2:
        raise CompositeConfigError("stress baseline must use report schema v2")
    checks = {
        "case": case["name"],
        "mode": "stress",
        "test": case["test"],
        "config_fingerprint": case["config_fingerprint"],
        "classifier_sha256": case["classifier_sha256"],
    }
    for key, expected in checks.items():
        if manifest.get(key) != expected:
            raise CompositeConfigError(f"stress baseline {key} mismatch")


def _baseline_manifest(path: Path) -> dict[str, Any]:
    directory = path.resolve()
    files = [directory / "manifest.json", directory / "summary.json"]
    files.extend(sorted((directory / "runs").glob("run-*/result.json")))
    files.extend(sorted((directory / "sequences").glob("**/*.json")))
    hashes = [
        {"path": str(file.relative_to(directory)), "sha256": _sha256_file(file)}
        for file in files if file.is_file()
    ]
    aggregate = hashlib.sha256(
        "".join(f"{item['path']}\0{item['sha256']}\n" for item in hashes).encode()
    ).hexdigest()
    return {"path": str(directory), "content_sha256": aggregate, "files": hashes}


def _compare_baseline(
    path: Path,
    case: dict[str, Any],
    current: dict[str, Any],
    current_runs: list[dict[str, Any]],
) -> dict[str, Any]:
    _validate_baseline(path, case)
    directory = path.resolve()
    baseline = json.loads((directory / "summary.json").read_text())
    baseline_runs = _load_run_results(directory)
    baseline_classes = {(item["result"], item["class_id"]) for item in baseline.get("classes", [])}
    current_classes = {(item["result"], item["class_id"]) for item in current.get("classes", [])}
    baseline_sequences = {
        (item["result"], item["class_id"], item["sequence_hash"])
        for item in baseline.get("sequences", [])
    }
    current_sequences = {
        (item["result"], item["class_id"], item["sequence_hash"])
        for item in current.get("sequences", [])
    }
    baseline_rate = _failure_rate(baseline)
    current_rate = _failure_rate(current)
    recent_divergence = None
    if baseline_runs and current_runs:
        recent_divergence = _first_divergence(
            list(baseline_runs[-1].get("sequence_tokens", [])),
            list(current_runs[-1].get("sequence_tokens", [])),
        )
    return {
        "path": str(directory),
        "baseline_failure_rate": baseline_rate,
        "current_failure_rate": current_rate,
        "failure_rate_delta": round(current_rate - baseline_rate, 6),
        "classes_added": _tuple_rows(current_classes - baseline_classes, ("result", "class_id")),
        "classes_removed": _tuple_rows(baseline_classes - current_classes, ("result", "class_id")),
        "sequences_added": _tuple_rows(current_sequences - baseline_sequences, ("result", "class_id", "sequence_hash")),
        "sequences_removed": _tuple_rows(baseline_sequences - current_sequences, ("result", "class_id", "sequence_hash")),
        "recent_sequence_first_divergence": recent_divergence,
        "affects_exit_status": False,
    }


def _failure_rate(summary: dict[str, Any]) -> float:
    total = int(summary.get("completed_runs", 0))
    failures = int(summary.get("totals", {}).get("failure", 0))
    return round(failures / total, 6) if total else 0.0


def _load_run_results(directory: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for path in sorted((directory / "runs").glob("run-*/result.json")):
        try:
            value = json.loads(path.read_text())
        except (OSError, json.JSONDecodeError):
            continue
        if isinstance(value, dict):
            rows.append(value)
    return rows


def _tuple_rows(values: set[tuple[Any, ...]], names: tuple[str, ...]) -> list[dict[str, Any]]:
    return [dict(zip(names, value)) for value in sorted(values)]


def _observed_text(stdout: str) -> tuple[str, dict[str, Any] | None]:
    parsed = _parse_stress_mem(stdout)
    return (stdout, None) if parsed is None else parsed


def _parse_stress_mem(stdout: str) -> tuple[str, dict[str, Any]] | None:
    normalized = ANSI_RE.sub("", stdout)
    for match in reversed(list(STRESS_MEM_HEADER_RE.finditer(normalized))):
        expected_bytes = int(match.group("bytes"))
        data_segment = STRESS_MEM_PROMPT_ECHO_RE.sub("", normalized[match.end():])
        hex_data = _take_hex_payload(data_segment, expected_bytes * 2)
        if hex_data is None:
            continue
        try:
            decoded = bytes.fromhex(hex_data).decode("utf-8", errors="replace")
        except ValueError:
            continue
        return decoded, {
            "bytes": expected_bytes,
            "total": int(match.group("total")),
            "overflow": match.group("overflow") == "1",
            "dropped": int(match.group("dropped")),
        }
    return None


def _take_hex_payload(data_segment: str, expected_hex_len: int) -> str | None:
    if expected_hex_len == 0:
        return ""
    chars: list[str] = []
    for char in data_segment:
        if char in "0123456789abcdef":
            chars.append(char)
            if len(chars) == expected_hex_len:
                return "".join(chars)
        elif not char.isspace():
            return None
    return None


def _extract_events(text: str) -> list[dict[str, Any]]:
    events: list[dict[str, Any]] = []
    for line_no, raw_line in enumerate(text.splitlines(), 1):
        line = _normalize_line(raw_line)
        if not line:
            continue
        early = _early_byte_events_from_line(line, line_no, len(events))
        if early:
            events.extend(early)
            continue
        event = _event_from_line(line, line_no)
        if event is not None:
            event["i"] = len(events)
            events.append(event)
    if not events:
        events.append({"i": 0, "line": 0, "kind": "run", "name": "NoObservedEvents"})
    return events


def _normalize_line(line: str) -> str:
    return ANSI_RE.sub("", line).strip()


def _normalize_text(text: str) -> str:
    return "\n".join(line for raw in text.splitlines() if (line := _normalize_line(raw)))


def _event_from_line(line: str, line_no: int) -> dict[str, Any] | None:
    if match := CHECKPOINT_RE.match(line):
        event: dict[str, Any] = {
            "line": line_no,
            "kind": "checkpoint",
            "name": match.group("name"),
            "source": "announce" if match.group("prefix") == "checkpoint" else "legacy-trace",
            "raw": line,
        }
        if match.group("task"):
            event["task"] = match.group("task")
        return event
    if match := READY_CHECK_FAILED_RE.match(line):
        return {"line": line_no, "kind": "ready_check_failed", "name": "ReadyCheckFailed", **match.groupdict(), "raw": line}
    if match := FAILURE_DIAGNOSTIC_RE.match(line):
        return {"line": line_no, "kind": "failure_diagnostic", "name": "FailureDiagnostic", **match.groupdict(), "raw": line}
    if match := PHASE_ERROR_RE.search(line):
        return {"line": line_no, "kind": "phase_error", "name": "PhaseEventError", **match.groupdict(), "raw": line}
    symptoms = (
        ("read user ELF failed", "ReadUserElfFailed"),
        ("arceos_ex initcall event failed", "InitcallEventFailed"),
        ("arceos_ex panic", "KernelPanic"),
        ("memory allocation of", "AllocationError"),
    )
    for needle, name in symptoms:
        if needle in line:
            return {"line": line_no, "kind": "symptom", "name": name, "raw": line}
    if line.startswith("wait4 child handoff"):
        return {"line": line_no, "kind": "boundary", "name": "Wait4ChildHandoff", "raw": line}
    if "lost+found" in line:
        return {"line": line_no, "kind": "user_output", "name": "DistroLsRootListing", "raw": line}
    if "user hello" in line:
        return {"line": line_no, "kind": "user_output", "name": "UserHello", "raw": line}
    if match := USER_EXIT_RE.search(line):
        return {"line": line_no, "kind": "user_exit", "name": "UserExitStatus", "status": match.group("status"), "raw": line}
    if match := SMOKE_RESULT_RE.search(line):
        return {"line": line_no, "kind": "smoke_result", "name": "SmokeResult", **match.groupdict(), "raw": line}
    return None


def _early_byte_events_from_line(line: str, line_no: int, start: int) -> list[dict[str, Any]]:
    if any(char not in EARLY_CHECKPOINT_BYTES and char != "?" for char in line):
        return []
    if not any(char in EARLY_CHECKPOINT_BYTES for char in line):
        return []
    return [
        {"i": start + index, "line": line_no, "kind": "checkpoint", "name": EARLY_CHECKPOINT_BYTES[char], "source": "early-byte", "raw": char}
        for index, char in enumerate(char for char in line if char in EARLY_CHECKPOINT_BYTES)
    ]


def _event_token(event: dict[str, Any]) -> str:
    kind = str(event.get("kind", "unknown"))
    name = str(event.get("name", "unknown"))
    if kind in {"ready_check_failed", "failure_diagnostic", "phase_error"}:
        details = ":".join(f"{key}={event[key]}" for key in sorted(event) if key not in {"i", "line", "kind", "name", "raw"})
        return f"{kind}:{name}:{details}"
    if kind == "user_exit":
        return f"{kind}:{name}:status={event.get('status')}"
    if kind == "smoke_result":
        return f"{kind}:{name}:passed={event.get('passed')}:failed={event.get('failed')}:total={event.get('total')}"
    return f"{kind}:{name}"


def _classifier_rules(data: dict[str, Any]) -> list[dict[str, Any]]:
    unknown = sorted(set(data) - {"rules"})
    if unknown:
        raise CompositeConfigError(f"unknown classifier field(s): {', '.join(unknown)}")
    raw = data.get("rules")
    if not isinstance(raw, list) or not raw:
        raise CompositeConfigError("classifier requires one or more [[rules]]")
    rules: list[dict[str, Any]] = []
    for index, rule in enumerate(raw):
        if not isinstance(rule, dict):
            raise CompositeConfigError(f"classifier rule {index} must be a table")
        unknown_rule = sorted(set(rule) - {"id", "result", "contains", "regex", "description"})
        if unknown_rule:
            raise CompositeConfigError(f"classifier rule {index} unknown field(s): {', '.join(unknown_rule)}")
        rule_id = _required_string(rule, "id", f"classifier rule {index}")
        result = _required_string(rule, "result", f"classifier rule {index}")
        if result not in {"success", "failure"}:
            raise CompositeConfigError(f"classifier rule {rule_id} result must be success or failure")
        contains = _string_list(rule.get("contains", []), f"classifier rule {rule_id}.contains")
        regex = _string_list(rule.get("regex", []), f"classifier rule {rule_id}.regex")
        for expression in regex:
            try:
                re.compile(expression)
            except re.error as error:
                raise CompositeConfigError(f"classifier rule {rule_id} invalid regex: {error}") from error
        rules.append({**rule, "id": rule_id, "result": result, "contains": contains, "regex": regex})
    return rules


def _classify(text: str, returncode: int | None, timed_out: bool, rules: list[dict[str, Any]]) -> dict[str, str]:
    if timed_out:
        return {"id": "timeout", "result": "failure", "description": "basic test timed out"}
    normalized = _normalize_text(text)
    for rule in rules:
        if rule["result"] == "failure" and _rule_matches(rule, normalized):
            return {"id": rule["id"], "result": "failure", "description": str(rule.get("description", ""))}
    if returncode not in (0, None):
        return {"id": "nonzero-exit", "result": "failure", "description": f"basic test returned {returncode}"}
    for rule in rules:
        if rule["result"] == "success" and _rule_matches(rule, normalized):
            return {"id": rule["id"], "result": "success", "description": str(rule.get("description", ""))}
    return {"id": "unknown-failure", "result": "failure", "description": "no success rule matched"}


def _rule_matches(rule: dict[str, Any], text: str) -> bool:
    return all(item in text for item in rule["contains"]) and all(re.search(item, text) for item in rule["regex"])


def _record_sequence(sequences: dict[tuple[str, str, str], dict[str, Any]], run: dict[str, Any]) -> None:
    key = (str(run["result"]), str(run["class_id"]), str(run["sequence_hash"]))
    entry = sequences.get(key)
    if entry is None:
        sequences[key] = {
            "schema_version": SCHEMA_VERSION,
            "result": key[0],
            "class_id": key[1],
            "sequence_hash": key[2],
            "count": 1,
            "first_run": run["run_id"],
            "last_run": run["run_id"],
            "tokens": list(run["sequence_tokens"]),
        }
    else:
        entry["count"] += 1
        entry["last_run"] = run["run_id"]


def _persisted_run_result(run: dict[str, Any]) -> dict[str, Any]:
    return {key: value for key, value in run.items() if key != "events_data"}


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
    completed = len(run_results)
    return {
        "schema_version": SCHEMA_VERSION,
        "case": case_name,
        "dry_run": dry_run,
        "requested_runs": requested_runs,
        "completed_runs": completed,
        "started_at": started.isoformat(),
        "ended_at": ended.isoformat(),
        "total_seconds": round(duration_seconds, 3),
        "average_run_seconds": round(sum(float(run["duration_seconds"]) for run in run_results) / completed, 3) if completed else None,
        "totals": {"success": totals.get("success", 0), "failure": totals.get("failure", 0)},
        "classes": [
            {"result": result, "class_id": class_id, "runs": count, "sequences": sequence_counts.get((result, class_id), 0), "features": _class_features(sequences).get(f"{result}/{class_id}", {})}
            for (result, class_id), count in sorted(class_counts.items())
        ],
        "sequences": [
            {"result": result, "class_id": class_id, "sequence_hash": sequence_hash, "count": entry["count"], "first_run": entry["first_run"], "last_run": entry["last_run"]}
            for (result, class_id, sequence_hash), entry in sorted(sequences.items())
        ],
        "failure_vs_success": _failure_success_comparisons(sequences),
    }


def _class_features(sequences: dict[tuple[str, str, str], dict[str, Any]]) -> dict[str, Any]:
    grouped: dict[tuple[str, str], list[list[str]]] = {}
    for (result, class_id, _), entry in sequences.items():
        grouped.setdefault((result, class_id), []).append(list(entry["tokens"]))
    features: dict[str, Any] = {}
    for (result, class_id), rows in grouped.items():
        sets = [set(row) for row in rows]
        features[f"{result}/{class_id}"] = {
            "common_prefix": _common_prefix(rows),
            "always_events": sorted(set.intersection(*sets) if sets else set()),
            "event_union": sorted(set.union(*sets) if sets else set()),
            "representative_sequence_count": len(rows),
        }
    return features


def _failure_success_comparisons(sequences: dict[tuple[str, str, str], dict[str, Any]]) -> list[dict[str, Any]]:
    successes = [entry for (result, _, _), entry in sequences.items() if result == "success"]
    comparisons: list[dict[str, Any]] = []
    for (result, class_id, sequence_hash), entry in sorted(sequences.items()):
        if result != "failure":
            continue
        if not successes:
            comparisons.append({"failure_class": class_id, "failure_sequence": sequence_hash, "status": "no_success_baseline"})
            continue
        best = max(successes, key=lambda item: _common_prefix_len(entry["tokens"], item["tokens"]))
        index = _common_prefix_len(entry["tokens"], best["tokens"])
        comparisons.append({
            "failure_class": class_id,
            "failure_sequence": sequence_hash,
            "closest_success_sequence": best["sequence_hash"],
            "common_prefix_length": index,
            "failure_event_at_divergence": _token_at(entry["tokens"], index),
            "success_event_at_divergence": _token_at(best["tokens"], index),
        })
    return comparisons


def _checkpoint_sequence_limited(events: list[dict[str, Any]], scope: list[str], max_counts: dict[str, int]) -> list[str]:
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


def _checkpoint_observed_but_not_compared_limited(events: list[dict[str, Any]], scope: list[str], max_counts: dict[str, int]) -> list[dict[str, Any]]:
    scope_set = set(scope)
    counts: Counter[str] = Counter()
    observed: dict[str, dict[str, Any]] = {}
    for event in events:
        if event.get("kind") != "checkpoint":
            continue
        name = str(event.get("name"))
        reason = "outside_checkpoint_scope"
        if name in scope_set:
            counts[name] += 1
            limit = max_counts.get(name)
            if limit is None or counts[name] <= limit:
                continue
            reason = "scope_count_limit"
        entry = observed.setdefault(name, {"name": name, "count": 0, "first_line": event.get("line", 0), "excluded_reason": reason})
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
    return {"index": index, "left": _token_at(left, index), "right": _token_at(right, index)}


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
    left = _checkpoint_sequence_limited(left_events, checkpoint_scope, max_counts)
    right = _checkpoint_sequence_limited(right_events, checkpoint_scope, max_counts)
    missing_left = _ordered_missing(checkpoint_scope, left)
    missing_right = _ordered_missing(checkpoint_scope, right)
    extra_left = _ordered_missing(left, right)
    extra_right = _ordered_missing(right, left)
    divergence = _first_divergence(left, right)
    passed = not missing_left and not missing_right and not extra_left and not extra_right and divergence is None
    return {
        "left_label": left_label,
        "right_label": right_label,
        "checkpoint_scope": checkpoint_scope,
        "checkpoint_scope_max_counts": max_counts,
        "left_sequence": left,
        "right_sequence": right,
        f"missing_from_{left_label}": missing_left,
        f"missing_from_{right_label}": missing_right,
        f"extra_in_{left_label}": extra_left,
        f"extra_in_{right_label}": extra_right,
        "observed_but_not_compared": {
            left_label: _checkpoint_observed_but_not_compared_limited(left_events, checkpoint_scope, max_counts),
            right_label: _checkpoint_observed_but_not_compared_limited(right_events, checkpoint_scope, max_counts),
        },
        "order_mismatch": divergence is not None and not missing_left and not missing_right and not extra_left and not extra_right,
        "first_divergence": divergence,
        "passed": passed,
    }


def _checkpoint_coverage_config(raw: object, repo_root: Path, scope: list[str]) -> dict[str, Any] | None:
    if raw is None:
        return None
    if not isinstance(raw, dict):
        raise CompositeConfigError("checkpoint_coverage must be a table")
    allowed = {"mapping_path", "required_mapping_kinds", "mode", "accounted_outside_scope"}
    unknown = sorted(set(raw) - allowed)
    if unknown:
        raise CompositeConfigError(f"checkpoint_coverage unknown field(s): {', '.join(unknown)}")
    mapping_path = _resolve_repo_path(repo_root, _required_string(raw, "mapping_path", "checkpoint_coverage"))
    kinds = _string_list(raw.get("required_mapping_kinds"), "checkpoint_coverage.required_mapping_kinds")
    mode = _required_string(raw, "mode", "checkpoint_coverage")
    if not kinds or mode != "explicit-accounting":
        raise CompositeConfigError("checkpoint_coverage requires mapping kinds and explicit-accounting mode")
    accounted = _non_empty_string_map(raw.get("accounted_outside_scope", {}), "checkpoint_coverage.accounted_outside_scope")
    audit = _audit_checkpoint_coverage(
        mapping_path=mapping_path,
        required_mapping_kinds=kinds,
        checkpoint_scope=scope,
        accounted_outside_scope=accounted,
        mode=mode,
    )
    return {"mapping_path": mapping_path, "required_mapping_kinds": kinds, "mode": mode, "accounted_outside_scope": accounted, "audit": audit}


def _audit_checkpoint_coverage(
    *, mapping_path: Path, required_mapping_kinds: list[str], checkpoint_scope: list[str],
    accounted_outside_scope: dict[str, str], mode: str,
) -> dict[str, Any]:
    rows = _load_checkpoint_mapping(mapping_path)
    required: list[str] = []
    for number, row in enumerate(rows, 1):
        if _mapping_row_string(row, number, "mapping_kind") in set(required_mapping_kinds):
            name = _mapping_row_string(row, number, "checkpoint_name")
            if name in required:
                raise CompositeConfigError(f"duplicate required checkpoint mapping name: {name}")
            required.append(name)
    scope_set = set(checkpoint_scope)
    required_set = set(required)
    unaccounted = [name for name in required if name not in scope_set and name not in accounted_outside_scope]
    unknown = sorted(name for name in accounted_outside_scope if name not in required_set)
    in_scope = sorted(name for name in accounted_outside_scope if name in scope_set)
    accounted = [name for name in required if name in accounted_outside_scope and name not in scope_set]
    audit = {
        "mapping_path": str(mapping_path), "required_mapping_kinds": required_mapping_kinds, "mode": mode,
        "required_total": len(required), "in_scope": sum(name in scope_set for name in required),
        "accounted_outside_scope": len(accounted), "unaccounted": len(unaccounted),
        "required_checkpoints": required,
        "in_scope_checkpoints": [name for name in required if name in scope_set],
        "accounted_outside_scope_checkpoints": [{"name": name, "reason": accounted_outside_scope[name]} for name in accounted],
        "unaccounted_checkpoints": unaccounted,
        "invalid_accounting": {"unknown_or_not_required": unknown, "already_in_scope": in_scope},
    }
    if unaccounted or unknown or in_scope:
        raise CheckpointCoverageError("checkpoint coverage audit failed", audit)
    return audit


def _load_checkpoint_mapping(path: Path) -> list[dict[str, Any]]:
    try:
        data = json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as error:
        raise CompositeConfigError(f"invalid checkpoint coverage mapping {path}: {error}") from error
    if not isinstance(data, list) or any(not isinstance(row, dict) for row in data):
        raise CompositeConfigError(f"checkpoint coverage mapping must be an array of objects: {path}")
    return data


def _mapping_row_string(row: dict[str, Any], number: int, key: str) -> str:
    value = row.get(key)
    if not isinstance(value, str) or not value:
        raise CompositeConfigError(f"checkpoint coverage mapping row {number} missing string {key}")
    return value


def _checkpoint_coverage_report(config: dict[str, Any]) -> dict[str, Any]:
    audit = config["audit"]
    return {key: audit[key] for key in (
        "mapping_path", "required_mapping_kinds", "mode", "required_total", "in_scope",
        "accounted_outside_scope", "unaccounted", "unaccounted_checkpoints",
    )}


def _write_report(path: Path, case_name: str, summary: dict[str, Any]) -> None:
    lines = [
        f"# Composite Report: {case_name}", "",
        f"- mode: {summary.get('mode')}",
        f"- dry_run: {summary['dry_run']}",
        f"- requested_runs: {summary['requested_runs']}",
        f"- completed_runs: {summary['completed_runs']}",
        f"- success: {summary['totals']['success']}",
        f"- failure: {summary['totals']['failure']}", "", "## Classes", "",
        "| result | class | runs | sequences |", "| --- | --- | ---: | ---: |",
    ]
    lines.extend(
        f"| {item['result']} | {item['class_id']} | {item['runs']} | {item['sequences']} |"
        for item in summary["classes"]
    )
    if not summary["classes"]:
        lines.append("| none | none | 0 | 0 |")
    lines.extend(["", "## Failure Vs Success", ""])
    lines.extend(f"- {item}" for item in summary["failure_vs_success"])
    if not summary["failure_vs_success"]:
        lines.append("No failure sequences to compare.")
    if "paired_checkpoint_diff" in summary:
        lines.extend(["", "## Paired Checkpoint Diff", ""])
        for index, diff in enumerate(summary["paired_checkpoint_diff"], 1):
            lines.append(f"- run {index}: {'passed' if diff['passed'] else 'failed'}")
            lines.append(f"  - first_divergence: {diff['first_divergence']}")
    if "historical_baseline" in summary:
        baseline = summary["historical_baseline"]
        lines.extend(["", "## Historical Baseline", ""])
        lines.append(f"- path: {baseline['path']}")
        lines.append(f"- failure_rate_delta: {baseline['failure_rate_delta']}")
        lines.append(f"- classes_added: {baseline['classes_added']}")
        lines.append(f"- classes_removed: {baseline['classes_removed']}")
        lines.append(f"- sequences_added: {baseline['sequences_added']}")
        lines.append(f"- sequences_removed: {baseline['sequences_removed']}")
        lines.append(f"- recent_sequence_first_divergence: {baseline['recent_sequence_first_divergence']}")
    path.write_text("\n".join(lines) + "\n")


def _case_result(case_name: str, case_path: Path, output_dir: Path, summary: dict[str, Any]) -> dict[str, Any]:
    return {"case_name": case_name, "case_path": str(case_path), "output_dir": str(output_dir), "report_path": str(output_dir / "report.md"), "summary": summary}


def _case_failed(result: dict[str, Any]) -> bool:
    return int(result["summary"]["totals"]["failure"]) > 0


def _print_suite_summary(results: list[dict[str, Any]]) -> None:
    print("\nstress suite summary:", flush=True)
    for result in results:
        totals = result["summary"]["totals"]
        print(f"  {result['case_name']}: success={totals['success']} failure={totals['failure']} report={result['report_path']}", flush=True)


def _load_toml(path: Path) -> dict[str, Any]:
    try:
        value = tomllib.loads(path.read_text())
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise CompositeConfigError(f"cannot parse {path}: {error}") from error
    if not isinstance(value, dict):
        raise CompositeConfigError(f"{path}: top level must be a table")
    return value


def _required_string(data: dict[str, Any], key: str, context: str) -> str:
    value = data.get(key)
    if not isinstance(value, str) or not value:
        raise CompositeConfigError(f"{context}: {key} must be a non-empty string")
    return value


def _string_list(value: object, name: str) -> list[str]:
    if not isinstance(value, list) or any(not isinstance(item, str) or not item for item in value):
        raise CompositeConfigError(f"{name} must be an array of non-empty strings")
    return list(value)


def _positive_integer_map(value: object, name: str) -> dict[str, int]:
    if value is None:
        return {}
    if not isinstance(value, dict):
        raise CompositeConfigError(f"{name} must be a table")
    if any(not isinstance(key, str) or not key or not isinstance(item, int) or isinstance(item, bool) or item <= 0 for key, item in value.items()):
        raise CompositeConfigError(f"{name} entries must be positive integers")
    return dict(value)


def _non_empty_string_map(value: object, name: str) -> dict[str, str]:
    if not isinstance(value, dict) or any(not isinstance(key, str) or not key or not isinstance(item, str) or not item for key, item in value.items()):
        raise CompositeConfigError(f"{name} entries must be non-empty strings")
    return dict(value)


def _resolve_repo_root(value: Path | None) -> Path:
    root = value.resolve() if value else Path(__file__).resolve().parents[4]
    if not (root / "Makefile").is_file():
        raise CompositeConfigError(f"repository root not found: {root}")
    return root


def _resolve_case_path(case_path: Path, value: str) -> Path:
    path = Path(value)
    resolved = path.resolve() if path.is_absolute() else (case_path.parent / path).resolve()
    if not resolved.is_file():
        raise CompositeConfigError(f"referenced file not found: {resolved}")
    return resolved


def _resolve_repo_path(repo_root: Path, value: str) -> Path:
    path = Path(value)
    resolved = path.resolve() if path.is_absolute() else (repo_root / path).resolve()
    if not resolved.is_file():
        raise CompositeConfigError(f"referenced repository file not found: {resolved}")
    return resolved


def _run_dir_name(case_name: str) -> str:
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S.%fZ")
    return f"{stamp}-{case_name}"


def _unique_output_dir(base: Path) -> Path:
    if not base.exists():
        return base
    for suffix in range(1, 1000):
        candidate = base.with_name(f"{base.name}-{suffix}")
        if not candidate.exists():
            return candidate
    raise RuntimeError(f"could not allocate output directory: {base}")


def _sequence_hash(tokens: list[str]) -> str:
    return hashlib.sha256("\n".join(tokens).encode()).hexdigest()[:16]


def _common_prefix(rows: list[list[str]]) -> list[str]:
    if not rows:
        return []
    result = list(rows[0])
    for row in rows[1:]:
        result = result[:_common_prefix_len(result, row)]
    return result


def _common_prefix_len(left: list[str], right: list[str]) -> int:
    index = 0
    while index < len(left) and index < len(right) and left[index] == right[index]:
        index += 1
    return index


def _token_at(tokens: list[str], index: int) -> str | None:
    return tokens[index] if index < len(tokens) else None


def _safe_path(value: str) -> str:
    return re.sub(r"[^A-Za-z0-9._-]+", "_", value)


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _git_output(repo_root: Path, args: list[str]) -> str:
    try:
        completed = subprocess.run(["git", *args], cwd=repo_root, check=False, capture_output=True, text=True)
    except OSError:
        return ""
    return completed.stdout.strip() if completed.returncode == 0 else ""


def _write_json(path: Path, data: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    raise SystemExit(main())
