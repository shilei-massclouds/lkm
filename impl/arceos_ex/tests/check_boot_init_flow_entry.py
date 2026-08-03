#!/usr/bin/env python3
"""Verify the InterruptType head adoption and BootInitFlow completion ABI."""

from __future__ import annotations

import argparse
import re
import subprocess
from pathlib import Path


SSTATUS_SIE = 1 << 1
SSTATUS_FS_VS = (0b11 << 9) | (0b11 << 13)
EARLY_PHYSICAL_DIRECT = ord("D")
EARLY_BOOT_INIT_STARTED = ord("O")
EARLY_INTERRUPT_PRESET = ord("I")


def run_objdump(objdump: str, *args: str) -> str:
    return subprocess.run(
        [objdump, *args], check=True, capture_output=True, text=True
    ).stdout


def symbols(objdump: str, elf: Path) -> dict[str, tuple[int, int]]:
    result: dict[str, tuple[int, int]] = {}
    for line in run_objdump(objdump, "-t", str(elf)).splitlines():
        parts = line.split()
        if len(parts) < 4 or re.fullmatch(r"[0-9a-f]+", parts[0]) is None:
            continue
        # GNU/LLVM objdump may insert a visibility column (for example
        # ``.hidden``) between the size and symbol name.  The size is the
        # rightmost hexadecimal field before the name, not necessarily the
        # penultimate column.
        size = next(
            (
                int(field, 16)
                for field in reversed(parts[1:-1])
                if re.fullmatch(r"[0-9a-f]+", field) is not None
            ),
            None,
        )
        if size is None:
            continue
        result[parts[-1]] = (int(parts[0], 16), size)
    return result


def disassemble_range(
    objdump: str, elf: Path, start: int, stop: int
) -> list[tuple[int, str]]:
    output = run_objdump(
        objdump,
        "-d",
        f"--start-address=0x{start:x}",
        f"--stop-address=0x{stop:x}",
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
        raise AssertionError(f"no instructions found in [0x{start:x}, 0x{stop:x})")
    return result


def unique_symbol(
    table: dict[str, tuple[int, int]], suffix: str
) -> tuple[str, int, int]:
    matches = [
        (name, address, size)
        for name, (address, size) in table.items()
        if name.endswith(suffix)
    ]
    if len(matches) != 1:
        raise AssertionError(f"expected one symbol ending in {suffix!r}, found {matches}")
    return matches[0]


def loaded_immediate_before(
    body: list[tuple[int, str]], index: int, register: str
) -> int:
    li_pattern = re.compile(rf"li {re.escape(register)}, (0x[0-9a-f]+|\d+)")
    addi_pattern = re.compile(
        rf"addi {re.escape(register)}, {re.escape(register)}, (-?(?:0x[0-9a-f]+|\d+))"
    )
    lui_pattern = re.compile(rf"lui {re.escape(register)}, (0x[0-9a-f]+|\d+)")
    value: int | None = None
    for _, instruction in body[max(0, index - 4) : index]:
        if match := li_pattern.fullmatch(instruction):
            value = int(match.group(1), 0)
        elif match := lui_pattern.fullmatch(instruction):
            value = int(match.group(1), 0) << 12
        elif match := addi_pattern.fullmatch(instruction):
            if value is None:
                continue
            value += int(match.group(1), 0)
    if value is None:
        raise AssertionError(f"cannot recover {register} immediate before instruction {index}")
    return value


def check_head(objdump: str, elf: Path, table: dict[str, tuple[int, int]]) -> None:
    start = table.get("_start", (0, 0))[0]
    secondary = table.get("arceos_ex_secondary_start_sbi", (0, 0))[0]
    if start == 0 or secondary <= start:
        raise AssertionError("_start or the following secondary entry symbol is absent")
    body = disassemble_range(objdump, elf, start, secondary)
    instructions = [instruction for _, instruction in body]

    sie = [index for index, value in enumerate(instructions) if value == "csrw sie, zero"]
    sip = [index for index, value in enumerate(instructions) if value == "csrw sip, zero"]
    if len(sie) != 1 or len(sip) != 1 or sip[0] != sie[0] + 1:
        raise AssertionError("_start must contain adjacent ordered csrw sie, zero -> csrw sip, zero")

    checkpoint_calls = [
        index
        for index, instruction in enumerate(instructions)
        if instruction.endswith("<arceos_ex_head_checkpoint>")
    ]
    early_values = [loaded_immediate_before(body, index, "a0") for index in checkpoint_calls]
    expected = [
        EARLY_PHYSICAL_DIRECT,
        EARLY_BOOT_INIT_STARTED,
        EARLY_INTERRUPT_PRESET,
    ]
    positions = []
    for value in expected:
        if early_values.count(value) != 1:
            raise AssertionError(
                f"early marker 0x{value:x} occurs {early_values.count(value)} times: {early_values}"
            )
        positions.append(early_values.index(value))
    if positions != sorted(positions) or positions[2] != early_values.index(EARLY_INTERRUPT_PRESET):
        raise AssertionError(
            "early markers must order PhysicalDirect -> BootInitFlow.Started -> InterruptType"
        )

    sstatus = [
        (index, match.group(1))
        for index, instruction in enumerate(instructions)
        if (match := re.fullmatch(r"csrc sstatus, (\w+)", instruction))
    ]
    if len(sstatus) != 1:
        raise AssertionError(f"_start must contain one Preset sstatus clear, found {sstatus}")
    index, register = sstatus[0]
    mask = loaded_immediate_before(body, index, register)
    if mask != SSTATUS_FS_VS or mask & SSTATUS_SIE:
        raise AssertionError(
            f"_start Preset sstatus clear mask is 0x{mask:x}, expected FS/VS 0x{SSTATUS_FS_VS:x} without SIE"
        )


def rust_function(source: str, signature: str) -> str:
    start = source.find(signature)
    if start < 0:
        raise AssertionError(f"missing Rust function signature {signature!r}")
    brace = source.find("{", start)
    if brace < 0:
        raise AssertionError(f"missing body for {signature!r}")
    depth = 0
    for index in range(brace, len(source)):
        if source[index] == "{":
            depth += 1
        elif source[index] == "}":
            depth -= 1
            if depth == 0:
                return source[start : index + 1]
    raise AssertionError(f"unterminated body for {signature!r}")


def check_adoption_sources(source_root: Path) -> None:
    interrupt_source = (source_root / "objects/interrupt_type.rs").read_text()
    adoption = rust_function(interrupt_source, "pub fn adopt_head_preset")
    if adoption.count("csr::read_sie()") != 1:
        raise AssertionError("InterruptType adoption must read live sie exactly once")
    forbidden = ["sip", "clear_supervisor_interrupt_pending", "reset_interrupt_handlers", "bind_interrupt_policy"]
    present = [token for token in forbidden if token in adoption]
    if present:
        raise AssertionError(f"InterruptType adoption has forbidden pending/handler work: {present}")

    setup = rust_function(interrupt_source, "pub fn setup(&mut self)")
    disable = setup.find("self.disable()?")
    fallback = setup.find("reset_interrupt_handlers();")
    if not 0 <= disable < fallback:
        raise AssertionError("InterruptType.Setup must close local control before fallback readiness")

    preset_source = (source_root / "flows/boot_init_flow/preset.rs").read_text()
    rust_entry = rust_function(
        preset_source,
        'extern "C" fn boot_init_flow_preset_rust_entry',
    )
    receipt_check = rust_entry.find("head_entry_receipts_valid")
    accept_enable = rust_entry.find("accept_enable_at_entry")
    assign_cpu_ref = rust_entry.find("bind_flow_cpu_ref")
    physical_direct = rust_entry.find("adopt_head_physical_on_cpu")
    preset_accept = rust_entry.find("adopt_head_preset_entry")
    if not 0 <= receipt_check < accept_enable < assign_cpu_ref < physical_direct < preset_accept:
        raise AssertionError(
            "Rust entry must validate receipts then adopt AcceptEnable -> AssignCpuRef -> PhysicalDirect -> Preset"
        )
    if "Checkpoint::BootInitFlowStarted" in rust_entry:
        raise AssertionError("Rust entry must not re-emit BootInitFlow.Started")

    prefix = rust_function(preset_source, "fn adopt_head_prefix")
    bind_task_stack = prefix.find("bind_boot_task_entry")
    current_cpu_setup = prefix.find("setup_boot_cpu")
    local_control_setup = prefix.find("setup_local_control")
    interrupt_setup = prefix.find("local_interrupt.setup()")
    if not 0 <= bind_task_stack < current_cpu_setup < local_control_setup < interrupt_setup:
        raise AssertionError(
            "BootInitFlow adoption must order BindTaskStack -> CurrentCPU.Setup -> local SIE closure -> InterruptType.Setup"
        )

    until_vm = rust_function(preset_source, "fn preset_until_vm_switch")
    prefix_adoption = until_vm.find("adopt_head_prefix")
    trap_preset = until_vm.find("trap.preset")
    vm_preset = until_vm.find("ctx.vm.preset")
    if (
        "ctx.kernel_addr_space\n        .preset" in until_vm
        or not 0 <= prefix_adoption < trap_preset < vm_preset
    ):
        raise AssertionError(
            "BootInitFlow must order its prefix before TrapType.Preset and Vm.Preset without directly driving KernelAddrSpace"
        )

    vm_source = (source_root / "objects/vm.rs").read_text()
    vm_preset_body = rust_function(vm_source, "pub fn preset")
    kernel_addr_preset = vm_preset_body.find("kernel_addr_space.preset")
    trampoline_setup = vm_preset_body.find("self.trampoline_vm\n            .setup")
    if not 0 <= kernel_addr_preset < trampoline_setup:
        raise AssertionError("Vm.Preset must first drive KernelAddrSpace.Preset")

    kernel_source = (source_root / "systems/kernel.rs").read_text()
    accept_body = rust_function(kernel_source, "pub fn accept_enable_at_entry")
    if "cpu_ref() != ctx.cpu_group.boot_cpu_ref()" in accept_body:
        raise AssertionError("AcceptEnable still depends on an already-bound BootInitFlow CpuRef")

    flow_source = (source_root / "flows/boot_init_flow/mod.rs").read_text()
    setup_source = (source_root / "flows/boot_init_flow/setup.rs").read_text()
    if "pub fn setup() -> !" in flow_source + setup_source:
        raise AssertionError("ordinary Rust BootInitFlow setup() bypass remains")
    if "refresh_task_stack_continuation" in flow_source + preset_source + setup_source:
        raise AssertionError("unexpected refresh_task_stack_continuation remains")
    start_kernel = rust_function(setup_source, 'pub extern "C" fn start_kernel')
    if "start_kernel_entry_guard_satisfied()" not in start_kernel:
        raise AssertionError("start_kernel does not enforce the exact entry guard")
    if "run(crate::context::context())" not in start_kernel:
        raise AssertionError("start_kernel does not start the BootInitFlow direct Setup helper")
    if "CorePreparePhaseStarted" in setup_source:
        raise AssertionError("BootInitFlow direct Setup helper emits a phase checkpoint")
    start_guard = rust_function(setup_source, "fn start_kernel_entry_guard_satisfied")
    if "crate::systems::kernel::enable_in_progress()" not in start_guard:
        raise AssertionError("start_kernel guard does not require accepted Kernel.Enable in progress")


def check_task_flow_embedding_sources(source_root: Path) -> None:
    task_source = (source_root / "objects/task.rs").read_text()
    task_body = rust_function(task_source, "pub struct Task {")
    if task_body.count("flow: TaskFlow,") != 1:
        raise AssertionError("Task must physically embed exactly one TaskFlow field")
    if "flow_ref: TaskFlowRef," in task_body:
        raise AssertionError("Task retains a duplicate FlowRef beside its embedded TaskFlow")

    forbidden_structs = (
        "pub struct BootInitFlow",
        "pub struct KernelInitFlow",
        "pub struct KthreaddFlow",
    )
    rust_sources = {
        path: path.read_text() for path in source_root.rglob("*.rs")
    }
    for declaration in forbidden_structs:
        owners = [str(path) for path, source in rust_sources.items() if declaration in source]
        if owners:
            raise AssertionError(f"concrete Flow wrapper remains for {declaration}: {owners}")

    carrier_items = (
        (source_root / "objects/user_boot.rs", "struct UserTaskStorageSlot {"),
        (source_root / "objects/smp_bringup.rs", "struct ApIdleTaskRecord {"),
        (source_root / "objects/scheduler.rs", "pub struct SmokeSchedulerTask {"),
    )
    for path, declaration in carrier_items:
        body = rust_function(path.read_text(), declaration)
        if "flow: TaskFlow" in body or "flow_ref: TaskFlowRef" in body:
            raise AssertionError(
                f"{declaration} retains Task-parallel Flow storage or FlowRef"
            )

    context = rust_function(
        (source_root / "context.rs").read_text(), "pub struct Context {"
    )
    duplicate_fields = (
        "boot_init_flow:",
        "kernel_init_flow:",
        "kthreadd_flow:",
    )
    present = [field for field in duplicate_fields if field in context]
    if present:
        raise AssertionError(f"Context retains Task-parallel Flow fields: {present}")


def check_local_control_setup(
    objdump: str, elf: Path, table: dict[str, tuple[int, int]]
) -> None:
    _, address, size = unique_symbol(table, "InterruptType19setup_local_control")
    body = disassemble_range(objdump, elf, address, address + size)
    instructions = [instruction for _, instruction in body]
    matches = [
        (index, match.group(1))
        for index, instruction in enumerate(instructions)
        if (match := re.fullmatch(r"csrc sstatus, (\w+)", instruction))
    ]
    if len(matches) != 1:
        raise AssertionError("setup_local_control must contain one sstatus clear")
    index, register = matches[0]
    mask = loaded_immediate_before(body, index, register)
    if mask != SSTATUS_SIE:
        raise AssertionError(f"setup_local_control clears 0x{mask:x}, expected SIE")


def check_completion_tail(
    objdump: str, elf: Path, table: dict[str, tuple[int, int]]
) -> None:
    start_kernel = table.get("start_kernel")
    completion = table.get("boot_init_flow_preset_completion")
    if start_kernel is None or completion is None:
        raise AssertionError("stable start_kernel or completion stub symbol is absent")
    address, size = completion
    if size == 0:
        raise AssertionError("completion stub has no ELF size")
    body = disassemble_range(objdump, elf, address, address + size)
    instructions = [instruction for _, instruction in body]
    if len(instructions) != 2:
        raise AssertionError(f"completion stub must lower to exactly two instructions: {instructions}")
    if not re.fullmatch(r"auipc t1, .+", instructions[0]):
        raise AssertionError(f"completion tail does not start with non-linking AUIPC: {instructions}")
    if not re.fullmatch(r"jr .+\(t1\) <start_kernel>", instructions[1]):
        raise AssertionError(f"completion tail does not uniquely target start_kernel: {instructions}")
    if any("ra" in instruction or instruction == "ret" for instruction in instructions):
        raise AssertionError(f"completion tail writes/uses ra or returns: {instructions}")

    whole = run_objdump(objdump, "-d", str(elf))
    references = []
    for line in whole.splitlines():
        if "<start_kernel>" not in line or re.match(r"^\s*[0-9a-f]+:", line) is None:
            continue
        references.append(line.strip())
    if len(references) != 1 or "jr" not in references[0] or "(t1)" not in references[0]:
        raise AssertionError(f"ELF has an ordinary or extra start_kernel reference: {references}")
    if any("refresh_task_stack_continuation" in name for name in table):
        raise AssertionError("ELF contains an extra refresh_task_stack_continuation")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--objdump", required=True)
    parser.add_argument("--elf", required=True, type=Path)
    parser.add_argument("--source-root", required=True, type=Path)
    args = parser.parse_args()
    if not args.elf.is_file():
        parser.error(f"ELF does not exist: {args.elf}")
    if not args.source_root.is_dir():
        parser.error(f"source root does not exist: {args.source_root}")

    table = symbols(args.objdump, args.elf)
    check_head(args.objdump, args.elf, table)
    check_adoption_sources(args.source_root)
    check_task_flow_embedding_sources(args.source_root)
    check_local_control_setup(args.objdump, args.elf, table)
    check_completion_tail(args.objdump, args.elf, table)
    print(
        "BootInitFlow entry verified: ordered sie/sip write, Preset excludes SIE, "
        "PhysicalDirect/Started/Interrupt early order, receipt adoption, Bind/CPU/Trap/Vm order, "
        "Setup SIE closure, Kernel.Enable guard, one embedded TaskFlow per Task, "
        "and non-linking tail start_kernel"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
