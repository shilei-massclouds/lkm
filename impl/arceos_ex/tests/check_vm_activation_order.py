#!/usr/bin/env python3
"""Verify the transient BP/AP translation activation instruction order."""

from __future__ import annotations

import argparse
import re
import subprocess
from pathlib import Path


def disassemble(objdump: str, elf: Path, symbol: str) -> list[str]:
    completed = subprocess.run(
        [objdump, "-d", f"--disassemble-symbols={symbol}", str(elf)],
        check=True,
        capture_output=True,
        text=True,
    )
    instructions: list[str] = []
    for line in completed.stdout.splitlines():
        parts = line.split("\t")
        if len(parts) < 2 or not re.match(r"^\s*[0-9a-f]+:", parts[0]):
            continue
        mnemonic = parts[1].strip()
        operands = parts[2].strip() if len(parts) > 2 else ""
        instructions.append(" ".join((mnemonic, operands)).strip())
    if not instructions:
        raise AssertionError(f"no instructions found for {symbol}")
    return instructions


def require_order(symbol: str, instructions: list[str], patterns: list[str]) -> None:
    cursor = 0
    for pattern in patterns:
        matcher = re.compile(pattern)
        while cursor < len(instructions) and not matcher.fullmatch(instructions[cursor]):
            cursor += 1
        if cursor == len(instructions):
            context = "\n  ".join(instructions)
            raise AssertionError(
                f"{symbol}: missing ordered instruction /{pattern}/ after index {cursor}\n"
                f"  {context}"
            )
        cursor += 1


def require_count(symbol: str, instructions: list[str], pattern: str, expected: int) -> None:
    matcher = re.compile(pattern)
    actual = sum(matcher.fullmatch(instruction) is not None for instruction in instructions)
    if actual != expected:
        raise AssertionError(
            f"{symbol}: /{pattern}/ count is {actual}, expected {expected}"
        )


def check_bp(instructions: list[str]) -> None:
    symbol = "arceos_ex_switch_to_early_vm"
    require_count(symbol, instructions, r"csrw satp, a[01]", 2)
    require_count(symbol, instructions, r"sfence\.vma", 2)
    require_count(symbol, instructions, r"sb \w+, 0x0\(a6\)", 2)
    require_count(symbol, instructions, r"sd \w+, 0x8\(a6\)", 2)
    require_order(
        symbol,
        instructions,
        [
            r"sfence\.vma",
            r"csrw satp, a0",
            r"csrr \w+, satp",
            r"sb \w+, 0x28\(a6\)",
            r"sb \w+, 0x29\(a6\)",
            r"sb \w+, 0x2a\(a6\)",
            r"sb \w+, 0x2b\(a6\)",
            r"sd a0, 0x30\(a6\)",
            r"sd \w+, 0x38\(a6\)",
            r"fence rw, w",
            r"sb \w+, 0x0\(a6\)",
            r"fence rw, w",
            r"sd \w+, 0x8\(a6\)",
            r"csrw satp, a1",
            r"sfence\.vma",
            r"csrr \w+, satp",
            r"sb \w+, 0x40\(a6\)",
            r"sb \w+, 0x41\(a6\)",
            r"sb \w+, 0x42\(a6\)",
            r"sb \w+, 0x43\(a6\)",
            r"sd a1, 0x48\(a6\)",
            r"sd \w+, 0x50\(a6\)",
            r"fence rw, w",
            r"sb \w+, 0x0\(a6\)",
            r"fence rw, w",
            r"sd \w+, 0x8\(a6\)",
        ],
    )


def check_ap(instructions: list[str]) -> None:
    symbol = "arceos_ex_secondary_start_sbi"
    require_count(symbol, instructions, r"csrw satp, t[12]", 2)
    require_count(symbol, instructions, r"sfence\.vma", 3)
    require_count(symbol, instructions, r"sb \w+, 0x0\(t[34]\)", 3)
    require_count(symbol, instructions, r"sd \w+, 0x8\(t[34]\)", 3)
    require_order(
        symbol,
        instructions,
        [
            r"sfence\.vma",
            r"sb zero, 0x10\(t3\)",
            r"sb \w+, 0x11\(t3\)",
            r"sb \w+, 0x12\(t3\)",
            r"sb \w+, 0x13\(t3\)",
            r"sd zero, 0x18\(t3\)",
            r"sd \w+, 0x20\(t3\)",
            r"fence rw, w",
            r"sb \w+, 0x0\(t3\)",
            r"fence rw, w",
            r"sd \w+, 0x8\(t3\)",
            r"sfence\.vma",
            r"csrw satp, t1",
            r"csrr \w+, satp",
            r"sb \w+, 0x28\(t4\)",
            r"sb \w+, 0x29\(t4\)",
            r"sb \w+, 0x2a\(t4\)",
            r"sb \w+, 0x2b\(t4\)",
            r"sd t1, 0x30\(t4\)",
            r"sd \w+, 0x38\(t4\)",
            r"fence rw, w",
            r"sb \w+, 0x0\(t4\)",
            r"fence rw, w",
            r"sd \w+, 0x8\(t4\)",
            r"csrw satp, t2",
            r"sfence\.vma",
            r"csrr \w+, satp",
            r"sb \w+, 0x40\(t4\)",
            r"sb \w+, 0x41\(t4\)",
            r"sb \w+, 0x42\(t4\)",
            r"sb \w+, 0x43\(t4\)",
            r"sd t2, 0x48\(t4\)",
            r"sd \w+, 0x50\(t4\)",
            r"fence rw, w",
            r"sb \w+, 0x0\(t4\)",
            r"fence rw, w",
            r"sd \w+, 0x8\(t4\)",
        ],
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--objdump", required=True)
    parser.add_argument("--elf", required=True, type=Path)
    args = parser.parse_args()

    if not args.elf.is_file():
        parser.error(f"ELF does not exist: {args.elf}")
    check_bp(disassemble(args.objdump, args.elf, "arceos_ex_switch_to_early_vm"))
    check_ap(disassemble(args.objdump, args.elf, "arceos_ex_secondary_start_sbi"))
    print(f"VM activation instruction order verified: {args.elf}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
