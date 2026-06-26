#!/usr/bin/env python3
"""Repeat stress cases and cluster observed event sequences."""

from __future__ import annotations

import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import sys
import time
import tomllib
from typing import Any


SCHEMA_VERSION = 1
DEFAULT_CASE = Path(__file__).resolve().parent / "cases" / "df-0001-user-boot.toml"
DEFAULT_OUT_ROOT = Path(__file__).resolve().parent / "out"
ANSI_RE = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")
TRACE_RE = re.compile(r"^trace: (?P<name>[^ ]+)(?: task=(?P<task>[^ ]+))?")
PHASE_ERROR_RE = re.compile(
    r"error=(?P<error>\S+) event=(?P<event>\S+) actual=(?P<actual>\S+) "
    r"expected=(?P<expected>\S+) target=(?P<target>\S+)"
)
SMOKE_RESULT_RE = re.compile(
    r"passed=(?P<passed>\d+) failed=(?P<failed>\d+) total=(?P<total>\d+)"
)
USER_EXIT_RE = re.compile(r"user exit status=(?P<status>-?\d+)")


def main(argv: list[str] | None = None) -> int:
    parser = _build_parser()
    args = parser.parse_args(argv)
    repo_root = _resolve_repo_root(args.repo_root)
    case_path = args.case.resolve()
    case = _load_toml(case_path)
    classifier_path = _resolve_case_path(case_path, _string(case, "classifier"))
    classifier = _load_toml(classifier_path)
    runs = args.runs if args.runs is not None else _integer(case, "default_runs")
    timeout = args.timeout if args.timeout is not None else _integer(case, "timeout_seconds")
    if runs < 0:
        parser.error("--runs must be non-negative")
    if timeout <= 0:
        parser.error("--timeout must be positive")

    case_name = _string(case, "name")
    out_root = args.out_dir.resolve() if args.out_dir else DEFAULT_OUT_ROOT
    output_dir = _unique_output_dir(out_root / _run_dir_name(case_name))
    output_dir.mkdir(parents=True, exist_ok=False)

    command = _string_list(case, "command")
    workdir = _resolve_workdir(repo_root, case.get("working_directory", "."))
    rules = _classifier_rules(classifier)
    manifest = _manifest(case, case_path, classifier_path, command, runs, timeout, repo_root, workdir)
    _write_json(output_dir / "manifest.json", manifest)

    sequences: dict[tuple[str, str, str], dict[str, Any]] = {}
    run_results: list[dict[str, Any]] = []

    if runs == 0 or args.dry_run:
        summary = _build_summary(case_name, runs, run_results, sequences, dry_run=True)
        _write_json(output_dir / "summary.json", summary)
        _write_report(output_dir / "report.md", case_name, summary)
        print(f"stress dry-run wrote {output_dir}")
        return 0

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
            env_updates=_string_map(case.get("env", {}), "env"),
            rules=rules,
        )
        run_results.append(run_result)
        _record_sequence(sequences, run_result)

    _write_sequences(output_dir, sequences)
    summary = _build_summary(case_name, runs, run_results, sequences, dry_run=False)
    _write_json(output_dir / "summary.json", summary)
    _write_report(output_dir / "report.md", case_name, summary)
    print(f"stress report: {output_dir / 'report.md'}")
    if args.fail_on_failure and summary["totals"]["failure"] > 0:
        return 1
    return 0


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Run arceos_ex stress cases.")
    parser.add_argument(
        "case",
        nargs="?",
        type=Path,
        default=DEFAULT_CASE,
        help=f"stress case TOML, default: {DEFAULT_CASE}",
    )
    parser.add_argument("--repo-root", type=Path, help="repository root, auto-detected by default")
    parser.add_argument("--runs", type=int, help="override case default_runs")
    parser.add_argument("--timeout", type=int, help="override case timeout_seconds")
    parser.add_argument("--out-dir", type=Path, help="output root, default: tests/stress/out")
    parser.add_argument("--dry-run", action="store_true", help="validate config and write an empty report")
    parser.add_argument(
        "--fail-on-failure",
        action="store_true",
        help="return non-zero if any run is classified as failure",
    )
    return parser


def _execute_one_run(
    *,
    run_id: str,
    run_dir: Path,
    command: list[str],
    repo_root: Path,
    workdir: Path,
    timeout: int,
    env_updates: dict[str, str],
    rules: list[dict[str, Any]],
) -> dict[str, Any]:
    started = datetime.now(timezone.utc)
    start_monotonic = time.monotonic()
    env = os.environ.copy()
    env.update(env_updates)
    stdout, returncode, timed_out = _run_command_capture(command, workdir, env, timeout)
    ended = datetime.now(timezone.utc)
    duration = time.monotonic() - start_monotonic

    (run_dir / "stdout.log").write_text(stdout, encoding="utf-8", errors="replace")
    (run_dir / "stderr.log").write_text(
        "stderr was merged into stdout.log to preserve event order.\n",
        encoding="utf-8",
    )

    events = _extract_events(stdout)
    sequence_tokens = [_event_token(event) for event in events]
    sequence_hash = _sequence_hash(sequence_tokens)
    classification = _classify(stdout, returncode, timed_out, rules)
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
        "result": classification["result"],
        "class_id": classification["id"],
        "class_description": classification.get("description", ""),
        "sequence_hash": sequence_hash,
        "event_count": len(events),
        "stdout": str(run_dir / "stdout.log"),
        "events": str(run_dir / "events.jsonl"),
    }
    _write_jsonl(run_dir / "events.jsonl", events)
    _write_json(run_dir / "result.json", result)
    _write_json(run_dir / "meta.json", {**result, "repo_root": str(repo_root)})
    return {**result, "events_data": events, "sequence_tokens": sequence_tokens}


def _run_setup_command(
    command: list[str], repo_root: Path, workdir: Path, timeout: int, output_dir: Path
) -> None:
    setup_dir = output_dir / "setup"
    setup_dir.mkdir(parents=True)
    stdout, returncode, timed_out = _run_command_capture(command, workdir, os.environ.copy(), timeout)
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


def _run_command_capture(
    command: list[str], workdir: Path, env: dict[str, str], timeout: int
) -> tuple[str, int | None, bool]:
    process = subprocess.Popen(
        command,
        cwd=workdir,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        start_new_session=True,
    )
    try:
        stdout, _ = process.communicate(timeout=timeout)
        return stdout, process.returncode, False
    except subprocess.TimeoutExpired:
        _kill_process_group(process)
        stdout, _ = process.communicate()
        return stdout, process.returncode, True


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
        line = ANSI_RE.sub("", raw_line).strip()
        if not line:
            continue
        event = _event_from_line(line, line_no)
        if event is not None:
            event["i"] = len(events)
            events.append(event)
    if not events:
        events.append({"i": 0, "line": 0, "kind": "run", "name": "NoObservedEvents"})
    return events


def _event_from_line(line: str, line_no: int) -> dict[str, Any] | None:
    if match := TRACE_RE.match(line):
        event = {
            "line": line_no,
            "kind": "trace",
            "name": match.group("name"),
            "raw": line,
        }
        if match.group("task"):
            event["task"] = match.group("task")
        return event
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


def _event_token(event: dict[str, Any]) -> str:
    kind = str(event.get("kind", "unknown"))
    name = str(event.get("name", "unknown"))
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
    for rule in rules:
        if rule["result"] != "failure":
            continue
        if _rule_matches(rule, text):
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
        if _rule_matches(rule, text):
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
) -> dict[str, Any]:
    totals = Counter(str(run["result"]) for run in run_results)
    class_counts = Counter((str(run["result"]), str(run["class_id"])) for run in run_results)
    sequence_counts = Counter((result, class_id) for result, class_id, _ in sequences)
    class_features = _class_features(sequences)
    comparisons = _failure_success_comparisons(sequences)
    return {
        "schema_version": SCHEMA_VERSION,
        "case": case_name,
        "dry_run": dry_run,
        "requested_runs": requested_runs,
        "completed_runs": len(run_results),
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


def _write_report(path: Path, case_name: str, summary: dict[str, Any]) -> None:
    lines = [
        f"# Stress Report: {case_name}",
        "",
        f"- dry_run: {summary['dry_run']}",
        f"- requested_runs: {summary['requested_runs']}",
        f"- completed_runs: {summary['completed_runs']}",
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
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def _manifest(
    case: dict[str, Any],
    case_path: Path,
    classifier_path: Path,
    command: list[str],
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


if __name__ == "__main__":
    raise SystemExit(main())
