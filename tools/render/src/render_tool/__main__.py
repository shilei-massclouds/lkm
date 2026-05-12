"""Command line entry point for the render stage tool."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from common import read_json
from common.view_json import view_json_to_view_model

from .render import render_view


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Render view JSON to text, DOT, or SVG.")
    parser.add_argument("view", type=Path, help="path to view.json")
    parser.add_argument(
        "--format",
        choices=["text", "dot", "svg"],
        default="text",
        help="output format, default: text",
    )
    parser.add_argument("-o", "--output", type=Path, help="write output to a file")
    args = parser.parse_args(argv)

    try:
        view_data = read_json(args.view)
        view = view_json_to_view_model(view_data)
        output = render_view(view, args.format)
    except OSError as exc:
        print(f"error: cannot read {args.view}: {exc}", file=sys.stderr)
        return 2
    except ValueError as exc:
        print(f"error: cannot render view JSON: {exc}", file=sys.stderr)
        return 2

    if args.output is not None:
        encoding = "ascii" if args.format == "dot" else "utf-8"
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(output + "\n", encoding=encoding)
    else:
        print(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
