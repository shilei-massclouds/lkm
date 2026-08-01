"""Command line entry point for code generation stage tool."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from common import read_json

from .linker import LinkerProfile, generate_riscv64_linker_script


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Generate implementation artifacts from LKM model JSON."
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    linker = subparsers.add_parser(
        "linker-script", help="generate a linker script from model Lds/Config"
    )
    linker.add_argument("model", type=Path, help="path to model.json")
    linker.add_argument("-o", "--output", type=Path, required=True)
    linker.add_argument("--kernel-link-addr", default="0xffffffff80000000")
    linker.add_argument("--page-size", default="4K")
    linker.add_argument("--boot-stack-size", default="16K")

    args = parser.parse_args(argv)
    if args.command == "linker-script":
        return _generate_linker_script(args)
    parser.error(f"unknown command: {args.command}")
    return 2


def _generate_linker_script(args: argparse.Namespace) -> int:
    try:
        raw_model = read_json(args.model)
        profile = LinkerProfile(
            kernel_link_addr=args.kernel_link_addr,
            page_size=args.page_size,
            boot_stack_size=args.boot_stack_size,
        )
        output = generate_riscv64_linker_script(raw_model, profile)
    except (OSError, ValueError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(output, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
