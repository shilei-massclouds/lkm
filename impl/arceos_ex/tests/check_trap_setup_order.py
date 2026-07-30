#!/usr/bin/env python3
"""Verify the formal stvec and zero sscratch TrapType.Setup commit."""

from __future__ import annotations

import argparse
import re
import subprocess
from pathlib import Path


def run_objdump(objdump: str, *args: str) -> str:
    return subprocess.run(
        [objdump, *args], check=True, capture_output=True, text=True
    ).stdout


def symbols(objdump: str, elf: Path) -> dict[str, tuple[int, int]]:
    result: dict[str, tuple[int, int]] = {}
    for line in run_objdump(objdump, "-t", str(elf)).splitlines():
        parts = line.split()
        if not parts or not re.fullmatch(r"[0-9a-f]+", parts[0]):
            continue
        try:
            section_index = parts.index(".text")
        except ValueError:
            continue
        if section_index + 2 >= len(parts) or not re.fullmatch(
            r"[0-9a-f]+", parts[section_index + 1]
        ):
            continue
        result[parts[-1]] = (
            int(parts[0], 16), int(parts[section_index + 1], 16)
        )
    return result


def instructions(
    objdump: str, elf: Path, start: int, size: int
) -> list[tuple[int, str]]:
    output = run_objdump(
        objdump,
        "-d",
        f"--start-address=0x{start:x}",
        f"--stop-address=0x{start + size:x}",
        str(elf),
    )
    result: list[tuple[int, str]] = []
    for line in output.splitlines():
        parts = line.split("\t")
        match = re.match(r"^\s*([0-9a-f]+):", parts[0])
        if match is None or len(parts) < 2:
            continue
        mnemonic = parts[1].strip()
        operands = parts[2].strip() if len(parts) > 2 else ""
        result.append((int(match.group(1), 16), " ".join((mnemonic, operands)).strip()))
    if not result:
        raise AssertionError("TrapType.Setup has no disassembled instructions")
    return result


def parse_immediate(value: str) -> int:
    return int(value, 0)


def loaded_pc_relative_address(
    body: list[tuple[int, str]], write_index: int, destination: str
) -> int:
    addi_pattern = re.compile(
        rf"addi {re.escape(destination)}, (\w+), (-?(?:0x[0-9a-f]+|\d+))"
    )
    for addi_index in range(write_index - 1, -1, -1):
        match = addi_pattern.fullmatch(body[addi_index][1])
        if match is None:
            continue
        base = match.group(1)
        offset = parse_immediate(match.group(2))
        auipc_pattern = re.compile(
            rf"auipc {re.escape(base)}, (-?(?:0x[0-9a-f]+|\d+))"
        )
        for auipc_index in range(addi_index - 1, -1, -1):
            auipc_match = auipc_pattern.fullmatch(body[auipc_index][1])
            if auipc_match is not None:
                upper = parse_immediate(auipc_match.group(1)) << 12
                return (body[auipc_index][0] + upper + offset) & ((1 << 64) - 1)
        break
    raise AssertionError("TrapType.Setup does not load stvec with a PC-relative function address")


def validate_cpu_formal_entry_table(
    elf: Path,
    table: dict[str, tuple[int, int]],
    body: list[tuple[int, str]],
    write_index: int,
    destination: str,
) -> None:
    names = ["formal_event_entry", *[f"formal_event_entry_cpu{i}" for i in range(1, 16)]]
    addresses = [table.get(name, (0, 0))[0] for name in names]
    if any(address == 0 or address % 4 != 0 for address in addresses):
        raise AssertionError("CPU-indexed formal trap entry table has a missing or unaligned entry")
    encoded_table = b"".join(address.to_bytes(8, "little") for address in addresses)
    if encoded_table not in elf.read_bytes():
        raise AssertionError("TrapType.Setup does not select from the exact CPU formal entry table")
    if not any(
        instruction.startswith(f"ld {destination},")
        for _, instruction in body[:write_index]
    ):
        raise AssertionError("TrapType.Setup stvec source is not loaded from the formal entry table")


def check(objdump: str, elf: Path) -> None:
    table = symbols(objdump, elf)
    setup_names = [
        name
        for name in table
        if "objects9trap_type" in name and name.endswith("TrapType5setup")
    ]
    if len(setup_names) != 1:
        raise AssertionError(f"expected one TrapType.Setup symbol, found {setup_names}")
    formal_address, _ = table.get("formal_event_entry", (0, 0))
    if formal_address == 0 or formal_address % 4 != 0:
        raise AssertionError("formal_event_entry is absent or not four-byte aligned")

    setup_address, setup_size = table[setup_names[0]]
    body = instructions(objdump, elf, setup_address, setup_size)
    exception_indices = [
        index
        for index, (_, instruction) in enumerate(body)
        if "exception_type" in instruction and "ExceptionType6preset" in instruction
    ]
    sscratch_indices = [
        index for index, (_, instruction) in enumerate(body)
        if instruction.startswith("csrw sscratch,")
    ]
    stvec_indices = [
        index for index, (_, instruction) in enumerate(body)
        if re.fullmatch(r"csrw stvec, \w+", instruction)
    ]
    if len(exception_indices) != 1 or len(sscratch_indices) != 1 or len(stvec_indices) != 1:
        raise AssertionError(
            "TrapType.Setup must contain one ExceptionType.Preset call, one sscratch write, "
            "and one stvec write"
        )
    if not exception_indices[0] < stvec_indices[0] < sscratch_indices[0]:
        raise AssertionError(
            "TrapType.Setup must prepare exception fallbacks, publish stvec, then clear sscratch"
        )
    if not re.fullmatch(r"csrw sscratch,\s*zero", body[sscratch_indices[0]][1]):
        raise AssertionError("TrapType.Setup must clear sscratch with the zero register")
    if any(instruction.startswith("csrw satp,") for _, instruction in body):
        raise AssertionError("TrapType.Setup must not write satp")

    stvec_instruction = body[stvec_indices[0]][1]
    stvec_register = stvec_instruction.rsplit(" ", 1)[-1]
    try:
        loaded_address = loaded_pc_relative_address(body, stvec_indices[0], stvec_register)
    except AssertionError:
        validate_cpu_formal_entry_table(
            elf, table, body, stvec_indices[0], stvec_register
        )
    else:
        if loaded_address != formal_address:
            raise AssertionError(
                f"TrapType.Setup writes 0x{loaded_address:x} to stvec, "
                f"expected formal_event_entry at 0x{formal_address:x}"
            )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--objdump", required=True)
    parser.add_argument("--elf", required=True, type=Path)
    args = parser.parse_args()
    if not args.elf.is_file():
        parser.error(f"ELF does not exist: {args.elf}")
    check(args.objdump, args.elf)
    print(f"TrapType.Setup instruction order verified: {args.elf}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
