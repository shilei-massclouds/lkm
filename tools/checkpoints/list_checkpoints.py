#!/usr/bin/env python3
from __future__ import annotations

import argparse
import ast
import json
import re
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Iterable


REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_SOURCE = REPO_ROOT / "impl" / "arceos_ex" / "src" / "trace" / "mod.rs"
DEFAULT_OUT_DIR = REPO_ROOT / "tools" / "out" / "checkpoints"
JSON_NAME = "arceos_ex_checkpoints.json"
MARKDOWN_NAME = "arceos_ex_checkpoints.md"


@dataclass(frozen=True)
class CheckpointRecord:
    index: int
    variant: str
    name: str
    early_byte: str | None
    source_file: str


class CheckpointParseError(ValueError):
    pass


def _find_matching_brace(text: str, open_index: int) -> int:
    if open_index < 0 or open_index >= len(text) or text[open_index] != "{":
        raise CheckpointParseError("expected opening brace")

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
            if char == "{":
                depth += 1
            elif char == "}":
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

    raise CheckpointParseError("unclosed brace")


def _extract_braced_body(text: str, marker_pattern: str, label: str) -> str:
    match = re.search(marker_pattern, text, re.MULTILINE | re.DOTALL)
    if match is None:
        raise CheckpointParseError(f"missing {label}")
    open_index = text.find("{", match.end() - 1)
    if open_index == -1:
        raise CheckpointParseError(f"missing opening brace for {label}")
    close_index = _find_matching_brace(text, open_index)
    return text[open_index + 1 : close_index]


def _parse_enum_variants(text: str) -> list[str]:
    body = _extract_braced_body(
        text,
        r"\bpub\s+enum\s+Checkpoint\s*\{",
        "Checkpoint enum",
    )
    variants: list[str] = []
    for raw_part in body.split(","):
        lines = [
            line.strip()
            for line in raw_part.splitlines()
            if line.strip() and not line.strip().startswith("#[")
        ]
        if not lines:
            continue
        candidate = " ".join(lines)
        match = re.match(r"([A-Za-z_][A-Za-z0-9_]*)\b", candidate)
        if match is None:
            raise CheckpointParseError(f"could not parse enum variant: {candidate}")
        variants.append(match.group(1))

    _check_unique(variants, "Checkpoint enum variant")
    if not variants:
        raise CheckpointParseError("Checkpoint enum is empty")
    return variants


def _extract_match_body(text: str, function_name: str) -> str:
    function_match = re.search(
        rf"\bfn\s+{re.escape(function_name)}\s*\([^)]*\).*?\bmatch\s+self\s*\{{",
        text,
        re.MULTILINE | re.DOTALL,
    )
    if function_match is None:
        raise CheckpointParseError(f"missing Checkpoint::{function_name}() match")
    open_index = text.rfind("{", 0, function_match.end())
    if open_index == -1:
        raise CheckpointParseError(f"missing match brace for {function_name}()")
    close_index = _find_matching_brace(text, open_index)
    return text[open_index + 1 : close_index]


def _match_arm_expressions(match_body: str) -> Iterable[tuple[str, str]]:
    arm_pattern = re.compile(
        r"^\s*Self::(?P<variant>[A-Za-z_][A-Za-z0-9_]*)\s*=>\s*(?P<expr>.*?)(?=^\s*(?:Self::[A-Za-z_][A-Za-z0-9_]*|_)\s*=>|\Z)",
        re.MULTILINE | re.DOTALL,
    )
    for arm_match in arm_pattern.finditer(match_body):
        yield arm_match.group("variant"), arm_match.group("expr").strip().rstrip(",").strip()


def _parse_name_mapping(text: str) -> dict[str, str]:
    mapping: dict[str, str] = {}
    for variant, expr in _match_arm_expressions(_extract_match_body(text, "name")):
        literal_match = re.search(r'"(?:\\.|[^"\\])*"', expr)
        if literal_match is None:
            raise CheckpointParseError(f"missing string literal for name arm {variant}")
        name = ast.literal_eval(literal_match.group(0))
        if not isinstance(name, str):
            raise CheckpointParseError(f"name arm {variant} is not a string")
        if variant in mapping:
            raise CheckpointParseError(f"duplicate name arm for {variant}")
        mapping[variant] = name

    if not mapping:
        raise CheckpointParseError("Checkpoint::name() has no Self arms")
    return mapping


def _parse_early_byte_mapping(text: str) -> dict[str, str]:
    mapping: dict[str, str] = {}
    for variant, expr in _match_arm_expressions(_extract_match_body(text, "early_byte")):
        literal_match = re.search(r"b'(?:\\.|[^'\\])+'|b\"(?:\\.|[^\"\\])+\"", expr)
        if literal_match is None:
            raise CheckpointParseError(
                f"missing byte literal for early_byte arm {variant}"
            )
        value = ast.literal_eval(literal_match.group(0))
        if not isinstance(value, bytes) or len(value) != 1:
            raise CheckpointParseError(
                f"early_byte arm {variant} must be one byte"
            )
        if variant in mapping:
            raise CheckpointParseError(f"duplicate early_byte arm for {variant}")
        mapping[variant] = value.decode("ascii")
    return mapping


def _check_unique(values: Iterable[str], label: str) -> None:
    seen: set[str] = set()
    duplicates: list[str] = []
    for value in values:
        if value in seen:
            duplicates.append(value)
        seen.add(value)
    if duplicates:
        joined = ", ".join(sorted(set(duplicates)))
        raise CheckpointParseError(f"duplicate {label}: {joined}")


def _source_file_for(source: Path) -> str:
    resolved = source.resolve()
    try:
        return resolved.relative_to(REPO_ROOT).as_posix()
    except ValueError:
        return resolved.as_posix()


def parse_checkpoints(source: Path = DEFAULT_SOURCE) -> list[CheckpointRecord]:
    text = source.read_text(encoding="utf-8")
    variants = _parse_enum_variants(text)
    names = _parse_name_mapping(text)
    early_bytes = _parse_early_byte_mapping(text)

    variant_set = set(variants)
    missing_names = [variant for variant in variants if variant not in names]
    extra_names = sorted(set(names) - variant_set)
    extra_early_bytes = sorted(set(early_bytes) - variant_set)
    if missing_names:
        raise CheckpointParseError(
            "Checkpoint::name() missing variants: " + ", ".join(missing_names)
        )
    if extra_names:
        raise CheckpointParseError(
            "Checkpoint::name() has unknown variants: " + ", ".join(extra_names)
        )
    if extra_early_bytes:
        raise CheckpointParseError(
            "Checkpoint::early_byte() has unknown variants: "
            + ", ".join(extra_early_bytes)
        )
    _check_unique(names.values(), "checkpoint name")

    source_file = _source_file_for(source)
    return [
        CheckpointRecord(
            index=index,
            variant=variant,
            name=names[variant],
            early_byte=early_bytes.get(variant),
            source_file=source_file,
        )
        for index, variant in enumerate(variants)
    ]


def _markdown_escape(value: str) -> str:
    return value.replace("|", r"\|")


def render_markdown(records: list[CheckpointRecord]) -> str:
    lines = [
        "# arceos_ex Checkpoints",
        "",
        "| index | variant | name | early_byte | source_file |",
        "| ---: | --- | --- | --- | --- |",
    ]
    for record in records:
        early_byte = record.early_byte if record.early_byte is not None else "null"
        lines.append(
            "| "
            + " | ".join(
                [
                    str(record.index),
                    _markdown_escape(record.variant),
                    _markdown_escape(record.name),
                    _markdown_escape(early_byte),
                    _markdown_escape(record.source_file),
                ]
            )
            + " |"
        )
    lines.append("")
    return "\n".join(lines)


def write_outputs(
    records: list[CheckpointRecord],
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


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Export arceos_ex Checkpoint inventory from trace/mod.rs.",
    )
    parser.add_argument(
        "--source",
        type=Path,
        default=DEFAULT_SOURCE,
        help="Rust source containing the Checkpoint enum and name mappings.",
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=DEFAULT_OUT_DIR,
        help="Directory for JSON and Markdown checkpoint inventory outputs.",
    )
    args = parser.parse_args(argv)

    records = parse_checkpoints(args.source)
    json_path, markdown_path = write_outputs(records, args.out_dir)
    print(f"wrote {len(records)} checkpoints")
    print(json_path)
    print(markdown_path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
