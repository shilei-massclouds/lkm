"""CLI for the tools2 model stage."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys

from tools2_common import (
    AST_SCHEMA,
    AST_VERSION,
    MODEL_SCHEMA,
    MODEL_VERSION,
    PRODUCER,
    ProtocolError,
    read_json,
    require_protocol,
    write_json,
)

from .builder import build_model, model_fingerprint


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Build the tools2 Signal model.")
    parser.add_argument("ast", type=Path)
    parser.add_argument("-o", "--output", type=Path)
    args = parser.parse_args(argv)
    try:
        ast = require_protocol(
            read_json(args.ast), schema=AST_SCHEMA, version=AST_VERSION, label="AST"
        )
        model, diagnostics = build_model(ast["document"])
        result = {
            "schema": MODEL_SCHEMA,
            "version": MODEL_VERSION,
            "producer": PRODUCER,
            "source": ast["source"],
            "model_fingerprint": model_fingerprint(model),
            "model": model,
            "diagnostics": diagnostics,
            "summary": {
                "systems": len(model["systems"]),
                "handlers": sum(
                    len(state["handlers"])
                    for system in model["systems"].values()
                    for state in system["states"].values()
                ),
                "errors": sum(item["category"] == "error" for item in diagnostics),
                "unsupported": sum(item["category"] == "unsupported" for item in diagnostics),
                "ok": not diagnostics,
            },
        }
        if args.output:
            write_json(args.output, result)
        else:
            json.dump(result, sys.stdout, ensure_ascii=False, sort_keys=True, indent=2)
            sys.stdout.write("\n")
        return 0
    except (KeyError, OSError, ProtocolError, ValueError) as exc:
        print(f"lkm-model: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
