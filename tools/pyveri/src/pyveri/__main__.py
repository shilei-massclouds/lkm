"""Command line entry point for pyveri."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from .model import build_model, summarize_model
from .parser import ParseError, parse_file, summarize
from .view import build_object_view, render_dot, render_text


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Parse an LKM object model spec.")
    parser.add_argument("spec", type=Path, help="path to the .spec input file")
    parser.add_argument(
        "--tree",
        action="store_true",
        help="compatibility alias for --text object",
    )
    parser.add_argument(
        "--text",
        choices=["object"],
        metavar="VIEW",
        help="print a plain text model view",
    )
    parser.add_argument(
        "--graph",
        choices=["object"],
        metavar="VIEW",
        help="print a Graphviz DOT model view",
    )
    parser.add_argument(
        "-o",
        "--output",
        type=Path,
        help="write the selected text or graph view to a file",
    )
    args = parser.parse_args(argv)

    output_modes = [bool(args.tree), bool(args.text), bool(args.graph)]
    if args.output is not None:
        if not any(output_modes):
            parser.error("-o/--output requires --text, --graph, or --tree")
        if sum(output_modes) > 1:
            parser.error("-o/--output can write only one selected view")

    try:
        document = parse_file(args.spec)
    except OSError as exc:
        print(f"error: cannot read {args.spec}: {exc}", file=sys.stderr)
        return 2
    except ParseError as exc:
        print(f"syntax_error: {exc}", file=sys.stderr)
        return 1

    result = build_model(document)
    for diagnostic in result.diagnostics:
        print(diagnostic.format(), file=sys.stderr)

    selected_outputs: list[str] = []

    graph_only = result.ok and args.graph and not args.text and not args.tree
    output_only = args.output is not None
    if not graph_only and not output_only:
        print(summarize(document))
        print(summarize_model(result))

    if result.ok:
        if args.tree:
            selected_outputs.append(render_text(build_object_view(result.model)))
        if args.text:
            selected_outputs.append(render_text(_build_view(result.model, args.text)))
        if args.graph:
            selected_outputs.append(render_dot(_build_view(result.model, args.graph)))

    if args.output is not None and selected_outputs:
        _write_output(args.output, "\n\n".join(selected_outputs), ascii_only=bool(args.graph))
    else:
        for output in selected_outputs:
            print(output)

    return 0 if result.ok else 1


def _build_view(model, name):
    if name == "object":
        return build_object_view(model)
    raise ValueError(f"unknown view: {name}")


def _write_output(path: Path, text: str, ascii_only: bool) -> None:
    encoding = "ascii" if ascii_only else "utf-8"
    path.write_text(text + "\n", encoding=encoding)


if __name__ == "__main__":
    raise SystemExit(main())
