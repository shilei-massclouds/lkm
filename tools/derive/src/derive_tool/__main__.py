"""Command line entry point for the derive stage tool."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from common import read_json, write_json
from common.defaults import DEFAULT_TARGET

from .derive_json import derivation_to_json
from .engine import derive
from .model_json import model_json_to_object_model


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Run derivation from model JSON.")
    parser.add_argument("model", type=Path, help="path to model.json")
    parser.add_argument("-o", "--output", type=Path, required=True, help="path to derive.json")
    parser.add_argument(
        "--target",
        default=DEFAULT_TARGET,
        help=f"target event, default: {DEFAULT_TARGET}",
    )
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

    result = derive(model, args.target)
    write_json(args.output, derivation_to_json(result, model_data))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
