"""CLI for the tools2 check stage."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys

from tools2_common import (
    CHECK_SCHEMA,
    CHECK_VERSION,
    DERIVE_SCHEMA,
    DERIVE_VERSION,
    PRODUCER,
    ProtocolError,
    read_json,
    require_protocol,
    write_json,
)

from .policy import check_derivation


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Check a tools2 Signal derivation.")
    parser.add_argument("derive", type=Path)
    parser.add_argument("-o", "--output", type=Path)
    args = parser.parse_args(argv)
    try:
        derivation = require_protocol(
            read_json(args.derive), schema=DERIVE_SCHEMA, version=DERIVE_VERSION, label="derive"
        )
        checked = check_derivation(derivation)
        result = {
            "schema": CHECK_SCHEMA,
            "version": CHECK_VERSION,
            "producer": PRODUCER,
            "source": derivation["source"],
            **checked,
        }
        if args.output:
            write_json(args.output, result)
        else:
            json.dump(result, sys.stdout, ensure_ascii=False, sort_keys=True, indent=2)
            sys.stdout.write("\n")
        return result["exit_code"]
    except (KeyError, OSError, ProtocolError, ValueError) as exc:
        print(f"lkm-check: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
