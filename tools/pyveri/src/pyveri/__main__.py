"""Command line entry point for pyveri."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from .parser import ParseError, parse_file, summarize


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Parse an LKM object model spec.")
    parser.add_argument("spec", type=Path, help="path to the .spec input file")
    args = parser.parse_args(argv)

    try:
        document = parse_file(args.spec)
    except OSError as exc:
        print(f"error: cannot read {args.spec}: {exc}", file=sys.stderr)
        return 2
    except ParseError as exc:
        print(f"syntax_error: {exc}", file=sys.stderr)
        return 1

    print(summarize(document))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
