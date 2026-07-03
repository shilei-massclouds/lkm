#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import sys
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Iterable


REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_MAPPING = REPO_ROOT / "tools" / "out" / "checkpoints" / "linux_checkpoint_mapping.json"
DEFAULT_OUT_DIR = REPO_ROOT / "tools" / "out" / "checkpoints"
JSON_NAME = "linux_checkpoint_coverage.json"
MARKDOWN_NAME = "linux_checkpoint_coverage.md"
MAPPING_KIND_ORDER = ("exact", "range", "unmapped")


@dataclass(frozen=True)
class LinuxCheckpointMappingRow:
    checkpoint_name: str
    linux_file: str | None
    mapping_kind: str
    confidence: str


@dataclass(frozen=True)
class MappingKindCount:
    mapping_kind: str
    count: int


@dataclass(frozen=True)
class ConfidenceCount:
    confidence: str
    count: int


@dataclass(frozen=True)
class LinuxFileCount:
    linux_file: str
    count: int


@dataclass(frozen=True)
class CheckpointFamilyCount:
    family: str
    count: int


@dataclass(frozen=True)
class LinuxCheckpointCoverage:
    total_checkpoints: int
    mapping_kind_counts: list[MappingKindCount]
    confidence_counts: list[ConfidenceCount]
    mapped_linux_file_counts: list[LinuxFileCount]
    unmapped_checkpoint_family_counts: list[CheckpointFamilyCount]
    unmapped_singleton_family_count: int


class LinuxCheckpointCoverageError(ValueError):
    pass


@dataclass(frozen=True)
class ArtifactDrift:
    path: Path
    reason: str


def _require_str(row: dict[str, object], row_number: int, field: str) -> str:
    value = row.get(field)
    if not isinstance(value, str):
        raise LinuxCheckpointCoverageError(
            f"mapping row {row_number} field {field} must be a string"
        )
    return value


def _require_optional_str(row: dict[str, object], row_number: int, field: str) -> str | None:
    value = row.get(field)
    if value is not None and not isinstance(value, str):
        raise LinuxCheckpointCoverageError(
            f"mapping row {row_number} field {field} must be a string or null"
        )
    return value


def load_mapping(path: Path = DEFAULT_MAPPING) -> list[LinuxCheckpointMappingRow]:
    raw_rows = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(raw_rows, list):
        raise LinuxCheckpointCoverageError("Linux checkpoint mapping must be a JSON array")

    records: list[LinuxCheckpointMappingRow] = []
    for row_number, row in enumerate(raw_rows, start=1):
        if not isinstance(row, dict):
            raise LinuxCheckpointCoverageError(f"mapping row {row_number} must be an object")

        checkpoint_name = _require_str(row, row_number, "checkpoint_name")
        mapping_kind = _require_str(row, row_number, "mapping_kind")
        if mapping_kind not in MAPPING_KIND_ORDER:
            raise LinuxCheckpointCoverageError(
                f"mapping row {row_number} has unknown mapping_kind {mapping_kind!r}"
            )
        records.append(
            LinuxCheckpointMappingRow(
                checkpoint_name=checkpoint_name,
                linux_file=_require_optional_str(row, row_number, "linux_file"),
                mapping_kind=mapping_kind,
                confidence=_require_str(row, row_number, "confidence"),
            )
        )

    return records


def _checkpoint_family(checkpoint_name: str) -> str:
    return checkpoint_name.split(".", 1)[0]


def _sorted_counts(counts: dict[str, int]) -> list[tuple[str, int]]:
    return sorted(counts.items(), key=lambda item: (-item[1], item[0]))


def summarize_mapping(records: Iterable[LinuxCheckpointMappingRow]) -> LinuxCheckpointCoverage:
    rows = list(records)
    mapping_kind_counts = {kind: 0 for kind in MAPPING_KIND_ORDER}
    confidence_counts: dict[str, int] = {}
    linux_file_counts: dict[str, int] = {}
    unmapped_family_counts: dict[str, int] = {}

    for row in rows:
        if row.mapping_kind not in mapping_kind_counts:
            raise LinuxCheckpointCoverageError(f"unknown mapping_kind {row.mapping_kind!r}")
        mapping_kind_counts[row.mapping_kind] += 1
        confidence_counts[row.confidence] = confidence_counts.get(row.confidence, 0) + 1
        if row.linux_file is not None:
            linux_file_counts[row.linux_file] = linux_file_counts.get(row.linux_file, 0) + 1
        if row.mapping_kind == "unmapped":
            family = _checkpoint_family(row.checkpoint_name)
            unmapped_family_counts[family] = unmapped_family_counts.get(family, 0) + 1

    return LinuxCheckpointCoverage(
        total_checkpoints=len(rows),
        mapping_kind_counts=[
            MappingKindCount(mapping_kind=kind, count=mapping_kind_counts[kind])
            for kind in MAPPING_KIND_ORDER
        ],
        confidence_counts=[
            ConfidenceCount(confidence=confidence, count=count)
            for confidence, count in _sorted_counts(confidence_counts)
        ],
        mapped_linux_file_counts=[
            LinuxFileCount(linux_file=linux_file, count=count)
            for linux_file, count in _sorted_counts(linux_file_counts)
        ],
        unmapped_checkpoint_family_counts=[
            CheckpointFamilyCount(family=family, count=count)
            for family, count in _sorted_counts(unmapped_family_counts)
        ],
        unmapped_singleton_family_count=sum(
            1 for count in unmapped_family_counts.values() if count == 1
        ),
    )


def _markdown_escape(value: str) -> str:
    return value.replace("|", r"\|")


def _render_count_table(title: str, key_name: str, rows: Iterable[tuple[str, int]]) -> list[str]:
    lines = [
        f"## {title}",
        "",
        f"| {key_name} | count |",
        "| --- | ---: |",
    ]
    for key, count in rows:
        lines.append(f"| {_markdown_escape(key)} | {count} |")
    lines.append("")
    return lines


def render_json(coverage: LinuxCheckpointCoverage) -> str:
    return json.dumps(asdict(coverage), indent=2, ensure_ascii=False) + "\n"


def render_markdown(coverage: LinuxCheckpointCoverage) -> str:
    lines = [
        "# Linux Checkpoint Mapping Coverage",
        "",
        f"- total checkpoints: {coverage.total_checkpoints}",
        "",
    ]
    lines.extend(
        _render_count_table(
            "Mapping Kind Counts",
            "mapping_kind",
            ((row.mapping_kind, row.count) for row in coverage.mapping_kind_counts),
        )
    )
    lines.extend(
        _render_count_table(
            "Confidence Counts",
            "confidence",
            ((row.confidence, row.count) for row in coverage.confidence_counts),
        )
    )
    lines.extend(
        _render_count_table(
            "Mapped Linux File Counts",
            "linux_file",
            ((row.linux_file, row.count) for row in coverage.mapped_linux_file_counts),
        )
    )
    family_rows = [
        (row.family, row.count)
        for row in coverage.unmapped_checkpoint_family_counts
        if row.count >= 2
    ]
    lines.extend(
        _render_count_table(
            "Unmapped Checkpoint Family Counts",
            "family",
            family_rows,
        )
    )
    lines.append(
        f"- singleton unmapped families: {coverage.unmapped_singleton_family_count}"
    )
    lines.append("")
    return "\n".join(lines)


def _expected_outputs(coverage: LinuxCheckpointCoverage) -> dict[str, str]:
    return {
        JSON_NAME: render_json(coverage),
        MARKDOWN_NAME: render_markdown(coverage),
    }


def write_outputs(
    coverage: LinuxCheckpointCoverage,
    out_dir: Path = DEFAULT_OUT_DIR,
) -> tuple[Path, Path]:
    out_dir.mkdir(parents=True, exist_ok=True)
    json_path = out_dir / JSON_NAME
    markdown_path = out_dir / MARKDOWN_NAME

    outputs = _expected_outputs(coverage)
    json_path.write_text(outputs[JSON_NAME], encoding="utf-8")
    markdown_path.write_text(outputs[MARKDOWN_NAME], encoding="utf-8")
    return json_path, markdown_path


def check_outputs(
    coverage: LinuxCheckpointCoverage,
    out_dir: Path = DEFAULT_OUT_DIR,
) -> list[ArtifactDrift]:
    drift: list[ArtifactDrift] = []
    for name, expected in _expected_outputs(coverage).items():
        path = out_dir / name
        if not path.is_file():
            drift.append(ArtifactDrift(path, "missing"))
            continue
        actual = path.read_text(encoding="utf-8")
        if actual != expected:
            drift.append(ArtifactDrift(path, "content differs"))
    return drift


def _mapping_kind_count(coverage: LinuxCheckpointCoverage, kind: str) -> int:
    for row in coverage.mapping_kind_counts:
        if row.mapping_kind == kind:
            return row.count
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Summarize Linux checkpoint mapping coverage for review.",
    )
    parser.add_argument(
        "--input",
        type=Path,
        default=DEFAULT_MAPPING,
        help="Linux checkpoint mapping JSON.",
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=DEFAULT_OUT_DIR,
        help="Directory for JSON and Markdown coverage outputs.",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Regenerate in memory and fail if tracked outputs have drifted.",
    )
    args = parser.parse_args(argv)

    records = load_mapping(args.input)
    coverage = summarize_mapping(records)
    if args.check:
        drift = check_outputs(coverage, args.out_dir)
        if drift:
            print("Linux checkpoint coverage artifact drift detected:", file=sys.stderr)
            for item in drift:
                print(f"{item.reason}: {item.path}", file=sys.stderr)
            return 1
        print(
            f"Linux checkpoint coverage artifacts are current "
            f"({coverage.total_checkpoints} checkpoints; "
            f"exact={_mapping_kind_count(coverage, 'exact')} "
            f"range={_mapping_kind_count(coverage, 'range')} "
            f"unmapped={_mapping_kind_count(coverage, 'unmapped')})"
        )
        return 0

    json_path, markdown_path = write_outputs(coverage, args.out_dir)
    print(f"wrote Linux checkpoint coverage for {coverage.total_checkpoints} checkpoints")
    print(
        f"exact={_mapping_kind_count(coverage, 'exact')} "
        f"range={_mapping_kind_count(coverage, 'range')} "
        f"unmapped={_mapping_kind_count(coverage, 'unmapped')}"
    )
    print(json_path)
    print(markdown_path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
