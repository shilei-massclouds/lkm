"""CLI for the tools2 view stage."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys

from tools2_common import (
    DERIVE_SCHEMA,
    DERIVE_VERSION,
    PRODUCER,
    ProtocolError,
    VIEW_SCHEMA,
    VIEW_VERSION,
    read_json,
    require_protocol,
    write_json,
)

from .builder import build_view


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Build a tools2 Signal trace view.")
    parser.add_argument("derive", type=Path)
    parser.add_argument("-o", "--output", type=Path)
    args = parser.parse_args(argv)
    try:
        derivation = require_protocol(
            read_json(args.derive), schema=DERIVE_SCHEMA, version=DERIVE_VERSION, label="derive"
        )
        result = {
            "schema": VIEW_SCHEMA,
            "version": VIEW_VERSION,
            "producer": PRODUCER,
            "source": derivation["source"],
            **build_view(derivation),
        }
        if args.output:
            write_json(args.output, result)
        else:
            json.dump(result, sys.stdout, ensure_ascii=False, sort_keys=True, indent=2)
            sys.stdout.write("\n")
        return 0
    except (KeyError, OSError, ProtocolError, ValueError) as exc:
        print(f"lkm-view: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
