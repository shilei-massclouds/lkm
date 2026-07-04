#!/usr/bin/env python3
from __future__ import annotations

import argparse
import difflib
import hashlib
import json
import os
import re
import sys
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Iterable


REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_MAPPING = REPO_ROOT / "tools" / "out" / "checkpoints" / "linux_checkpoint_mapping.json"
DEFAULT_LINUX_TREE = REPO_ROOT.parent / "linux-6.12"
DEFAULT_OUT_DIR = REPO_ROOT / "tools" / "out" / "checkpoints"
DEFAULT_PLAN = DEFAULT_OUT_DIR / "linux_checkpoint_instrumentation_plan.json"
JSON_NAME = "linux_checkpoint_instrumentation_plan.json"
MARKDOWN_NAME = "linux_checkpoint_instrumentation_plan.md"
MARKER_PREFIX = "LKM_CHECKPOINT"
INSTRUMENTATION_INCLUDE = "#include <linux/lkm_checkpoints.h>"
INSTRUMENTATION_CONFIG = "CONFIG_LKM_CHECKPOINTS"
INSTRUMENTATION_CALL = "LKM_RUNTIME_CHECKPOINT"
FINGERPRINT_PATTERN = re.compile(r"^sha256:[0-9a-f]{64}$")
MARKER_PATTERN = re.compile(
    r"/\*\s*"
    + re.escape(MARKER_PREFIX)
    + r"\s+name=(?P<name>[^\s*/]+)"
    + r"\s+variant=(?P<variant>[^\s*/]+)"
    + r"\s+fingerprint=(?P<fingerprint>sha256:[0-9a-f]{64})"
    + r"\s*\*/"
)
SOURCE_SUFFIXES = (".c", ".h", ".S", ".s")


@dataclass(frozen=True)
class LinuxCheckpointMappingRow:
    checkpoint_index: int
    checkpoint_name: str
    checkpoint_variant: str
    linux_file: str | None
    linux_symbol: str | None
    linux_anchor: str | None
    mapping_kind: str
    confidence: str


@dataclass(frozen=True)
class LinuxInstrumentationPlanEntry:
    checkpoint_index: int
    checkpoint_name: str
    checkpoint_variant: str
    linux_file: str
    linux_symbol: str
    linux_anchor: str
    confidence: str
    marker: str
    anchor_fingerprint: str


@dataclass(frozen=True)
class LinuxMarker:
    checkpoint_name: str
    checkpoint_variant: str
    anchor_fingerprint: str
    linux_file: str
    line: int
    marker: str


@dataclass(frozen=True)
class MarkerCheckProblem:
    kind: str
    checkpoint_name: str
    checkpoint_variant: str
    linux_file: str
    line: int | None
    expected_fingerprint: str | None
    actual_fingerprint: str | None


@dataclass(frozen=True)
class MarkerPatchResult:
    diff: str
    inserted_markers: int
    changed_files: int
    skipped_existing_markers: int


class LinuxInstrumentationPlanError(ValueError):
    pass


@dataclass(frozen=True)
class ArtifactDrift:
    path: Path
    reason: str


def _require_int(row: dict[str, object], row_number: int, field: str) -> int:
    value = row.get(field)
    if not isinstance(value, int):
        raise LinuxInstrumentationPlanError(
            f"mapping row {row_number} field {field} must be an integer"
        )
    return value


def _require_str(row: dict[str, object], row_number: int, field: str) -> str:
    value = row.get(field)
    if not isinstance(value, str):
        raise LinuxInstrumentationPlanError(
            f"mapping row {row_number} field {field} must be a string"
        )
    return value


def _require_optional_str(
    row: dict[str, object],
    row_number: int,
    field: str,
) -> str | None:
    value = row.get(field)
    if value is not None and not isinstance(value, str):
        raise LinuxInstrumentationPlanError(
            f"mapping row {row_number} field {field} must be a string or null"
        )
    return value


def load_mapping(path: Path = DEFAULT_MAPPING) -> list[LinuxCheckpointMappingRow]:
    raw_rows = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(raw_rows, list):
        raise LinuxInstrumentationPlanError("Linux checkpoint mapping must be a JSON array")

    records: list[LinuxCheckpointMappingRow] = []
    for row_number, row in enumerate(raw_rows, start=1):
        if not isinstance(row, dict):
            raise LinuxInstrumentationPlanError(f"mapping row {row_number} must be an object")

        mapping_kind = _require_str(row, row_number, "mapping_kind")
        if mapping_kind not in ("exact", "range", "unmapped"):
            raise LinuxInstrumentationPlanError(
                f"mapping row {row_number} has unknown mapping_kind {mapping_kind!r}"
            )
        records.append(
            LinuxCheckpointMappingRow(
                checkpoint_index=_require_int(row, row_number, "checkpoint_index"),
                checkpoint_name=_require_str(row, row_number, "checkpoint_name"),
                checkpoint_variant=_require_str(row, row_number, "checkpoint_variant"),
                linux_file=_require_optional_str(row, row_number, "linux_file"),
                linux_symbol=_require_optional_str(row, row_number, "linux_symbol"),
                linux_anchor=_require_optional_str(row, row_number, "linux_anchor"),
                mapping_kind=mapping_kind,
                confidence=_require_str(row, row_number, "confidence"),
            )
        )

    return records


def _require_fingerprint(row: dict[str, object], row_number: int, field: str) -> str:
    value = _require_str(row, row_number, field)
    if FINGERPRINT_PATTERN.fullmatch(value) is None:
        raise LinuxInstrumentationPlanError(
            f"plan row {row_number} field {field} must be a sha256 fingerprint"
        )
    return value


def load_plan(path: Path = DEFAULT_PLAN) -> list[LinuxInstrumentationPlanEntry]:
    raw_rows = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(raw_rows, list):
        raise LinuxInstrumentationPlanError("Linux instrumentation plan must be a JSON array")

    plan: list[LinuxInstrumentationPlanEntry] = []
    seen: set[tuple[str, str]] = set()
    for row_number, row in enumerate(raw_rows, start=1):
        if not isinstance(row, dict):
            raise LinuxInstrumentationPlanError(f"plan row {row_number} must be an object")

        checkpoint_index = _require_int(row, row_number, "checkpoint_index")
        checkpoint_name = _require_str(row, row_number, "checkpoint_name")
        checkpoint_variant = _require_str(row, row_number, "checkpoint_variant")
        linux_file = _require_str(row, row_number, "linux_file")
        linux_symbol = _require_str(row, row_number, "linux_symbol")
        linux_anchor = _require_str(row, row_number, "linux_anchor")
        confidence = _require_str(row, row_number, "confidence")
        marker = _require_str(row, row_number, "marker")
        anchor_fingerprint = _require_fingerprint(row, row_number, "anchor_fingerprint")

        identity = (checkpoint_name, checkpoint_variant)
        if identity in seen:
            raise LinuxInstrumentationPlanError(
                "duplicate checkpoint marker identity in plan: "
                f"{checkpoint_name} {checkpoint_variant}"
            )
        seen.add(identity)

        expected_marker = marker_for(checkpoint_name, checkpoint_variant, anchor_fingerprint)
        if marker != expected_marker:
            raise LinuxInstrumentationPlanError(
                f"plan row {row_number} marker does not match checkpoint identity/fingerprint"
            )

        plan.append(
            LinuxInstrumentationPlanEntry(
                checkpoint_index=checkpoint_index,
                checkpoint_name=checkpoint_name,
                checkpoint_variant=checkpoint_variant,
                linux_file=linux_file,
                linux_symbol=linux_symbol,
                linux_anchor=linux_anchor,
                confidence=confidence,
                marker=marker,
                anchor_fingerprint=anchor_fingerprint,
            )
        )

    return plan


def _anchor_line(linux_anchor: str) -> int:
    match = re.search(r"\bline\s+([0-9]+)(?::|\b)", linux_anchor)
    if match is None:
        raise LinuxInstrumentationPlanError(
            f"Linux anchor does not contain a source line: {linux_anchor!r}"
        )
    return int(match.group(1))


def _ignored_context_lines(lines: list[str]) -> set[int]:
    ignored: set[int] = set()
    instrumentation_block_depth = 0
    ignore_next_blank = False
    for index, line in enumerate(lines):
        stripped = line.strip()
        if ignore_next_blank:
            if not stripped:
                ignored.add(index)
                ignore_next_blank = False
                continue
            ignore_next_blank = False
        if INSTRUMENTATION_INCLUDE in line:
            ignored.add(index)
            continue
        if re.match(r"#\s*if(?:n?def)?\b", stripped) and INSTRUMENTATION_CONFIG in stripped:
            instrumentation_block_depth = 1
            ignored.add(index)
            continue
        if instrumentation_block_depth:
            ignored.add(index)
            if re.match(r"#\s*if(?:n?def)?\b", stripped):
                instrumentation_block_depth += 1
            elif re.match(r"#\s*endif\b", stripped):
                instrumentation_block_depth -= 1
                if instrumentation_block_depth == 0:
                    ignore_next_blank = True
            continue
        if INSTRUMENTATION_CALL in line:
            ignored.add(index)
    return ignored


def _context_line_is_ignored(line: str, index: int, ignored_lines: set[int]) -> bool:
    return index in ignored_lines or MARKER_PREFIX in line


def _anchor_physical_index(lines: list[str], line_number: int) -> int:
    ignored_lines = _ignored_context_lines(lines)
    logical_line = 0
    for index, line in enumerate(lines):
        if _context_line_is_ignored(line, index, ignored_lines):
            continue
        logical_line += 1
        if logical_line == line_number:
            return index
    raise LinuxInstrumentationPlanError(
        f"Linux anchor line {line_number} is outside source file with "
        f"{logical_line} non-marker lines"
    )


def _source_context(lines: list[str], line_number: int, radius: int = 2) -> list[str]:
    ignored_lines = _ignored_context_lines(lines)
    anchor_index = _anchor_physical_index(lines, line_number)

    before: list[str] = []
    index = anchor_index - 1
    while index >= 0 and len(before) < radius:
        if not _context_line_is_ignored(lines[index], index, ignored_lines):
            before.append(lines[index].rstrip())
        index -= 1

    after: list[str] = []
    index = anchor_index + 1
    while index < len(lines) and len(after) < radius:
        if not _context_line_is_ignored(lines[index], index, ignored_lines):
            after.append(lines[index].rstrip())
        index += 1

    return list(reversed(before)) + [lines[anchor_index].rstrip()] + after


def anchor_fingerprint(
    linux_tree: Path,
    linux_file: str,
    linux_symbol: str,
    linux_anchor: str,
) -> str:
    path = linux_tree / linux_file
    if not path.is_file():
        raise LinuxInstrumentationPlanError(f"Linux source file is missing: {path}")
    text = path.read_text(encoding="utf-8", errors="replace")
    lines = text.splitlines()
    context = _source_context(lines, _anchor_line(linux_anchor))
    payload = "\n".join(
        [
            f"linux_file={linux_file}",
            f"linux_symbol={linux_symbol}",
            "context:",
            *context,
        ]
    )
    digest = hashlib.sha256(payload.encode("utf-8")).hexdigest()
    return f"sha256:{digest}"


def marker_for(
    checkpoint_name: str,
    checkpoint_variant: str,
    fingerprint: str,
) -> str:
    return (
        f"/* {MARKER_PREFIX} name={checkpoint_name} "
        f"variant={checkpoint_variant} fingerprint={fingerprint} */"
    )


def build_plan(
    records: Iterable[LinuxCheckpointMappingRow],
    linux_tree: Path = DEFAULT_LINUX_TREE,
) -> list[LinuxInstrumentationPlanEntry]:
    plan: list[LinuxInstrumentationPlanEntry] = []
    seen: set[tuple[str, str]] = set()

    for record in records:
        if record.mapping_kind != "exact":
            continue
        if (
            record.linux_file is None
            or record.linux_symbol is None
            or record.linux_anchor is None
        ):
            raise LinuxInstrumentationPlanError(
                f"exact mapping for {record.checkpoint_name} is missing Linux fields"
            )
        identity = (record.checkpoint_name, record.checkpoint_variant)
        if identity in seen:
            raise LinuxInstrumentationPlanError(
                "duplicate checkpoint marker identity: "
                f"{record.checkpoint_name} {record.checkpoint_variant}"
            )
        seen.add(identity)

        fingerprint = anchor_fingerprint(
            linux_tree,
            record.linux_file,
            record.linux_symbol,
            record.linux_anchor,
        )
        plan.append(
            LinuxInstrumentationPlanEntry(
                checkpoint_index=record.checkpoint_index,
                checkpoint_name=record.checkpoint_name,
                checkpoint_variant=record.checkpoint_variant,
                linux_file=record.linux_file,
                linux_symbol=record.linux_symbol,
                linux_anchor=record.linux_anchor,
                confidence=record.confidence,
                marker=marker_for(
                    record.checkpoint_name,
                    record.checkpoint_variant,
                    fingerprint,
                ),
                anchor_fingerprint=fingerprint,
            )
        )

    return plan


def _markdown_escape(value: str) -> str:
    return value.replace("|", r"\|")


def render_json(plan: list[LinuxInstrumentationPlanEntry]) -> str:
    json_rows = [asdict(entry) for entry in plan]
    return json.dumps(json_rows, indent=2, ensure_ascii=False) + "\n"


def render_markdown(plan: list[LinuxInstrumentationPlanEntry]) -> str:
    lines = [
        "# Linux Checkpoint Instrumentation Plan",
        "",
        f"- planned exact checkpoints: {len(plan)}",
        "- marker identity: checkpoint_name + checkpoint_variant",
        "- skipped mapping kinds: range, unmapped",
        "",
        "| checkpoint_index | checkpoint_name | checkpoint_variant | confidence | linux_file | linux_symbol | linux_anchor | marker | anchor_fingerprint |",
        "| ---: | --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    for entry in plan:
        lines.append(
            "| "
            + " | ".join(
                [
                    str(entry.checkpoint_index),
                    _markdown_escape(entry.checkpoint_name),
                    _markdown_escape(entry.checkpoint_variant),
                    _markdown_escape(entry.confidence),
                    _markdown_escape(entry.linux_file),
                    _markdown_escape(entry.linux_symbol),
                    _markdown_escape(entry.linux_anchor),
                    _markdown_escape(entry.marker),
                    _markdown_escape(entry.anchor_fingerprint),
                ]
            )
            + " |"
        )
    lines.append("")
    return "\n".join(lines)


def _expected_outputs(plan: list[LinuxInstrumentationPlanEntry]) -> dict[str, str]:
    return {
        JSON_NAME: render_json(plan),
        MARKDOWN_NAME: render_markdown(plan),
    }


def write_outputs(
    plan: list[LinuxInstrumentationPlanEntry],
    out_dir: Path = DEFAULT_OUT_DIR,
) -> tuple[Path, Path]:
    out_dir.mkdir(parents=True, exist_ok=True)
    json_path = out_dir / JSON_NAME
    markdown_path = out_dir / MARKDOWN_NAME

    outputs = _expected_outputs(plan)
    json_path.write_text(outputs[JSON_NAME], encoding="utf-8")
    markdown_path.write_text(outputs[MARKDOWN_NAME], encoding="utf-8")
    return json_path, markdown_path


def check_outputs(
    plan: list[LinuxInstrumentationPlanEntry],
    out_dir: Path = DEFAULT_OUT_DIR,
) -> list[ArtifactDrift]:
    drift: list[ArtifactDrift] = []
    for name, expected in _expected_outputs(plan).items():
        path = out_dir / name
        if not path.is_file():
            drift.append(ArtifactDrift(path, "missing"))
            continue
        actual = path.read_text(encoding="utf-8")
        if actual != expected:
            drift.append(ArtifactDrift(path, "content differs"))
    return drift


def _scan_path(path: Path, linux_tree: Path) -> list[LinuxMarker]:
    text = path.read_text(encoding="utf-8", errors="replace")
    relative_file = path.relative_to(linux_tree).as_posix()
    markers: list[LinuxMarker] = []
    for match in MARKER_PATTERN.finditer(text):
        markers.append(
            LinuxMarker(
                checkpoint_name=match.group("name"),
                checkpoint_variant=match.group("variant"),
                anchor_fingerprint=match.group("fingerprint"),
                linux_file=relative_file,
                line=text.count("\n", 0, match.start()) + 1,
                marker=match.group(0),
            )
        )
    return markers


def scan_markers(linux_tree: Path) -> list[LinuxMarker]:
    markers: list[LinuxMarker] = []
    for root, dirs, files in os.walk(linux_tree):
        dirs[:] = [item for item in dirs if item not in {".git", ".hg", ".svn"}]
        root_path = Path(root)
        for name in files:
            path = root_path / name
            if path.suffix not in SOURCE_SUFFIXES:
                continue
            try:
                markers.extend(_scan_path(path, linux_tree))
            except OSError as exc:
                raise LinuxInstrumentationPlanError(
                    f"failed to scan Linux marker file {path}: {exc}"
                ) from exc
    return markers


def check_markers(
    plan: Iterable[LinuxInstrumentationPlanEntry],
    linux_tree: Path = DEFAULT_LINUX_TREE,
) -> list[MarkerCheckProblem]:
    expected = {
        (entry.checkpoint_name, entry.checkpoint_variant): entry
        for entry in plan
    }
    actual: dict[tuple[str, str], list[LinuxMarker]] = {}
    for marker in scan_markers(linux_tree):
        actual.setdefault((marker.checkpoint_name, marker.checkpoint_variant), []).append(
            marker
        )

    problems: list[MarkerCheckProblem] = []
    for identity, entry in expected.items():
        markers = actual.get(identity, [])
        if not markers:
            problems.append(
                MarkerCheckProblem(
                    kind="missing marker",
                    checkpoint_name=entry.checkpoint_name,
                    checkpoint_variant=entry.checkpoint_variant,
                    linux_file=entry.linux_file,
                    line=None,
                    expected_fingerprint=entry.anchor_fingerprint,
                    actual_fingerprint=None,
                )
            )
            continue
        for marker in markers:
            if marker.anchor_fingerprint == entry.anchor_fingerprint:
                continue
            problems.append(
                MarkerCheckProblem(
                    kind="fingerprint mismatch",
                    checkpoint_name=entry.checkpoint_name,
                    checkpoint_variant=entry.checkpoint_variant,
                    linux_file=marker.linux_file,
                    line=marker.line,
                    expected_fingerprint=entry.anchor_fingerprint,
                    actual_fingerprint=marker.anchor_fingerprint,
                )
            )

    for identity, markers in actual.items():
        if identity in expected:
            continue
        for marker in markers:
            problems.append(
                MarkerCheckProblem(
                    kind="stale marker",
                    checkpoint_name=marker.checkpoint_name,
                    checkpoint_variant=marker.checkpoint_variant,
                    linux_file=marker.linux_file,
                    line=marker.line,
                    expected_fingerprint=None,
                    actual_fingerprint=marker.anchor_fingerprint,
                )
            )

    return sorted(
        problems,
        key=lambda problem: (
            problem.kind,
            problem.checkpoint_name,
            problem.checkpoint_variant,
            problem.linux_file,
            problem.line or 0,
        ),
    )


def marker_problem_counts(
    problems: Iterable[MarkerCheckProblem],
) -> dict[str, int]:
    counts = {
        "missing marker": 0,
        "stale marker": 0,
        "fingerprint mismatch": 0,
    }
    for problem in problems:
        counts[problem.kind] = counts.get(problem.kind, 0) + 1
    return counts


def format_marker_summary(problems: Iterable[MarkerCheckProblem]) -> str:
    counts = marker_problem_counts(problems)
    return (
        f"{counts.get('missing marker', 0)} missing / "
        f"{counts.get('stale marker', 0)} stale / "
        f"{counts.get('fingerprint mismatch', 0)} mismatch"
    )


def format_marker_problem(problem: MarkerCheckProblem) -> str:
    location = problem.linux_file
    if problem.line is not None:
        location = f"{location}:{problem.line}"
    parts = [
        f"{problem.kind}: {problem.checkpoint_name}",
        f"variant={problem.checkpoint_variant}",
        f"location={location}",
    ]
    if problem.expected_fingerprint is not None:
        parts.append(f"expected={problem.expected_fingerprint}")
    if problem.actual_fingerprint is not None:
        parts.append(f"actual={problem.actual_fingerprint}")
    return " ".join(parts)


def _line_indent(line: str) -> str:
    match = re.match(r"[ \t]*", line)
    if match is None:
        return ""
    return match.group(0)


def _line_ending(line: str) -> str:
    if line.endswith("\r\n"):
        return "\r\n"
    if line.endswith("\n"):
        return "\n"
    if line.endswith("\r"):
        return "\r"
    return "\n"


def _path_is_relative_to(path: Path, parent: Path) -> bool:
    try:
        path.resolve().relative_to(parent.resolve())
    except ValueError:
        return False
    return True


def _anchor_fingerprint_problems(
    plan: Iterable[LinuxInstrumentationPlanEntry],
    linux_tree: Path,
) -> list[MarkerCheckProblem]:
    problems: list[MarkerCheckProblem] = []
    for entry in plan:
        actual = anchor_fingerprint(
            linux_tree,
            entry.linux_file,
            entry.linux_symbol,
            entry.linux_anchor,
        )
        if actual == entry.anchor_fingerprint:
            continue
        problems.append(
            MarkerCheckProblem(
                kind="fingerprint mismatch",
                checkpoint_name=entry.checkpoint_name,
                checkpoint_variant=entry.checkpoint_variant,
                linux_file=entry.linux_file,
                line=_anchor_line(entry.linux_anchor),
                expected_fingerprint=entry.anchor_fingerprint,
                actual_fingerprint=actual,
            )
        )
    return problems


def marker_patch_blockers(
    plan: list[LinuxInstrumentationPlanEntry],
    linux_tree: Path,
) -> list[MarkerCheckProblem]:
    problems = [
        problem
        for problem in check_markers(plan, linux_tree)
        if problem.kind != "missing marker"
    ]
    problems.extend(_anchor_fingerprint_problems(plan, linux_tree))
    return sorted(
        problems,
        key=lambda problem: (
            problem.kind,
            problem.checkpoint_name,
            problem.checkpoint_variant,
            problem.linux_file,
            problem.line or 0,
        ),
    )


def render_marker_patch(
    plan: list[LinuxInstrumentationPlanEntry],
    linux_tree: Path = DEFAULT_LINUX_TREE,
    check_blockers: bool = True,
) -> MarkerPatchResult:
    if check_blockers:
        blockers = marker_patch_blockers(plan, linux_tree)
        if blockers:
            raise LinuxInstrumentationPlanError(
                "refusing to emit Linux marker patch with blocking marker problems: "
                + format_marker_summary(blockers)
            )

    existing_markers = {marker.marker for marker in scan_markers(linux_tree)}
    by_file: dict[str, dict[int, list[LinuxInstrumentationPlanEntry]]] = {}
    skipped_existing = 0
    for entry in plan:
        if entry.marker in existing_markers:
            skipped_existing += 1
            continue
        anchor_line = _anchor_line(entry.linux_anchor)
        by_file.setdefault(entry.linux_file, {}).setdefault(anchor_line, []).append(entry)

    diff_parts: list[str] = []
    inserted_markers = 0
    changed_files = 0
    for linux_file in sorted(by_file):
        path = linux_tree / linux_file
        if not path.is_file():
            raise LinuxInstrumentationPlanError(f"Linux source file is missing: {path}")

        original_text = path.read_text(encoding="utf-8", errors="replace")
        original_lines = original_text.splitlines(keepends=True)
        insertions: dict[int, list[str]] = {}
        for anchor_line, entries in by_file[linux_file].items():
            anchor_index = _anchor_physical_index(original_lines, anchor_line)
            anchor_source_line = original_lines[anchor_index]
            indent = _line_indent(anchor_source_line)
            ending = _line_ending(anchor_source_line)
            sorted_entries = sorted(
                entries,
                key=lambda entry: (
                    entry.checkpoint_index,
                    entry.checkpoint_name,
                    entry.checkpoint_variant,
                ),
            )
            insertions[anchor_index] = [
                f"{indent}{entry.marker}{ending}" for entry in sorted_entries
            ]
            inserted_markers += len(sorted_entries)

        new_lines: list[str] = []
        for index, line in enumerate(original_lines):
            new_lines.extend(insertions.get(index, []))
            new_lines.append(line)
        new_text = "".join(new_lines)
        if new_text == original_text:
            continue

        changed_files += 1
        diff_lines = difflib.unified_diff(
            original_text.splitlines(),
            new_text.splitlines(),
            fromfile=f"a/{linux_file}",
            tofile=f"b/{linux_file}",
            lineterm="",
        )
        diff_parts.append("\n".join(diff_lines) + "\n")

    return MarkerPatchResult(
        diff="".join(diff_parts),
        inserted_markers=inserted_markers,
        changed_files=changed_files,
        skipped_existing_markers=skipped_existing,
    )


def write_marker_patch(
    plan: list[LinuxInstrumentationPlanEntry],
    linux_tree: Path,
    patch_path: Path,
    check_blockers: bool = True,
) -> MarkerPatchResult:
    if _path_is_relative_to(patch_path, linux_tree):
        raise LinuxInstrumentationPlanError(
            f"refusing to write marker patch inside Linux tree: {patch_path}"
        )

    result = render_marker_patch(plan, linux_tree, check_blockers=check_blockers)
    patch_path.parent.mkdir(parents=True, exist_ok=True)
    patch_path.write_text(result.diff, encoding="utf-8")
    return result


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Plan Linux checkpoint instrumentation markers from exact mappings.",
    )
    parser.add_argument(
        "--input",
        type=Path,
        default=DEFAULT_MAPPING,
        help="Linux checkpoint mapping JSON.",
    )
    parser.add_argument(
        "--linux-tree",
        type=Path,
        default=DEFAULT_LINUX_TREE,
        help="Read-only Linux reference source tree.",
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=DEFAULT_OUT_DIR,
        help="Directory for JSON and Markdown instrumentation plan outputs.",
    )
    parser.add_argument(
        "--plan",
        type=Path,
        default=None,
        help=(
            "Instrumentation plan JSON for --emit-marker-patch "
            "(default: <out-dir>/linux_checkpoint_instrumentation_plan.json)."
        ),
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Regenerate in memory and fail if tracked outputs have drifted.",
    )
    parser.add_argument(
        "--check-markers",
        action="store_true",
        help="Scan the Linux tree for missing, stale or moved checkpoint markers.",
    )
    parser.add_argument(
        "--emit-marker-patch",
        type=Path,
        default=None,
        help="Write a reviewable unified diff that inserts missing Linux markers.",
    )
    args = parser.parse_args(argv)

    if args.emit_marker_patch is not None and (args.check or args.check_markers):
        print("--emit-marker-patch cannot be combined with --check or --check-markers", file=sys.stderr)
        return 1

    if not args.linux_tree.is_dir():
        print(f"Linux reference tree is missing: {args.linux_tree}", file=sys.stderr)
        return 1

    try:
        if args.emit_marker_patch is not None:
            plan_path = args.plan if args.plan is not None else args.out_dir / JSON_NAME
            plan = load_plan(plan_path)
            blockers = marker_patch_blockers(plan, args.linux_tree)
            if blockers:
                print("Linux checkpoint marker patch generation failed:", file=sys.stderr)
                print(f"marker summary: {format_marker_summary(blockers)}", file=sys.stderr)
                for problem in blockers:
                    print(format_marker_problem(problem), file=sys.stderr)
                return 1

            result = write_marker_patch(
                plan,
                args.linux_tree,
                args.emit_marker_patch,
                check_blockers=False,
            )
            print(
                "wrote Linux checkpoint marker patch "
                f"({result.inserted_markers} insertions, "
                f"{result.skipped_existing_markers} existing, "
                f"{result.changed_files} files changed)"
            )
            print(args.emit_marker_patch)
            return 0

        records = load_mapping(args.input)
        plan = build_plan(records, args.linux_tree)

        if args.check:
            drift = check_outputs(plan, args.out_dir)
            if drift:
                print("Linux checkpoint instrumentation plan artifact drift detected:", file=sys.stderr)
                for item in drift:
                    print(f"{item.reason}: {item.path}", file=sys.stderr)
                return 1
            print(
                "Linux checkpoint instrumentation plan artifacts are current "
                f"({len(plan)} planned exact checkpoints)"
            )
            if not args.check_markers:
                return 0

        if args.check_markers:
            problems = check_markers(plan, args.linux_tree)
            if problems:
                print("Linux checkpoint marker check failed:", file=sys.stderr)
                print(f"marker summary: {format_marker_summary(problems)}", file=sys.stderr)
                for problem in problems:
                    print(format_marker_problem(problem), file=sys.stderr)
                return 1
            print("marker summary: 0 missing / 0 stale / 0 mismatch")
            print(f"Linux checkpoint markers are current ({len(plan)} planned markers)")
            return 0

        json_path, markdown_path = write_outputs(plan, args.out_dir)
        print(f"wrote Linux checkpoint instrumentation plan for {len(plan)} exact mappings")
        print(json_path)
        print(markdown_path)
        return 0
    except (json.JSONDecodeError, OSError, LinuxInstrumentationPlanError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
