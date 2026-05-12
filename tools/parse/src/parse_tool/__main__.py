"""Command line entry point for the parse stage tool."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from common import write_json

from .ast_json import document_to_ast_json
from .parser import ParseError, parse_file


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Parse an LKM .spec file into AST JSON.")
    parser.add_argument("spec", type=Path, help="path to the .spec input file")
    parser.add_argument("-o", "--output", type=Path, required=True, help="path to ast.json")
    args = parser.parse_args(argv)

    try:
        document = parse_file(args.spec)
    except OSError as exc:
        print(f"error: cannot read {args.spec}: {exc}", file=sys.stderr)
        return 2
    except ParseError as exc:
        print(f"syntax_error: {exc}", file=sys.stderr)
        return 1

    write_json(args.output, document_to_ast_json(document, args.spec))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
