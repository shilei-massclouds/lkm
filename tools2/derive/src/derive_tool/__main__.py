"""CLI for the tools2 Signal derivation stage."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys

from tools2_common import (
    DERIVE_SCHEMA,
    DERIVE_VERSION,
    MODEL_SCHEMA,
    MODEL_VERSION,
    PRODUCER,
    ProtocolError,
    read_json,
    require_protocol,
    write_json,
)

from .engine import DerivationProblem, derive, parse_budget


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Run bounded tools2 Signal derivation.")
    parser.add_argument("model", type=Path)
    parser.add_argument("--signal", required=True, help="root Signal as Target.SignalName")
    parser.add_argument("--source", default="Environment")
    parser.add_argument("--scenario", type=Path)
    parser.add_argument("--max-depth", type=parse_budget, default=3, metavar="N|all")
    parser.add_argument("--max-breadth", type=parse_budget, default=3, metavar="N|all")
    parser.add_argument("-o", "--output", type=Path)
    args = parser.parse_args(argv)
    try:
        model = require_protocol(
            read_json(args.model), schema=MODEL_SCHEMA, version=MODEL_VERSION, label="model"
        )
        derivation = derive(
            model,
            signal=args.signal,
            source=args.source,
            scenario=args.scenario,
            max_depth=args.max_depth,
            max_breadth=args.max_breadth,
        )
        result = {
            "schema": DERIVE_SCHEMA,
            "version": DERIVE_VERSION,
            "producer": PRODUCER,
            "source": model["source"],
            **derivation,
        }
        if args.output:
            write_json(args.output, result)
        else:
            json.dump(result, sys.stdout, ensure_ascii=False, sort_keys=True, indent=2)
            sys.stdout.write("\n")
        return 0
    except (KeyError, OSError, ProtocolError, DerivationProblem, ValueError) as exc:
        print(f"lkm-derive: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
