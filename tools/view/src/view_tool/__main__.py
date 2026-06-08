"""Command line entry point for the view stage tool."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from common import DERIVE_SCHEMA, DERIVE_VERSION, read_json, write_json
from common.model_json import model_json_to_object_model

from .builder import (
    DEFAULT_TRACE_ACTION_DEPTH,
    build_drives_view,
    build_object_view,
    build_timeline_view,
    build_trace_view,
)
from .view_json import view_to_json


VIEW_CHOICES = ("object", "drives", "timeline", "trace")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Build view JSON from model JSON.")
    parser.add_argument("input", type=Path, help="path to model.json, or derive.json for trace")
    parser.add_argument("view", choices=VIEW_CHOICES, help="view name")
    parser.add_argument(
        "--trace-action-depth",
        type=_trace_action_depth,
        default=DEFAULT_TRACE_ACTION_DEPTH,
        metavar="N",
        help=(
            "maximum nested action depth for trace view; "
            "use 'all' to disable the limit, default: 3"
        ),
    )
    parser.add_argument("-o", "--output", type=Path, required=True, help="path to view.json")
    args = parser.parse_args(argv)

    try:
        input_data = read_json(args.input)
        if args.view == "trace":
            _require_derive_json(input_data)
            view = build_trace_view(input_data, max_action_depth=args.trace_action_depth)
        else:
            model = model_json_to_object_model(input_data)
            view = _build_view(model, args.view)
    except OSError as exc:
        print(f"error: cannot read {args.input}: {exc}", file=sys.stderr)
        return 2
    except ValueError as exc:
        print(f"error: invalid input JSON: {exc}", file=sys.stderr)
        return 2

    write_json(args.output, view_to_json(view, input_data))
    return 0


def _require_derive_json(data: dict) -> None:
    if data.get("schema") != DERIVE_SCHEMA:
        raise ValueError(f"expected schema {DERIVE_SCHEMA!r}")
    if data.get("version") != DERIVE_VERSION:
        raise ValueError(f"expected version {DERIVE_VERSION}")


def _trace_action_depth(value: str) -> int | None:
    if value == "all":
        return None
    try:
        depth = int(value)
    except ValueError as exc:
        raise argparse.ArgumentTypeError("expected a non-negative integer or 'all'") from exc
    if depth < 0:
        raise argparse.ArgumentTypeError("expected a non-negative integer or 'all'")
    return depth


def _build_view(model, name):
    if name == "object":
        return build_object_view(model)
    if name == "drives":
        return build_drives_view(model)
    if name == "timeline":
        return build_timeline_view(model)
    raise ValueError(f"unknown view: {name}")


if __name__ == "__main__":
    raise SystemExit(main())
