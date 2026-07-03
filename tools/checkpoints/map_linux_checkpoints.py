#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Iterable


REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_INVENTORY = REPO_ROOT / "tools" / "out" / "checkpoints" / "arceos_ex_checkpoints.json"
DEFAULT_LINUX_TREE = REPO_ROOT.parent / "linux-6.12"
DEFAULT_OUT_DIR = REPO_ROOT / "tools" / "out" / "checkpoints"
JSON_NAME = "linux_checkpoint_mapping.json"
MARKDOWN_NAME = "linux_checkpoint_mapping.md"


@dataclass(frozen=True)
class CheckpointInventoryRecord:
    index: int
    variant: str
    name: str


@dataclass(frozen=True)
class LinuxCheckpointMappingRecord:
    checkpoint_index: int
    checkpoint_name: str
    checkpoint_variant: str
    linux_file: str | None
    linux_symbol: str | None
    linux_anchor: str | None
    mapping_kind: str
    confidence: str
    notes: str


@dataclass(frozen=True)
class LinuxSymbol:
    relative_file: str
    name: str
    start_line: int
    end_line: int
    body: str
    body_start_offset: int
    text: str


@dataclass(frozen=True)
class AnchorMatch:
    line: int
    offset: int
    text: str


@dataclass(frozen=True)
class MappingRule:
    mapping_kind: str
    linux_file: str
    linux_symbol: str
    confidence: str
    notes: str
    anchor_pattern: str | None = None
    start_anchor_pattern: str | None = None
    end_anchor_pattern: str | None = None


class LinuxCheckpointMappingError(ValueError):
    pass


def _line_number(text: str, offset: int) -> int:
    return text.count("\n", 0, offset) + 1


def _line_text(text: str, offset: int) -> str:
    line_start = text.rfind("\n", 0, offset) + 1
    line_end = text.find("\n", offset)
    if line_end == -1:
        line_end = len(text)
    return text[line_start:line_end].strip()


def _find_matching_delimiter(text: str, open_index: int, open_char: str, close_char: str) -> int:
    if open_index < 0 or open_index >= len(text) or text[open_index] != open_char:
        raise LinuxCheckpointMappingError(f"expected {open_char!r}")

    depth = 0
    i = open_index
    state = "code"
    while i < len(text):
        char = text[i]
        next_char = text[i + 1] if i + 1 < len(text) else ""

        if state == "code":
            if char == "/" and next_char == "/":
                state = "line_comment"
                i += 2
                continue
            if char == "/" and next_char == "*":
                state = "block_comment"
                i += 2
                continue
            if char == '"':
                state = "string"
                i += 1
                continue
            if char == "'":
                state = "char"
                i += 1
                continue
            if char == open_char:
                depth += 1
            elif char == close_char:
                depth -= 1
                if depth == 0:
                    return i
        elif state == "line_comment":
            if char == "\n":
                state = "code"
        elif state == "block_comment":
            if char == "*" and next_char == "/":
                state = "code"
                i += 2
                continue
        elif state == "string":
            if char == "\\":
                i += 2
                continue
            if char == '"':
                state = "code"
        elif state == "char":
            if char == "\\":
                i += 2
                continue
            if char == "'":
                state = "code"

        i += 1

    raise LinuxCheckpointMappingError(f"unclosed {open_char!r}")


class LinuxSourceIndex:
    def __init__(self, root: Path):
        self.root = root
        self._texts: dict[str, str] = {}

    def read_text(self, relative_file: str) -> str | None:
        if relative_file in self._texts:
            return self._texts[relative_file]
        path = self.root / relative_file
        if not path.is_file():
            return None
        text = path.read_text(encoding="utf-8", errors="replace")
        self._texts[relative_file] = text
        return text

    def find_symbol(self, relative_file: str, symbol: str) -> LinuxSymbol | None:
        text = self.read_text(relative_file)
        if text is None:
            return None

        for match in re.finditer(rf"\b{re.escape(symbol)}\s*\(", text):
            open_paren = text.find("(", match.start(), match.end() + 1)
            if open_paren == -1:
                continue
            try:
                close_paren = _find_matching_delimiter(text, open_paren, "(", ")")
            except LinuxCheckpointMappingError:
                continue

            search_end = min(len(text), close_paren + 512)
            semicolon = text.find(";", close_paren, search_end)
            open_brace = text.find("{", close_paren, search_end)
            if open_brace == -1:
                continue
            if semicolon != -1 and semicolon < open_brace:
                continue

            try:
                close_brace = _find_matching_delimiter(text, open_brace, "{", "}")
            except LinuxCheckpointMappingError:
                continue

            return LinuxSymbol(
                relative_file=relative_file,
                name=symbol,
                start_line=_line_number(text, match.start()),
                end_line=_line_number(text, close_brace),
                body=text[open_brace + 1 : close_brace],
                body_start_offset=open_brace + 1,
                text=text,
            )

        return None


def _find_anchor(symbol: LinuxSymbol, pattern: str) -> AnchorMatch | None:
    match = re.search(pattern, symbol.body, re.MULTILINE)
    if match is None:
        return None
    absolute = symbol.body_start_offset + match.start()
    return AnchorMatch(
        line=_line_number(symbol.text, absolute),
        offset=absolute,
        text=_line_text(symbol.text, absolute),
    )


def _function_anchor(symbol: LinuxSymbol) -> str:
    return f"{symbol.name}() definition line {symbol.start_line}"


def _resolved_anchor(symbol: LinuxSymbol, anchor: AnchorMatch) -> str:
    return f"{symbol.name}() line {anchor.line}: {anchor.text}"


def _unmapped(record: CheckpointInventoryRecord, notes: str) -> LinuxCheckpointMappingRecord:
    return LinuxCheckpointMappingRecord(
        checkpoint_index=record.index,
        checkpoint_name=record.name,
        checkpoint_variant=record.variant,
        linux_file=None,
        linux_symbol=None,
        linux_anchor=None,
        mapping_kind="unmapped",
        confidence="none",
        notes=notes,
    )


def _resolve_rule(
    record: CheckpointInventoryRecord,
    rule: MappingRule,
    linux_sources: LinuxSourceIndex,
) -> LinuxCheckpointMappingRecord:
    symbol = linux_sources.find_symbol(rule.linux_file, rule.linux_symbol)
    if symbol is None:
        return _unmapped(
            record,
            f"Linux symbol {rule.linux_file}::{rule.linux_symbol} was not found.",
        )

    if rule.mapping_kind == "exact":
        if rule.anchor_pattern is None:
            linux_anchor = _function_anchor(symbol)
        else:
            anchor = _find_anchor(symbol, rule.anchor_pattern)
            if anchor is None:
                return _unmapped(
                    record,
                    (
                        f"Linux anchor for {rule.linux_file}::{rule.linux_symbol} "
                        "was not found."
                    ),
                )
            linux_anchor = _resolved_anchor(symbol, anchor)
    elif rule.mapping_kind == "range":
        if rule.start_anchor_pattern is None or rule.end_anchor_pattern is None:
            raise LinuxCheckpointMappingError("range rules require start and end anchors")
        start_anchor = _find_anchor(symbol, rule.start_anchor_pattern)
        end_anchor = _find_anchor(symbol, rule.end_anchor_pattern)
        if start_anchor is None or end_anchor is None:
            return _unmapped(
                record,
                (
                    f"Linux range anchors for {rule.linux_file}::{rule.linux_symbol} "
                    "were not both found."
                ),
            )
        if start_anchor.offset > end_anchor.offset:
            return _unmapped(
                record,
                (
                    f"Linux range anchors for {rule.linux_file}::{rule.linux_symbol} "
                    "were found out of order."
                ),
            )
        linux_anchor = (
            f"{symbol.name}() lines {start_anchor.line}-{end_anchor.line}: "
            f"{start_anchor.text} .. {end_anchor.text}"
        )
    else:
        raise LinuxCheckpointMappingError(f"unsupported mapping kind: {rule.mapping_kind}")

    return LinuxCheckpointMappingRecord(
        checkpoint_index=record.index,
        checkpoint_name=record.name,
        checkpoint_variant=record.variant,
        linux_file=rule.linux_file,
        linux_symbol=rule.linux_symbol,
        linux_anchor=linux_anchor,
        mapping_kind=rule.mapping_kind,
        confidence=rule.confidence,
        notes=rule.notes,
    )


def default_mapping_rules() -> dict[str, MappingRule]:
    return {
        "StartupTimeline.Started": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="start_kernel",
            confidence="high",
            notes="Linux C boot timeline entry anchor.",
        ),
        "MmCoreInitPhase.Started": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="start_kernel",
            anchor_pattern=r"\bmm_core_init\s*\(",
            confidence="high",
            notes="Linux start_kernel() call site for mm_core_init().",
        ),
        "MmCoreInitPhase.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="mm/mm_init.c",
            linux_symbol="mm_core_init",
            confidence="high",
            notes="Linux mm_core_init() function boundary.",
        ),
        "SchedInitPhase.Started": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="start_kernel",
            anchor_pattern=r"\bsched_init\s*\(",
            confidence="high",
            notes="Linux start_kernel() call site for sched_init().",
        ),
        "SchedInitPhase.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="kernel/sched/core.c",
            linux_symbol="sched_init",
            confidence="high",
            notes="Linux sched_init() function boundary.",
        ),
        "BootInitRestInitPhase.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="rest_init",
            confidence="high",
            notes="Linux rest_init() creates init/kthreadd and enters boot idle.",
        ),
        "RootfsPhase.Started": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init_freeable",
            anchor_pattern=r"\bprepare_namespace\s*\(",
            confidence="high",
            notes="Linux kernel_init_freeable() call site for prepare_namespace().",
        ),
        "RootfsPhase.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="init/do_mounts.c",
            linux_symbol="prepare_namespace",
            confidence="high",
            notes="Linux prepare_namespace() rootfs preparation boundary.",
        ),
        "PayloadPhase.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init",
            anchor_pattern=r"\bdo_sysctl_args\s*\(",
            confidence="medium",
            notes="Linux kernel_init() reaches the post-finalize payload-selection boundary.",
        ),
        "PayloadPhase.Online": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init",
            anchor_pattern=r'try_to_run_init_process\s*\(\s*"/sbin/init"',
            confidence="medium",
            notes="Linux kernel_init() default init candidate handoff anchor.",
        ),
    }


def default_unmapped_notes() -> dict[str, str]:
    return {
        "EntryPreludePhase.Started": (
            "EntryPreludePhase starts before this mapping pass claims a portable "
            "Linux init/main.c anchor; RISC-V head.S entry mapping is left for a later pass."
        ),
        "EntryPreludePhase.Ready": (
            "EntryPreludePhase completes before this mapping pass claims a portable "
            "Linux init/main.c anchor; RISC-V head.S entry mapping is left for a later pass."
        ),
    }


def load_inventory(path: Path) -> list[CheckpointInventoryRecord]:
    raw_rows = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(raw_rows, list):
        raise LinuxCheckpointMappingError("checkpoint inventory must be a JSON array")

    records: list[CheckpointInventoryRecord] = []
    for row_number, row in enumerate(raw_rows, start=1):
        if not isinstance(row, dict):
            raise LinuxCheckpointMappingError(f"inventory row {row_number} must be an object")
        try:
            index = row["index"]
            variant = row["variant"]
            name = row["name"]
        except KeyError as exc:
            raise LinuxCheckpointMappingError(
                f"inventory row {row_number} missing field {exc.args[0]}"
            ) from exc
        if not isinstance(index, int) or not isinstance(variant, str) or not isinstance(name, str):
            raise LinuxCheckpointMappingError(
                f"inventory row {row_number} has invalid index/variant/name types"
            )
        records.append(CheckpointInventoryRecord(index=index, variant=variant, name=name))

    return records


def map_checkpoints(
    records: Iterable[CheckpointInventoryRecord],
    linux_tree: Path = DEFAULT_LINUX_TREE,
    rules: dict[str, MappingRule] | None = None,
) -> list[LinuxCheckpointMappingRecord]:
    active_rules = default_mapping_rules() if rules is None else rules
    unmapped_notes = default_unmapped_notes()
    linux_sources = LinuxSourceIndex(linux_tree)
    mapped: list[LinuxCheckpointMappingRecord] = []

    for record in records:
        rule = active_rules.get(record.name)
        if rule is None:
            mapped.append(
                _unmapped(
                    record,
                    unmapped_notes.get(
                        record.name,
                        "No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass.",
                    ),
                )
            )
            continue
        mapped.append(_resolve_rule(record, rule, linux_sources))

    return mapped


def _markdown_escape(value: str) -> str:
    return value.replace("|", r"\|")


def _cell(value: str | None) -> str:
    if value is None:
        return "null"
    return _markdown_escape(value)


def render_markdown(records: list[LinuxCheckpointMappingRecord]) -> str:
    counts = {kind: 0 for kind in ("exact", "range", "unmapped")}
    for record in records:
        counts[record.mapping_kind] = counts.get(record.mapping_kind, 0) + 1

    lines = [
        "# Linux Checkpoint Mapping",
        "",
        f"- exact: {counts.get('exact', 0)}",
        f"- range: {counts.get('range', 0)}",
        f"- unmapped: {counts.get('unmapped', 0)}",
        "",
        "| checkpoint_index | checkpoint_name | checkpoint_variant | mapping_kind | confidence | linux_file | linux_symbol | linux_anchor | notes |",
        "| ---: | --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    for record in records:
        lines.append(
            "| "
            + " | ".join(
                [
                    str(record.checkpoint_index),
                    _cell(record.checkpoint_name),
                    _cell(record.checkpoint_variant),
                    _cell(record.mapping_kind),
                    _cell(record.confidence),
                    _cell(record.linux_file),
                    _cell(record.linux_symbol),
                    _cell(record.linux_anchor),
                    _cell(record.notes),
                ]
            )
            + " |"
        )
    lines.append("")
    return "\n".join(lines)


def write_outputs(
    records: list[LinuxCheckpointMappingRecord],
    out_dir: Path = DEFAULT_OUT_DIR,
) -> tuple[Path, Path]:
    out_dir.mkdir(parents=True, exist_ok=True)
    json_path = out_dir / JSON_NAME
    markdown_path = out_dir / MARKDOWN_NAME

    json_rows = [asdict(record) for record in records]
    json_path.write_text(
        json.dumps(json_rows, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    markdown_path.write_text(render_markdown(records), encoding="utf-8")
    return json_path, markdown_path


def _count_by_kind(records: Iterable[LinuxCheckpointMappingRecord]) -> dict[str, int]:
    counts: dict[str, int] = {}
    for record in records:
        counts[record.mapping_kind] = counts.get(record.mapping_kind, 0) + 1
    return counts


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Map arceos_ex checkpoints to read-only Linux source anchors.",
    )
    parser.add_argument(
        "--input",
        type=Path,
        default=DEFAULT_INVENTORY,
        help="arceos_ex checkpoint inventory JSON.",
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
        help="Directory for JSON and Markdown mapping outputs.",
    )
    args = parser.parse_args(argv)

    inventory = load_inventory(args.input)
    records = map_checkpoints(inventory, args.linux_tree)
    json_path, markdown_path = write_outputs(records, args.out_dir)
    counts = _count_by_kind(records)
    print(f"wrote {len(records)} Linux checkpoint mappings")
    print(f"exact={counts.get('exact', 0)} range={counts.get('range', 0)} unmapped={counts.get('unmapped', 0)}")
    print(json_path)
    print(markdown_path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
