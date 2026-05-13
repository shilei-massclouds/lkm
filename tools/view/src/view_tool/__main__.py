"""Command line entry point for the view stage tool."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from common import read_json, write_json
from common.model_json import model_json_to_object_model

from .builder import build_drives_view, build_object_view, build_timeline_view
from .view_json import view_to_json


VIEW_CHOICES = ("object", "drives", "timeline")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Build view JSON from model JSON.")
    parser.add_argument("model", type=Path, help="path to model.json")
    parser.add_argument("view", choices=VIEW_CHOICES, help="view name")
    parser.add_argument("-o", "--output", type=Path, required=True, help="path to view.json")
    args = parser.parse_args(argv)

    try:
        model_data = read_json(args.model)
        model = model_json_to_object_model(model_data)
    except OSError as exc:
        print(f"error: cannot read {args.model}: {exc}", file=sys.stderr)
        return 2
    except ValueError as exc:
        print(f"error: invalid model JSON: {exc}", file=sys.stderr)
        return 2

    view = _build_view(model, args.view)
    write_json(args.output, view_to_json(view, model_data))
    return 0


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
