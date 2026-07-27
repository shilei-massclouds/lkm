"""CLI for the tools2 parser stage."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys

from tools2_common import AST_SCHEMA, AST_VERSION, PRODUCER, stable_source_path, write_json

from .parser import parse_spec


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Parse the tools2 Signal DSL subset.")
    parser.add_argument("spec", type=Path)
    parser.add_argument("-o", "--output", type=Path)
    args = parser.parse_args(argv)
    try:
        document = parse_spec(args.spec)
        result = {
            "schema": AST_SCHEMA,
            "version": AST_VERSION,
            "producer": PRODUCER,
            "source": stable_source_path(args.spec),
            "document": document,
            "summary": {
                "systems": len(document["systems"]),
                "externals": len(document["externals"]),
                "enums": len(document["enums"]),
                "diagnostics": len(document["diagnostics"]),
            },
        }
        if args.output:
            write_json(args.output, result)
        else:
            json.dump(result, sys.stdout, ensure_ascii=False, sort_keys=True, indent=2)
            sys.stdout.write("\n")
        return 0
    except (OSError, ValueError) as exc:
        print(f"lkm-parse: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
