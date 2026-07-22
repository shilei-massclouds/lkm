"""CLI for the tools2 text renderer."""

from __future__ import annotations

import argparse
from pathlib import Path
import sys

from tools2_common import (
    ProtocolError,
    VIEW_SCHEMA,
    VIEW_VERSION,
    read_json,
    require_protocol,
)

from .text import render_text


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Render a tools2 Signal view.")
    parser.add_argument("view", type=Path)
    parser.add_argument("--format", choices=("text",), default="text")
    parser.add_argument("-o", "--output", type=Path)
    args = parser.parse_args(argv)
    try:
        view = require_protocol(
            read_json(args.view), schema=VIEW_SCHEMA, version=VIEW_VERSION, label="view"
        )
        text = render_text(view)
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(text, encoding="utf-8")
        else:
            sys.stdout.write(text)
        return 0
    except (KeyError, OSError, ProtocolError, ValueError) as exc:
        print(f"lkm-render: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
