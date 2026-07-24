"""CLI for tools2 model/view to offline Signal animation HTML."""

from __future__ import annotations

import argparse
from pathlib import Path
import sys

from tools2_common import (
    MODEL_SCHEMA,
    MODEL_VERSION,
    ProtocolError,
    VIEW_SCHEMA,
    VIEW_VERSION,
    read_json,
    require_protocol,
)

from .builder import build_animation
from .html import write_html


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Build an offline tools2 Signal animation.")
    parser.add_argument("model", type=Path)
    parser.add_argument("view", type=Path)
    parser.add_argument("-o", "--output", type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        model = require_protocol(
            read_json(args.model), schema=MODEL_SCHEMA, version=MODEL_VERSION, label="model"
        )
        view = require_protocol(
            read_json(args.view), schema=VIEW_SCHEMA, version=VIEW_VERSION, label="view"
        )
        write_html(args.output, build_animation(model, view))
        return 0
    except (KeyError, OSError, ProtocolError, ValueError) as exc:
        print(f"lkm-animate: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
