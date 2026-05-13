"""Command line entry point for the model stage tool."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from common import read_json, write_json

from .ast_json import ast_json_to_document
from .builder import build_model
from .model_json import build_result_to_model_json


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Build model JSON from AST JSON.")
    parser.add_argument("ast", type=Path, help="path to ast.json")
    parser.add_argument("-o", "--output", type=Path, required=True, help="path to model.json")
    args = parser.parse_args(argv)

    try:
        ast_data = read_json(args.ast)
        document = ast_json_to_document(ast_data)
    except OSError as exc:
        print(f"error: cannot read {args.ast}: {exc}", file=sys.stderr)
        return 2
    except ValueError as exc:
        print(f"error: invalid AST JSON: {exc}", file=sys.stderr)
        return 2

    result = build_model(document)
    for diagnostic in result.diagnostics:
        print(diagnostic.format(), file=sys.stderr)

    write_json(args.output, build_result_to_model_json(result, ast_data))
    return 0 if result.ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
