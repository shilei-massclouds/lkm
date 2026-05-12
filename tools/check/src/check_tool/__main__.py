"""Command line entry point for the check stage tool."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from common import read_json, write_json

from .check_json import check_derivation


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Check derivation JSON with a policy.")
    parser.add_argument("derive", type=Path, help="path to derive.json")
    parser.add_argument("-o", "--output", type=Path, required=True, help="path to check.json")
    parser.add_argument(
        "--policy",
        default="default",
        choices=["default"],
        help="verification policy, default: default",
    )
    args = parser.parse_args(argv)

    try:
        derive_data = read_json(args.derive)
        check_data = check_derivation(derive_data, args.policy)
    except OSError as exc:
        print(f"error: cannot read {args.derive}: {exc}", file=sys.stderr)
        return 2
    except ValueError as exc:
        print(f"error: invalid derive JSON: {exc}", file=sys.stderr)
        return 2

    write_json(args.output, check_data)
    _print_summary(check_data)
    return int(check_data["exit_code"])


def _print_summary(check_data: dict[str, object]) -> None:
    print(f"check: {check_data['verdict']}")
    print(f"policy: {check_data['policy']}")
    print(f"target: {check_data['target']}")
    print(f"target_reached: {'yes' if check_data['summary']['target_reached'] else 'no'}")
    print(f"blocked: {check_data['summary']['blocked']}")
    print(f"contradiction: {check_data['summary']['contradiction']}")
    print(f"obligation: {check_data['summary']['obligation']}")
    print(f"deferred: {check_data['summary']['deferred']}")


if __name__ == "__main__":
    raise SystemExit(main())
