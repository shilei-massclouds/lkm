#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Iterable


REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_INVENTORY = REPO_ROOT / "tools" / "out" / "checkpoints" / "arceos_ex_checkpoints.json"
DEFAULT_LINUX_TREE = REPO_ROOT.parent / "linux-6.12"
DEFAULT_OUT_DIR = REPO_ROOT / "tools" / "out" / "checkpoints"
JSON_NAME = "linux_checkpoint_mapping.json"
MARKDOWN_NAME = "linux_checkpoint_mapping.md"
INSTRUMENTATION_INCLUDE = "#include <linux/lkm_checkpoints.h>"
INSTRUMENTATION_CONFIG = "CONFIG_LKM_CHECKPOINTS"
INSTRUMENTATION_CALL = "LKM_RUNTIME_CHECKPOINT"
INSTRUMENTATION_RECORD_CALL = "lkm_checkpoint_record("
LKM_CHECKPOINT_MARKER_LINE_RE = re.compile(
    r"^[ \t]*/\* LKM_CHECKPOINT\b.*\*/[ \t]*(?:\r?\n|\r)?$"
)


@dataclass(frozen=True)
class CheckpointInventoryRecord:
    index: int
    variant: str
    name: str


@dataclass(frozen=True)
class LinuxCheckpointMappingRecord:
    checkpoint_index: int
    checkpoint_name: str
    checkpoint_variant: str
    linux_file: str | None
    linux_symbol: str | None
    linux_anchor: str | None
    mapping_kind: str
    confidence: str
    notes: str


@dataclass(frozen=True)
class LinuxSymbol:
    relative_file: str
    name: str
    kind: str
    start_line: int
    end_line: int
    body: str
    body_start_offset: int
    text: str


@dataclass(frozen=True)
class AnchorMatch:
    line: int
    offset: int
    text: str


@dataclass(frozen=True)
class MappingRule:
    mapping_kind: str
    linux_file: str
    linux_symbol: str
    confidence: str
    notes: str
    anchor_pattern: str | None = None
    start_anchor_pattern: str | None = None
    end_anchor_pattern: str | None = None


class LinuxCheckpointMappingError(ValueError):
    pass


@dataclass(frozen=True)
class ArtifactDrift:
    path: Path
    reason: str


def _line_number(text: str, offset: int) -> int:
    return text.count("\n", 0, offset) + 1


def _line_text(text: str, offset: int) -> str:
    line_start = text.rfind("\n", 0, offset) + 1
    line_end = text.find("\n", offset)
    if line_end == -1:
        line_end = len(text)
    return text[line_start:line_end].strip()


def _strip_lkm_checkpoint_marker_lines(text: str) -> str:
    return "".join(
        line
        for line in text.splitlines(keepends=True)
        if not LKM_CHECKPOINT_MARKER_LINE_RE.fullmatch(line)
    )


def _strip_lkm_runtime_instrumentation_lines(text: str) -> str:
    kept: list[str] = []
    block_depth = 0
    ignore_next_blank = False

    for line in text.splitlines(keepends=True):
        stripped = line.strip()
        if ignore_next_blank:
            if not stripped:
                ignore_next_blank = False
                continue
            ignore_next_blank = False

        if block_depth:
            if re.match(r"#\s*if(?:n?def)?\b", stripped):
                block_depth += 1
            elif re.match(r"#\s*endif\b", stripped):
                block_depth -= 1
                if block_depth == 0:
                    ignore_next_blank = True
            continue

        if INSTRUMENTATION_INCLUDE in line:
            continue
        if re.match(r"#\s*if(?:n?def)?\b", stripped) and INSTRUMENTATION_CONFIG in stripped:
            block_depth = 1
            continue
        if INSTRUMENTATION_CALL in line or INSTRUMENTATION_RECORD_CALL in line:
            continue

        kept.append(line)

    return "".join(kept)


def _find_matching_delimiter(text: str, open_index: int, open_char: str, close_char: str) -> int:
    if open_index < 0 or open_index >= len(text) or text[open_index] != open_char:
        raise LinuxCheckpointMappingError(f"expected {open_char!r}")

    depth = 0
    i = open_index
    state = "code"
    while i < len(text):
        char = text[i]
        next_char = text[i + 1] if i + 1 < len(text) else ""

        if state == "code":
            if char == "/" and next_char == "/":
                state = "line_comment"
                i += 2
                continue
            if char == "/" and next_char == "*":
                state = "block_comment"
                i += 2
                continue
            if char == '"':
                state = "string"
                i += 1
                continue
            if char == "'":
                state = "char"
                i += 1
                continue
            if char == open_char:
                depth += 1
            elif char == close_char:
                depth -= 1
                if depth == 0:
                    return i
        elif state == "line_comment":
            if char == "\n":
                state = "code"
        elif state == "block_comment":
            if char == "*" and next_char == "/":
                state = "code"
                i += 2
                continue
        elif state == "string":
            if char == "\\":
                i += 2
                continue
            if char == '"':
                state = "code"
        elif state == "char":
            if char == "\\":
                i += 2
                continue
            if char == "'":
                state = "code"

        i += 1

    raise LinuxCheckpointMappingError(f"unclosed {open_char!r}")


def _offset_is_code(text: str, offset: int) -> bool:
    i = 0
    state = "code"
    while i < offset:
        char = text[i]
        next_char = text[i + 1] if i + 1 < len(text) else ""

        if state == "code":
            if char == "/" and next_char == "/":
                state = "line_comment"
                i += 2
                continue
            if char == "/" and next_char == "*":
                state = "block_comment"
                i += 2
                continue
            if char == '"':
                state = "string"
                i += 1
                continue
            if char == "'":
                state = "char"
                i += 1
                continue
        elif state == "line_comment":
            if char == "\n":
                state = "code"
        elif state == "block_comment":
            if char == "*" and next_char == "/":
                state = "code"
                i += 2
                continue
        elif state == "string":
            if char == "\\":
                i += 2
                continue
            if char == '"':
                state = "code"
        elif state == "char":
            if char == "\\":
                i += 2
                continue
            if char == "'":
                state = "code"

        i += 1

    return state == "code"


class LinuxSourceIndex:
    def __init__(self, root: Path):
        self.root = root
        self._texts: dict[str, str] = {}

    def read_text(self, relative_file: str) -> str | None:
        if relative_file in self._texts:
            return self._texts[relative_file]
        path = self.root / relative_file
        if not path.is_file():
            return None
        text = path.read_text(encoding="utf-8", errors="replace")
        text = _strip_lkm_checkpoint_marker_lines(text)
        text = _strip_lkm_runtime_instrumentation_lines(text)
        self._texts[relative_file] = text
        return text

    def find_symbol(self, relative_file: str, symbol: str) -> LinuxSymbol | None:
        text = self.read_text(relative_file)
        if text is None:
            return None

        if relative_file.endswith((".S", ".s")):
            return self._find_assembly_symbol(relative_file, symbol, text)

        syscall_macro = self._find_syscall_macro(relative_file, symbol, text)
        if syscall_macro is not None:
            return syscall_macro

        return self._find_c_function(relative_file, symbol, text)

    def _find_syscall_macro(
        self,
        relative_file: str,
        symbol: str,
        text: str,
    ) -> LinuxSymbol | None:
        symbol_match = re.fullmatch(
            r"(SYSCALL_DEFINE[0-9]+)\(([A-Za-z_][A-Za-z0-9_]*)\)",
            symbol,
        )
        if symbol_match is None:
            return None

        macro_name, syscall_name = symbol_match.groups()
        macro_pattern = re.compile(
            rf"^[ \t]*{re.escape(macro_name)}\s*\(\s*"
            rf"{re.escape(syscall_name)}\b",
            re.MULTILINE,
        )
        for match in macro_pattern.finditer(text):
            if not _offset_is_code(text, match.start()):
                continue
            open_paren = text.find("(", match.start(), match.end() + 1)
            if open_paren == -1:
                continue
            try:
                close_paren = _find_matching_delimiter(text, open_paren, "(", ")")
            except LinuxCheckpointMappingError:
                continue

            search_end = min(len(text), close_paren + 4096)
            open_brace = text.find("{", close_paren, search_end)
            if open_brace == -1:
                continue

            between_signature_and_body = text[close_paren + 1 : open_brace]
            if (
                re.search(r"\b(?:SYSCALL_DEFINE|COMPAT_SYSCALL_DEFINE)[0-9]+\s*\(", between_signature_and_body)
                or re.search(r"^[ \t]*#\s*(?:elif|else|endif)\b", between_signature_and_body, re.MULTILINE)
            ):
                continue

            try:
                close_brace = _find_matching_delimiter(text, open_brace, "{", "}")
            except LinuxCheckpointMappingError:
                continue

            return LinuxSymbol(
                relative_file=relative_file,
                name=symbol,
                kind="syscall_macro",
                start_line=_line_number(text, match.start()),
                end_line=_line_number(text, close_brace),
                body=text[open_brace + 1 : close_brace],
                body_start_offset=open_brace + 1,
                text=text,
            )

        return None

    def _find_c_function(
        self,
        relative_file: str,
        symbol: str,
        text: str,
    ) -> LinuxSymbol | None:
        for match in re.finditer(rf"\b{re.escape(symbol)}\s*\(", text):
            if not _offset_is_code(text, match.start()):
                continue
            open_paren = text.find("(", match.start(), match.end() + 1)
            if open_paren == -1:
                continue
            try:
                close_paren = _find_matching_delimiter(text, open_paren, "(", ")")
            except LinuxCheckpointMappingError:
                continue

            search_end = min(len(text), close_paren + 512)
            semicolon = text.find(";", close_paren, search_end)
            open_brace = text.find("{", close_paren, search_end)
            if open_brace == -1:
                continue
            if semicolon != -1 and semicolon < open_brace:
                continue

            try:
                close_brace = _find_matching_delimiter(text, open_brace, "{", "}")
            except LinuxCheckpointMappingError:
                continue

            return LinuxSymbol(
                relative_file=relative_file,
                name=symbol,
                kind="c_function",
                start_line=_line_number(text, match.start()),
                end_line=_line_number(text, close_brace),
                body=text[open_brace + 1 : close_brace],
                body_start_offset=open_brace + 1,
                text=text,
            )

        return None

    def _find_assembly_symbol(
        self,
        relative_file: str,
        symbol: str,
        text: str,
    ) -> LinuxSymbol | None:
        sym_macro = re.compile(
            rf"^[ \t]*SYM_[A-Z0-9_]*START[A-Z0-9_]*\(\s*{re.escape(symbol)}\s*\)",
            re.MULTILINE,
        )
        match = sym_macro.search(text)
        if match is not None:
            end_match = re.search(
                rf"^[ \t]*SYM_[A-Z0-9_]*END[A-Z0-9_]*\(\s*{re.escape(symbol)}\s*\)",
                text[match.end() :],
                re.MULTILINE,
            )
            if end_match is not None:
                body_end = match.end() + end_match.start()
                end_offset = match.end() + end_match.end()
            else:
                body_end = _next_assembly_symbol_boundary(text, match.end())
                end_offset = body_end
            return LinuxSymbol(
                relative_file=relative_file,
                name=symbol,
                kind="assembly_symbol",
                start_line=_line_number(text, match.start()),
                end_line=_line_number(text, end_offset),
                body=text[match.end() : body_end],
                body_start_offset=match.end(),
                text=text,
            )

        label_pattern = re.compile(
            rf"^[ \t]*{re.escape(symbol)}:\s*(?:$|[#/@])",
            re.MULTILINE,
        )
        match = label_pattern.search(text)
        if match is None:
            return None

        body_end = _next_assembly_symbol_boundary(text, match.end())
        return LinuxSymbol(
            relative_file=relative_file,
            name=symbol,
            kind="assembly_label",
            start_line=_line_number(text, match.start()),
            end_line=_line_number(text, body_end),
            body=text[match.end() : body_end],
            body_start_offset=match.end(),
            text=text,
        )


def _next_assembly_symbol_boundary(text: str, offset: int) -> int:
    boundary_pattern = re.compile(
        r"^[ \t]*(?:SYM_[A-Z0-9_]*START[A-Z0-9_]*\(|(?:\.global[^\n]*\n[ \t]*)?[A-Za-z_][A-Za-z0-9_$]*:\s*(?:$|[#/@]))",
        re.MULTILINE,
    )
    match = boundary_pattern.search(text, offset)
    if match is None:
        return len(text)
    return match.start()


def _find_anchor(symbol: LinuxSymbol, pattern: str) -> AnchorMatch | None:
    match = re.search(pattern, symbol.body, re.MULTILINE)
    if match is None:
        return None
    absolute = symbol.body_start_offset + match.start()
    return AnchorMatch(
        line=_line_number(symbol.text, absolute),
        offset=absolute,
        text=_line_text(symbol.text, absolute),
    )


def _symbol_display(symbol: LinuxSymbol) -> str:
    if symbol.kind == "c_function":
        return f"{symbol.name}()"
    return symbol.name


def _function_anchor(symbol: LinuxSymbol) -> str:
    return f"{_symbol_display(symbol)} definition line {symbol.start_line}"


def _resolved_anchor(symbol: LinuxSymbol, anchor: AnchorMatch) -> str:
    return f"{_symbol_display(symbol)} line {anchor.line}: {anchor.text}"


def _unmapped(record: CheckpointInventoryRecord, notes: str) -> LinuxCheckpointMappingRecord:
    return LinuxCheckpointMappingRecord(
        checkpoint_index=record.index,
        checkpoint_name=record.name,
        checkpoint_variant=record.variant,
        linux_file=None,
        linux_symbol=None,
        linux_anchor=None,
        mapping_kind="unmapped",
        confidence="none",
        notes=notes,
    )


def _resolve_rule(
    record: CheckpointInventoryRecord,
    rule: MappingRule,
    linux_sources: LinuxSourceIndex,
) -> LinuxCheckpointMappingRecord:
    symbol = linux_sources.find_symbol(rule.linux_file, rule.linux_symbol)
    if symbol is None:
        return _unmapped(
            record,
            f"Linux symbol {rule.linux_file}::{rule.linux_symbol} was not found.",
        )

    if rule.mapping_kind == "exact":
        if rule.anchor_pattern is None:
            linux_anchor = _function_anchor(symbol)
        else:
            anchor = _find_anchor(symbol, rule.anchor_pattern)
            if anchor is None:
                return _unmapped(
                    record,
                    (
                        f"Linux anchor for {rule.linux_file}::{rule.linux_symbol} "
                        "was not found."
                    ),
                )
            linux_anchor = _resolved_anchor(symbol, anchor)
    elif rule.mapping_kind == "range":
        if rule.start_anchor_pattern is None or rule.end_anchor_pattern is None:
            raise LinuxCheckpointMappingError("range rules require start and end anchors")
        start_anchor = _find_anchor(symbol, rule.start_anchor_pattern)
        end_anchor = _find_anchor(symbol, rule.end_anchor_pattern)
        if start_anchor is None or end_anchor is None:
            return _unmapped(
                record,
                (
                    f"Linux range anchors for {rule.linux_file}::{rule.linux_symbol} "
                    "were not both found."
                ),
            )
        if start_anchor.offset > end_anchor.offset:
            return _unmapped(
                record,
                (
                    f"Linux range anchors for {rule.linux_file}::{rule.linux_symbol} "
                    "were found out of order."
                ),
            )
        linux_anchor = (
            f"{_symbol_display(symbol)} lines {start_anchor.line}-{end_anchor.line}: "
            f"{start_anchor.text} .. {end_anchor.text}"
        )
    else:
        raise LinuxCheckpointMappingError(f"unsupported mapping kind: {rule.mapping_kind}")

    return LinuxCheckpointMappingRecord(
        checkpoint_index=record.index,
        checkpoint_name=record.name,
        checkpoint_variant=record.variant,
        linux_file=rule.linux_file,
        linux_symbol=rule.linux_symbol,
        linux_anchor=linux_anchor,
        mapping_kind=rule.mapping_kind,
        confidence=rule.confidence,
        notes=rule.notes,
    )


def default_mapping_rules() -> dict[str, MappingRule]:
    return {
        "EntryPreludePhase.Started": MappingRule(
            mapping_kind="exact",
            linux_file="arch/riscv/kernel/head.S",
            linux_symbol="_start",
            confidence="high",
            notes="RISC-V64 Linux boot image entry symbol; architecture-scoped head.S mapping.",
        ),
        "EntryPreludePhase.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="arch/riscv/kernel/head.S",
            linux_symbol="_start_kernel",
            anchor_pattern=r"^[ \t]*tail[ \t]+start_kernel\b",
            confidence="high",
            notes="RISC-V64 head.S handoff from _start_kernel to Linux start_kernel().",
        ),
        "KernelImage.Prepared": MappingRule(
            mapping_kind="range",
            linux_file="arch/riscv/kernel/head.S",
            linux_symbol="_start_kernel",
            start_anchor_pattern=r"^[ \t]*\.Lclear_bss:",
            end_anchor_pattern=r"^[ \t]*\.Lclear_bss_done:",
            confidence="medium",
            notes="RISC-V64 head.S BSS clear interval; not a portable Linux kernel-image object boundary.",
        ),
        "KernelImage.Ready": MappingRule(
            mapping_kind="range",
            linux_file="arch/riscv/mm/init.c",
            linux_symbol="setup_vm",
            start_anchor_pattern=r"\bkernel_map\.virt_addr\s*=",
            end_anchor_pattern=r"\bcreate_kernel_page_table\s*\(\s*early_pg_dir\s*,\s*true\s*\)",
            confidence="medium",
            notes="RISC-V64 setup_vm() kernel_map initialization through early kernel mapping construction.",
        ),
        "KernelImage.Online": MappingRule(
            mapping_kind="exact",
            linux_file="arch/riscv/kernel/head.S",
            linux_symbol="relocate_enable_mmu",
            anchor_pattern=r"^[ \t]*load_global_pointer\b",
            confidence="medium",
            notes="RISC-V64 relocation boundary after virtual addressing is active; object equivalence is partial.",
        ),
        "EventStream.Prepared": MappingRule(
            mapping_kind="exact",
            linux_file="arch/riscv/kernel/head.S",
            linux_symbol="_start_kernel",
            anchor_pattern=r"^[ \t]*csrw[ \t]+CSR_TVEC,\s*a3\b",
            confidence="medium",
            notes="RISC-V64 early fallback trap-vector setup before setup_vm(); architecture-scoped mapping.",
        ),
        "EventStream.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="arch/riscv/kernel/head.S",
            linux_symbol="_start",
            anchor_pattern=r"^[ \t]*la[ \t]+a0,\s*handle_exception\b",
            confidence="high",
            notes="RISC-V64 formal trap-vector target in .Lsetup_trap_vector.",
        ),
        "ExceptionStream.Prepared": MappingRule(
            mapping_kind="exact",
            linux_file="arch/riscv/kernel/head.S",
            linux_symbol="_start_kernel",
            anchor_pattern=r"^[ \t]*csrw[ \t]+CSR_TVEC,\s*a3\b",
            confidence="medium",
            notes="RISC-V64 early fallback exception path uses the temporary spin trap vector.",
        ),
        "ExceptionStream.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="arch/riscv/kernel/head.S",
            linux_symbol="_start",
            anchor_pattern=r"^[ \t]*la[ \t]+a0,\s*handle_exception\b",
            confidence="high",
            notes="RISC-V64 formal exception entry target installed by .Lsetup_trap_vector.",
        ),
        "TrampolineVm.Ready": MappingRule(
            mapping_kind="range",
            linux_file="arch/riscv/mm/init.c",
            linux_symbol="setup_vm",
            start_anchor_pattern=r"Setup trampoline PGD",
            end_anchor_pattern=r"\bcreate_pmd_mapping\s*\(\s*trampoline_pmd\b",
            confidence="medium",
            notes="RISC-V64 setup_vm() trampoline page-table construction interval.",
        ),
        "TrampolineVm.Online": MappingRule(
            mapping_kind="exact",
            linux_file="arch/riscv/kernel/head.S",
            linux_symbol="relocate_enable_mmu",
            anchor_pattern=r"^[ \t]*csrw[ \t]+CSR_SATP,\s*a0\b",
            confidence="high",
            notes="RISC-V64 relocate_enable_mmu loads the trampoline page directory into satp.",
        ),
        "RawDtb.Prepared": MappingRule(
            mapping_kind="exact",
            linux_file="arch/riscv/mm/init.c",
            linux_symbol="setup_vm",
            anchor_pattern=r"\bcreate_fdt_early_page_table\s*\(\s*__fix_to_virt\s*\(\s*FIX_FDT\s*\)\s*,\s*dtb_pa\s*\)",
            confidence="medium",
            notes="RISC-V64 setup_vm() consumes the boot DTB physical address for early FDT mapping.",
        ),
        "RawDtb.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="arch/riscv/mm/init.c",
            linux_symbol="create_fdt_early_page_table",
            anchor_pattern=r"\bdtb_early_pa\s*=\s*dtb_pa\s*;",
            confidence="high",
            notes="RISC-V64 early FDT helper records dtb_early_pa after creating the fixmap-backed DTB view.",
        ),
        "FixMap.Ready": MappingRule(
            mapping_kind="range",
            linux_file="arch/riscv/mm/init.c",
            linux_symbol="setup_vm",
            start_anchor_pattern=r"Setup early PGD for fixmap",
            end_anchor_pattern=r"\bcreate_pmd_mapping\s*\(\s*fixmap_pmd\s*,\s*FIXADDR_START\b",
            confidence="medium",
            notes="RISC-V64 setup_vm() early fixmap page-table construction interval.",
        ),
        "EarlyVm.Prepared": MappingRule(
            mapping_kind="range",
            linux_file="arch/riscv/mm/init.c",
            linux_symbol="setup_vm",
            start_anchor_pattern=r"\bpt_ops_set_early\s*\(",
            end_anchor_pattern=r"\bcreate_kernel_page_table\s*\(\s*early_pg_dir\s*,\s*true\s*\)",
            confidence="medium",
            notes="RISC-V64 setup_vm() early page-table preparation interval.",
        ),
        "EarlyVm.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="arch/riscv/mm/init.c",
            linux_symbol="setup_vm",
            anchor_pattern=r"\bcreate_kernel_page_table\s*\(\s*early_pg_dir\s*,\s*true\s*\)",
            confidence="high",
            notes="RISC-V64 setup_vm() constructs the early kernel page table.",
        ),
        "EarlyVm.Online": MappingRule(
            mapping_kind="exact",
            linux_file="arch/riscv/kernel/head.S",
            linux_symbol="relocate_enable_mmu",
            anchor_pattern=r"^[ \t]*csrw[ \t]+CSR_SATP,\s*a2\b",
            confidence="high",
            notes="RISC-V64 relocate_enable_mmu switches from trampoline mappings to the early kernel page table.",
        ),
        "Kernel.Started": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="start_kernel",
            confidence="high",
            notes="Linux C kernel entry anchor.",
        ),
        "MmCoreInitPhase.Started": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="start_kernel",
            anchor_pattern=r"\bmm_core_init\s*\(",
            confidence="high",
            notes="Linux start_kernel() call site for mm_core_init().",
        ),
        "MmCoreInitPhase.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="mm/mm_init.c",
            linux_symbol="mm_core_init",
            confidence="high",
            notes="Linux mm_core_init() function boundary.",
        ),
        "SchedInitPhase.Started": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="start_kernel",
            anchor_pattern=r"\bsched_init\s*\(",
            confidence="high",
            notes="Linux start_kernel() call site for sched_init().",
        ),
        "SchedInitPhase.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="kernel/sched/core.c",
            linux_symbol="sched_init",
            confidence="high",
            notes="Linux sched_init() function boundary.",
        ),
        "BootInitRestInitPhase.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="rest_init",
            confidence="high",
            notes="Linux rest_init() creates init/kthreadd and enters boot idle.",
        ),
        "RootfsPhase.Started": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init_freeable",
            anchor_pattern=r"\bkunit_run_all_tests\s*\(\s*\)\s*;",
            confidence="high",
            notes="Linux kernel_init_freeable() RootfsPhase entry at kunit_run_all_tests(); the later RamdiskExecuteCommand.EaccessCheckpoint preserves the prepare_namespace pre-boundary.",
        ),
        "RootfsPhase.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="init/do_mounts.c",
            linux_symbol="prepare_namespace",
            confidence="high",
            notes="Linux prepare_namespace() rootfs preparation boundary.",
        ),
        "KUnitRuntime.TrimmedReady": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init_freeable",
            anchor_pattern=r"\bkunit_run_all_tests\s*\(\s*\)\s*;",
            confidence="medium",
            notes="Linux kernel_init_freeable() reserves the KUnit run position; arceos_ex records the CONFIG_KUNIT=n trimmed/no-op fact at this call site.",
        ),
        "InitramfsSync.DeferredReady": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init_freeable",
            anchor_pattern=r"\bwait_for_initramfs\s*\(\s*\)\s*;",
            confidence="medium",
            notes="Linux kernel_init_freeable() waits for initramfs unpacking; arceos_ex maps only the deferred initramfs synchronization boundary.",
        ),
        "RootfsConsole.DeferredReady": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init_freeable",
            anchor_pattern=r"\bconsole_on_rootfs\s*\(\s*\)\s*;",
            confidence="medium",
            notes="Linux kernel_init_freeable() opens /dev/console on rootfs; arceos_ex keeps the rootfs console handoff as a deferred boundary.",
        ),
        "RootfsPrepareNamespacePaths.Ready": MappingRule(
            mapping_kind="range",
            linux_file="init/do_mounts.c",
            linux_symbol="prepare_namespace",
            start_anchor_pattern=r"\bwait_for_device_probe\s*\(\s*\)\s*;",
            end_anchor_pattern=r"\binit_chroot\s*\(\s*\"\.\s*\"\s*\)\s*;",
            confidence="medium",
            notes="Linux prepare_namespace() path-classification interval from device-probe wait through final chroot; arceos_ex records the rootdelay/rootwait/initrd/md/NFS/CIFS/devtmpfs classifications without expanding each path.",
        ),
        "RamdiskExecuteCommand.EaccessCheckpoint": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init_freeable",
            anchor_pattern=r"\binit_eaccess\s*\(\s*ramdisk_execute_command\s*\)",
            confidence="medium",
            notes="Linux kernel_init_freeable() checks ramdisk_execute_command accessibility before deciding whether to enter prepare_namespace().",
        ),
        "RootFS.Online": MappingRule(
            mapping_kind="exact",
            linux_file="init/do_mounts.c",
            linux_symbol="prepare_namespace",
            anchor_pattern=r"\binit_chroot\s*\(\s*\"\.\s*\"\s*\)\s*;",
            confidence="medium",
            notes="Linux prepare_namespace() completes the visible root switch at init_chroot(\".\"); arceos_ex records the RootFS object online fact at that boundary.",
        ),
        "IntegrityKeys.DeferredReady": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init_freeable",
            anchor_pattern=r"\bintegrity_load_keys\s*\(\s*\)\s*;",
            confidence="medium",
            notes="Linux kernel_init_freeable() calls integrity_load_keys() after rootfs is available; arceos_ex keeps integrity/IMA/EVM key loading as a deferred/trimmed boundary.",
        ),
        "RootfsBoundary.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init_freeable",
            anchor_pattern=r"\bintegrity_load_keys\s*\(\s*\)\s*;",
            confidence="medium",
            notes="Linux kernel_init_freeable() reaches the Rootfs end boundary after integrity_load_keys(), immediately before returning to kernel_init(); this is not an independent Linux object.",
        ),
        "PayloadPhase.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init",
            anchor_pattern=r"\bdo_sysctl_args\s*\(",
            confidence="medium",
            notes="Linux kernel_init() reaches the post-finalize payload-selection boundary.",
        ),
        "PayloadPhase.Online": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init",
            anchor_pattern=r'try_to_run_init_process\s*\(\s*"/sbin/init"',
            confidence="medium",
            notes=(
                "Linux kernel_init() default init candidate handoff anchor. "
                "Requested-init paths record the same runtime checkpoint before "
                "run_init_process(execute_command), but the single-fingerprint "
                "LKM_CHECKPOINT marker remains on this canonical fallback anchor."
            ),
        ),
        "CorePreparePhase.Started": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="start_kernel",
            anchor_pattern=r"\bsetup_arch\s*\(",
            confidence="medium",
            notes="Linux start_kernel() architecture setup call; RISC-V paging_init is inside setup_arch().",
        ),
        "CorePreparePhase.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="start_kernel",
            anchor_pattern=r"\btrap_init\s*\(",
            confidence="medium",
            notes="Linux start_kernel() trap_init() call near the CorePreparePhase ready boundary.",
        ),
        "IrqTimeInitPhase.Started": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="start_kernel",
            anchor_pattern=r"\bearly_irq_init\s*\(",
            confidence="high",
            notes="Linux start_kernel() begins the IRQ/time init call interval at early_irq_init().",
        ),
        "IrqTimeInitPhase.Ready": MappingRule(
            mapping_kind="range",
            linux_file="init/main.c",
            linux_symbol="start_kernel",
            start_anchor_pattern=r"\bearly_irq_init\s*\(",
            end_anchor_pattern=r"\btime_init\s*\(",
            confidence="medium",
            notes="Linux start_kernel() IRQ/tick/timer/time initialization interval.",
        ),
        "LocalIrqEnablePhase.Started": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="start_kernel",
            anchor_pattern=r"\bearly_boot_irqs_disabled\s*=\s*false\s*;",
            confidence="high",
            notes="Linux start_kernel() clears the early IRQ-disabled guard before enabling local IRQs.",
        ),
        "LocalIrqEnablePhase.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="start_kernel",
            anchor_pattern=r"\blocal_irq_enable\s*\(",
            confidence="high",
            notes="Linux start_kernel() local_irq_enable() boundary.",
        ),
        "ProcessPreparePhase.Started": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="start_kernel",
            anchor_pattern=r"\bpid_idr_init\s*\(",
            confidence="medium",
            notes="Linux start_kernel() process/task namespace preparation interval starts at pid_idr_init().",
        ),
        "ProcessPreparePhase.Ready": MappingRule(
            mapping_kind="range",
            linux_file="init/main.c",
            linux_symbol="start_kernel",
            start_anchor_pattern=r"\bpid_idr_init\s*\(",
            end_anchor_pattern=r"\bdelayacct_init\s*\(",
            confidence="medium",
            notes="Linux start_kernel() process/task/credential/cache preparation interval before rest_init().",
        ),
        "PreSmpInitPhase.Started": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init_freeable",
            anchor_pattern=r"\bsmp_prepare_cpus\s*\(",
            confidence="high",
            notes="Linux kernel_init_freeable() starts pre-SMP preparation at smp_prepare_cpus().",
        ),
        "PreSmpInitPhase.Ready": MappingRule(
            mapping_kind="range",
            linux_file="init/main.c",
            linux_symbol="kernel_init_freeable",
            start_anchor_pattern=r"\bsmp_prepare_cpus\s*\(",
            end_anchor_pattern=r"\blockup_detector_init\s*\(",
            confidence="medium",
            notes="Linux kernel_init_freeable() pre-SMP preparation interval before smp_init().",
        ),
        "SmpBringupPhase.Started": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init_freeable",
            anchor_pattern=r"\bsmp_init\s*\(",
            confidence="high",
            notes="Linux kernel_init_freeable() SMP bringup call.",
        ),
        "CpuStartProvider.BootDataSelected": MappingRule(
            mapping_kind="exact",
            linux_file="arch/riscv/kernel/cpu_ops_sbi.c",
            linux_symbol="sbi_cpu_start",
            anchor_pattern=r"\bbdata->task_ptr\s*=\s*tidle\s*;",
            confidence="high",
            notes="Linux RISC-V HSM cpu_start selects per-CPU SBI boot data and binds the target idle task pointer.",
        ),
        "CpuStartProvider.BootDataPublished": MappingRule(
            mapping_kind="exact",
            linux_file="arch/riscv/kernel/cpu_ops_sbi.c",
            linux_symbol="sbi_cpu_start",
            anchor_pattern=r"Make sure boot data is updated",
            confidence="high",
            notes="Linux RISC-V HSM cpu_start publishes task_ptr/stack_ptr with the boot-data smp_mb() before hart_start.",
        ),
        "CpuStartProvider.HsmStartIssued": MappingRule(
            mapping_kind="exact",
            linux_file="arch/riscv/kernel/cpu_ops_sbi.c",
            linux_symbol="sbi_hsm_hart_start",
            anchor_pattern=r"\bsbi_ecall\s*\(\s*SBI_EXT_HSM\s*,\s*SBI_EXT_HSM_HART_START",
            confidence="high",
            notes="Linux RISC-V ordered booting issues SBI HSM HART_START for the target hart.",
        ),
        "CpuStartProvider.HsmStartReturned": MappingRule(
            mapping_kind="range",
            linux_file="arch/riscv/kernel/cpu_ops_sbi.c",
            linux_symbol="sbi_hsm_hart_start",
            start_anchor_pattern=r"\bsbi_ecall\s*\(\s*SBI_EXT_HSM\s*,\s*SBI_EXT_HSM_HART_START",
            end_anchor_pattern=r"\breturn\s+0\s*;",
            confidence="high",
            notes="Linux RISC-V HSM start return handling after the SBI HART_START ecall.",
        ),
        "CpuStartProvider.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="arch/riscv/kernel/smpboot.c",
            linux_symbol="start_secondary_cpu",
            anchor_pattern=r"\breturn\s+cpu_ops->cpu_start\s*\(\s*cpu\s*,\s*tidle\s*\)",
            confidence="medium",
            notes="Linux RISC-V BP-side cpu_start provider call boundary through start_secondary_cpu().",
        ),
        "ApEntryPreludePhase.Started": MappingRule(
            mapping_kind="exact",
            linux_file="arch/riscv/kernel/head.S",
            linux_symbol="secondary_start_sbi",
            confidence="high",
            notes="RISC-V Linux AP entry symbol for SBI HSM ordered booting; distinct from the BP _start path.",
        ),
        "ApEntryPreludePhase.BootDataConsumed": MappingRule(
            mapping_kind="range",
            linux_file="arch/riscv/kernel/head.S",
            linux_symbol="secondary_start_sbi",
            start_anchor_pattern=r"a0 contains the hartid & a1 contains boot data",
            end_anchor_pattern=r"REG_L\s+sp,\s*\(a3\)",
            confidence="high",
            notes="Linux secondary_start_sbi consumes SBI boot data and loads the AP current task and stack.",
        ),
        "ApEntryPreludePhase.CurrentStackEstablished": MappingRule(
            mapping_kind="exact",
            linux_file="arch/riscv/kernel/head.S",
            linux_symbol="secondary_start_sbi",
            anchor_pattern=r"REG_L\s+sp,\s*\(a3\)",
            confidence="high",
            notes="Linux secondary_start_sbi loads the AP stack pointer from SBI boot data.",
        ),
        "ApEntryPreludePhase.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="arch/riscv/kernel/head.S",
            linux_symbol="secondary_start_sbi",
            anchor_pattern=r"\bcall\s+smp_callin\b",
            confidence="high",
            notes="Linux secondary_start_sbi reaches the AP C bringup handoff after setting the AP task, stack, MMU, trap vector and SCS.",
        ),
        "ApSmpCallinPhase.Started": MappingRule(
            mapping_kind="exact",
            linux_file="arch/riscv/kernel/smpboot.c",
            linux_symbol="smp_callin",
            confidence="high",
            notes="Linux RISC-V C entry point for a secondary processor.",
        ),
        "ApSmpCallinPhase.CpuRunningProduced": MappingRule(
            mapping_kind="exact",
            linux_file="arch/riscv/kernel/smpboot.c",
            linux_symbol="smp_callin",
            anchor_pattern=r"\bcomplete\s*\(\s*&cpu_running\s*\)\s*;",
            confidence="high",
            notes="Linux RISC-V AP completes cpu_running after online, IPI, cache and TLB setup.",
        ),
        "ApSmpCallinPhase.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="arch/riscv/kernel/smpboot.c",
            linux_symbol="smp_callin",
            anchor_pattern=r"\bcomplete\s*\(\s*&cpu_running\s*\)\s*;",
            confidence="high",
            notes="Linux RISC-V smp_callin reaches the cpu_running completion boundary before enabling local IRQs.",
        ),
        "ApOnlineIdlePhase.Started": MappingRule(
            mapping_kind="exact",
            linux_file="arch/riscv/kernel/smpboot.c",
            linux_symbol="smp_callin",
            anchor_pattern=r"\blocal_irq_enable\s*\(\s*\)\s*;",
            confidence="high",
            notes="Linux RISC-V AP online-idle path begins by enabling local interrupts after cpu_running.",
        ),
        "ApOnlineIdlePhase.DoneUpProduced": MappingRule(
            mapping_kind="exact",
            linux_file="kernel/cpu.c",
            linux_symbol="cpuhp_online_idle",
            anchor_pattern=r"\bcomplete_ap_thread\s*\(\s*st\s*,\s*true\s*\)\s*;",
            confidence="high",
            notes="Linux generic CPU hotplug online-idle path completes done_up for the BP-side waiter.",
        ),
        "ApOnlineIdlePhase.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="kernel/sched/idle.c",
            linux_symbol="cpu_startup_entry",
            anchor_pattern=r"\bdo_idle\s*\(\s*\)\s*;",
            confidence="medium",
            notes="Linux AP enters the idle loop after CPUHP_AP_ONLINE_IDLE handoff.",
        ),
        "SecondaryCpuStartupAck.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="arch/riscv/kernel/smpboot.c",
            linux_symbol="__cpu_up",
            anchor_pattern=r"\bwait_for_completion_timeout\s*\(\s*&cpu_running",
            confidence="high",
            notes="Linux RISC-V BP-side __cpu_up waits for the AP cpu_running completion.",
        ),
        "SecondaryCpuOnlineAck.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="kernel/cpu.c",
            linux_symbol="bringup_wait_for_ap_online",
            anchor_pattern=r"\bwait_for_ap_thread\s*\(\s*st\s*,\s*true\s*\)\s*;",
            confidence="high",
            notes="Linux generic CPU hotplug waits until the AP reaches CPUHP_AP_ONLINE_IDLE and done_up has completed.",
        ),
        "SmpBringupPhase.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init_freeable",
            anchor_pattern=r"\bsched_init_smp\s*\(",
            confidence="medium",
            notes="Linux kernel_init_freeable() scheduler SMP completion call after smp_init().",
        ),
        "RuntimeCorePhase.Started": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init_freeable",
            anchor_pattern=r"\bsched_init_smp\s*\(",
            confidence="medium",
            notes="Linux kernel_init_freeable() RuntimeCorePhase Preset starts at the sched_init_smp() action entry.",
        ),
        "RuntimeCorePhase.Ready": MappingRule(
            mapping_kind="range",
            linux_file="init/main.c",
            linux_symbol="kernel_init_freeable",
            start_anchor_pattern=r"\bworkqueue_init_topology\s*\(",
            end_anchor_pattern=r"\bpage_alloc_init_late\s*\(",
            confidence="medium",
            notes="Linux kernel_init_freeable() runtime core topology/async/padata/page-alloc-late interval.",
        ),
        "Scheduler.SmpReady": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init_freeable",
            anchor_pattern=r"\bsched_init_smp\s*\(",
            confidence="medium",
            notes="Linux kernel_init_freeable() sched_init_smp() call; reuses the SmpBringupPhase.Ready anchor but records the scheduler SMP runtime object fact.",
        ),
        "Workqueue.TopologyReady": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init_freeable",
            anchor_pattern=r"\bworkqueue_init_topology\s*\(",
            confidence="medium",
            notes="Linux kernel_init_freeable() workqueue_init_topology() call; records the Workqueue topology object fact after SMP scheduler setup.",
        ),
        "AsyncCore.DeferredReady": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init_freeable",
            anchor_pattern=r"\basync_init\s*\(",
            confidence="medium",
            notes="Linux kernel_init_freeable() async_init() call; arceos_ex keeps async internals deferred and maps only the deferred runtime-core boundary.",
        ),
        "PadataCore.DeferredReady": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init_freeable",
            anchor_pattern=r"\bpadata_init\s*\(",
            confidence="medium",
            notes="Linux kernel_init_freeable() padata_init() call; maps the deferred padata boundary without expanding padata object details.",
        ),
        "PageAllocator.LateReady": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init_freeable",
            anchor_pattern=r"\bpage_alloc_init_late\s*\(",
            confidence="medium",
            notes="Linux kernel_init_freeable() page_alloc_init_late() call; records the PageAllocator late object fact without changing the earlier allocator lifecycle view.",
        ),
        "RuntimeCoreBoundary.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init_freeable",
            anchor_pattern=r"\bpage_alloc_init_late\s*\(",
            confidence="medium",
            notes="Linux kernel_init_freeable() page_alloc_init_late() call; marks the Runtime Core window end boundary before do_basic_setup(), not an independent Linux object.",
        ),
        "InitcallPhase.Started": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init_freeable",
            anchor_pattern=r"\bdo_basic_setup\s*\(",
            confidence="high",
            notes="Linux kernel_init_freeable() enters do_basic_setup().",
        ),
        "InitcallPhase.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="do_basic_setup",
            anchor_pattern=r"\bdo_initcalls\s*\(",
            confidence="high",
            notes="Linux do_basic_setup() initcall execution anchor.",
        ),
        "CpusetSmp.TrimmedReady": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="do_basic_setup",
            anchor_pattern=r"\bcpuset_init_smp\s*\(",
            confidence="medium",
            notes="Linux do_basic_setup() cpuset_init_smp() call; arceos_ex records the cpuset/cgroup trimmed/no-op object fact at this reserved position.",
        ),
        "DriverCore.DeferredReady": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="do_basic_setup",
            anchor_pattern=r"\bdriver_init\s*\(",
            confidence="medium",
            notes="Linux do_basic_setup() driver_init() call; maps only the driver core base/deferred boundary without expanding driver model subobjects.",
        ),
        "IrqProcView.DeferredReady": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="do_basic_setup",
            anchor_pattern=r"\binit_irq_proc\s*\(",
            confidence="medium",
            notes="Linux do_basic_setup() init_irq_proc() call; maps the procfs IRQ view deferred boundary without expanding per-IRQ proc export details.",
        ),
        "CtorTable.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="do_basic_setup",
            anchor_pattern=r"\bdo_ctors\s*\(",
            confidence="medium",
            notes="Linux do_basic_setup() do_ctors() call; maps the constructor table dispatch boundary and current trimmed/empty constructor-table fact.",
        ),
        "InitcallTable.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="do_basic_setup",
            anchor_pattern=r"\bdo_initcalls\s*\(",
            confidence="medium",
            notes="Linux do_basic_setup() do_initcalls() call; maps the initcall levels dispatcher/table object fact, not any individual initcall entry side effect.",
        ),
        "InitcallBoundary.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="do_basic_setup",
            anchor_pattern=r"\bdo_initcalls\s*\(",
            confidence="medium",
            notes="Linux do_basic_setup() do_initcalls() call; marks the do_basic_setup() end boundary before kernel_init_freeable() continues to kunit_run_all_tests(), not an independent Linux object.",
        ),
        "FinalizePhase.Started": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init",
            anchor_pattern=r"\basync_synchronize_full\s*\(",
            confidence="medium",
            notes="Linux kernel_init() begins final async/initmem cleanup after kernel_init_freeable().",
        ),
        "FinalizePhase.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init",
            anchor_pattern=r"\bsystem_state\s*=\s*SYSTEM_RUNNING\s*;",
            confidence="high",
            notes="Linux kernel_init() marks SYSTEM_RUNNING before payload selection.",
        ),
        "AsyncFullSync.DeferredReady": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init",
            anchor_pattern=r"\basync_synchronize_full\s*\(\s*\)\s*;",
            confidence="medium",
            notes="Linux kernel_init() waits for async __init work before initmem cleanup; arceos_ex models this as a deferred finalize boundary.",
        ),
        "SystemState.FreeingInitmemCheckpoint": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init",
            anchor_pattern=r"\bsystem_state\s*=\s*SYSTEM_FREEING_INITMEM\s*;",
            confidence="high",
            notes="Linux kernel_init() explicitly marks the system_state transition into initmem freeing.",
        ),
        "InitMemoryCleanup.DeferredReady": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init",
            anchor_pattern=r"\bfree_initmem\s*\(\s*\)\s*;",
            confidence="medium",
            notes="Linux kernel_init() initmem cleanup call site; arceos_ex keeps this as a deferred cleanup boundary rather than a separate Linux object.",
        ),
        "KernelMappingProtection.DeferredReady": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init",
            anchor_pattern=r"\bmark_readonly\s*\(\s*\)\s*;",
            confidence="medium",
            notes="Linux kernel_init() finalizes kernel mapping permissions at mark_readonly(); arceos_ex records the mapping-protection path as a deferred finalize boundary.",
        ),
        "PtiFinalize.TrimmedReady": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init",
            anchor_pattern=r"\bpti_finalize\s*\(\s*\)\s*;",
            confidence="medium",
            notes="Linux kernel_init() calls pti_finalize() after kernel mappings are finalized; arceos_ex records the PTI path as a trimmed/no-op fact for this configuration.",
        ),
        "SystemState.Online": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init",
            anchor_pattern=r"\bsystem_state\s*=\s*SYSTEM_RUNNING\s*;",
            confidence="high",
            notes="Linux kernel_init() marks SYSTEM_RUNNING after initmem cleanup and before payload/init selection.",
        ),
        "RcuCore.InkernelBootEnded": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init",
            anchor_pattern=r"\brcu_end_inkernel_boot\s*\(\s*\)\s*;",
            confidence="medium",
            notes="Linux kernel_init() calls rcu_end_inkernel_boot(); arceos_ex records the RcuCore object fact that in-kernel boot has ended without expanding full RCU GP runtime.",
        ),
        "RcuBootEnd.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init",
            anchor_pattern=r"\brcu_end_inkernel_boot\s*\(\s*\)\s*;",
            confidence="medium",
            notes="Linux kernel_init() rcu_end_inkernel_boot() call marks the RCU boot-end boundary after SYSTEM_RUNNING; this reuses the RcuCore anchor but records the phase boundary fact.",
        ),
        "SysctlArgs.DeferredReady": MappingRule(
            mapping_kind="exact",
            linux_file="init/main.c",
            linux_symbol="kernel_init",
            anchor_pattern=r"\bdo_sysctl_args\s*\(\s*\)\s*;",
            confidence="medium",
            notes="Linux kernel_init() processes boot-time sysctl arguments before init payload selection; arceos_ex keeps sysctl args handling as a deferred finalize boundary.",
        ),
        "SyscallTable.ExecveArgsReady": MappingRule(
            mapping_kind="exact",
            linux_file="fs/exec.c",
            linux_symbol="do_execveat_common",
            anchor_pattern=r"\bretval\s*=\s*copy_strings\s*\(\s*bprm->argc\s*,\s*argv\s*,\s*bprm\s*\)\s*;",
            confidence="high",
            notes="Linux do_execveat_common() has copied filename, envp and argv into linux_binprm before bprm_execve().",
        ),
        "UserExec.MainElfReady": MappingRule(
            mapping_kind="exact",
            linux_file="fs/binfmt_elf.c",
            linux_symbol="load_elf_binary",
            anchor_pattern=r"\belf_phdata\s*=\s*load_elf_phdrs\s*\(\s*elf_ex\s*,\s*bprm->file\s*\)\s*;",
            confidence="high",
            notes=(
                "Linux ELF loader has read the runtime exec main executable "
                "program headers; instrumentation must guard this shared "
                "load_elf_binary() anchor so kernel_execve() boot init records "
                "only UserBoot.MainElfReady."
            ),
        ),
        "UserExec.InterpreterReady": MappingRule(
            mapping_kind="exact",
            linux_file="fs/binfmt_elf.c",
            linux_symbol="load_elf_binary",
            anchor_pattern=r"\binterp_elf_phdata\s*=\s*load_elf_phdrs\s*\(\s*interp_elf_ex\s*,\s*interpreter\s*\)\s*;",
            confidence="high",
            notes=(
                "Linux PT_INTERP path has opened and parsed the runtime exec "
                "interpreter ELF program headers when an interpreter is present; "
                "instrumentation must guard this shared load_elf_binary() anchor "
                "so kernel_execve() boot init records only UserBoot.InterpreterReady."
            ),
        ),
        "UserExec.AddressSpaceReady": MappingRule(
            mapping_kind="range",
            linux_file="fs/binfmt_elf.c",
            linux_symbol="load_elf_binary",
            start_anchor_pattern=r"\bretval\s*=\s*setup_arg_pages\s*\(",
            end_anchor_pattern=r"\bmm->start_stack\s*=\s*bprm->p\s*;",
            confidence="medium",
            notes="Linux load_elf_binary() stack setup, PT_LOAD mapping, interpreter load, ELF tables and mm layout interval.",
        ),
        "UserExec.TrapFrameReady": MappingRule(
            mapping_kind="exact",
            linux_file="arch/riscv/kernel/process.c",
            linux_symbol="start_thread",
            anchor_pattern=r"\bregs->epc\s*=\s*pc\s*;",
            confidence="high",
            notes=(
                "RISC-V start_thread() installs the user entry PC and stack in "
                "pt_regs for runtime exec return; instrumentation must guard "
                "against boot init kernel_execve() ownership."
            ),
        ),
        "UserExec.SatpReady": MappingRule(
            mapping_kind="exact",
            linux_file="fs/exec.c",
            linux_symbol="exec_mmap",
            anchor_pattern=r"\bactivate_mm\s*\(\s*active_mm\s*,\s*mm\s*\)\s*;",
            confidence="medium",
            notes=(
                "Linux exec_mmap() installs and activates the runtime exec mm; "
                "Linux has no separate arceos_ex satp token boundary, and "
                "instrumentation must guard against boot init kernel_execve() "
                "ownership."
            ),
        ),
        "UserExec.ContextReplaced": MappingRule(
            mapping_kind="exact",
            linux_file="fs/exec.c",
            linux_symbol="begin_new_exec",
            anchor_pattern=r"\bretval\s*=\s*exec_mmap\s*\(\s*bprm->mm\s*\)\s*;",
            confidence="medium",
            notes=(
                "Linux begin_new_exec() crosses the runtime exec point-of-no-return "
                "and hands the nascent exec mm to exec_mmap(); instrumentation "
                "must guard against boot init kernel_execve() ownership."
            ),
        ),
        "UserExec.SatpSwitched": MappingRule(
            mapping_kind="exact",
            linux_file="arch/riscv/kernel/entry.S",
            linux_symbol="ret_from_exception",
            anchor_pattern=r"^[ \t]*csrw[ \t]+CSR_STATUS,\s*a0\b",
            confidence="medium",
            notes=(
                "RISC-V return-to-user path restores trap CSRs before sret; "
                "runtime instrumentation must guard on saved SR_SPP == 0, record "
                "before restoring temporary registers and avoid treating every "
                "ordinary syscall return as a UserExec return."
            ),
        ),
        "UserExec.ReturnFrameReady": MappingRule(
            mapping_kind="exact",
            linux_file="arch/riscv/kernel/entry.S",
            linux_symbol="ret_from_exception",
            anchor_pattern=r"^[ \t]*sret\b",
            confidence="medium",
            notes=(
                "RISC-V ret_from_exception reaches the final sret return-to-user "
                "boundary; runtime instrumentation must guard on saved SR_SPP == 0, "
                "record before the final general-register restore and avoid treating "
                "every ordinary syscall return as a UserExec return."
            ),
        ),
        "UserBoot.MainElfReady": MappingRule(
            mapping_kind="exact",
            linux_file="fs/binfmt_elf.c",
            linux_symbol="load_elf_binary",
            anchor_pattern=r"\belf_phdata\s*=\s*load_elf_phdrs\s*\(\s*elf_ex\s*,\s*bprm->file\s*\)\s*;",
            confidence="high",
            notes=(
                "Boot-time init exec view of the main ELF program-header parse; "
                "this shares the runtime UserExec.MainElfReady Linux anchor but "
                "is reached from kernel_init() via kernel_execve(), so runtime "
                "instrumentation must not emit UserExec.MainElfReady for this owner."
            ),
        ),
        "UserBoot.InterpreterReady": MappingRule(
            mapping_kind="exact",
            linux_file="fs/binfmt_elf.c",
            linux_symbol="load_elf_binary",
            anchor_pattern=r"\binterp_elf_phdata\s*=\s*load_elf_phdrs\s*\(\s*interp_elf_ex\s*,\s*interpreter\s*\)\s*;",
            confidence="high",
            notes=(
                "Boot-time init exec view of the PT_INTERP program-header parse "
                "when an interpreter is present; the same Linux loader anchor is "
                "used by runtime UserExec, so runtime instrumentation must not "
                "emit UserExec.InterpreterReady for this owner."
            ),
        ),
        "UserBoot.InitAttemptFailed": MappingRule(
            mapping_kind="range",
            linux_file="init/main.c",
            linux_symbol="try_to_run_init_process",
            start_anchor_pattern=r"\bret\s*=\s*run_init_process\s*\(\s*init_filename\s*\)\s*;",
            end_anchor_pattern=r"\breturn\s+ret\s*;",
            confidence="medium",
            notes="Linux folds a default init candidate attempt, non-ENOENT diagnostic and fallback return into try_to_run_init_process(); there is no separate stage/reason checkpoint.",
        ),
        "UserBoot.AddressSpaceSetupStart": MappingRule(
            mapping_kind="exact",
            linux_file="fs/binfmt_elf.c",
            linux_symbol="load_elf_binary",
            anchor_pattern=r"\bretval\s*=\s*begin_new_exec\s*\(\s*bprm\s*\)\s*;",
            confidence="medium",
            notes=(
                "Boot-time init exec reaches the first new-exec/address-space "
                "handoff in load_elf_binary(); Linux has no UserBoot-specific "
                "address-space object boundary, and runtime UserExec records "
                "must be guarded out for this owner."
            ),
        ),
        "UserInitProcess.EnterUserMode": MappingRule(
            mapping_kind="exact",
            linux_file="arch/riscv/kernel/entry.S",
            linux_symbol="ret_from_exception",
            anchor_pattern=r"^[ \t]*sret\b",
            confidence="medium",
            notes=(
                "RISC-V ret_from_exception final sret is the architecture-scoped "
                "boot init return-to-user handoff; runtime instrumentation must "
                "guard on saved SR_SPP == 0, record before the final "
                "general-register restore and avoid reporting every later "
                "ordinary syscall return as another UserInitProcess entry."
            ),
        ),
        "UserAddressSpace.Ready": MappingRule(
            mapping_kind="exact",
            linux_file="fs/exec.c",
            linux_symbol="exec_mmap",
            anchor_pattern=r"\bactivate_mm\s*\(\s*active_mm\s*,\s*mm\s*\)\s*;",
            confidence="medium",
            notes="Linux exec_mmap() installs and activates the new mm; Linux does not expose a separate UserAddressSpace object boundary.",
        ),
        "SyscallTable.OpenAt": MappingRule(
            mapping_kind="exact",
            linux_file="fs/open.c",
            linux_symbol="SYSCALL_DEFINE4(openat)",
            anchor_pattern=r"\breturn\s+do_sys_open\s*\(\s*dfd\s*,\s*filename\s*,\s*flags\s*,\s*mode\s*\)\s*;",
            confidence="high",
            notes="Linux openat syscall wrapper dispatches to do_sys_open().",
        ),
        "SyscallTable.Read": MappingRule(
            mapping_kind="exact",
            linux_file="fs/read_write.c",
            linux_symbol="SYSCALL_DEFINE3(read)",
            anchor_pattern=r"\breturn\s+ksys_read\s*\(\s*fd\s*,\s*buf\s*,\s*count\s*\)\s*;",
            confidence="high",
            notes="Linux read syscall wrapper dispatches to ksys_read().",
        ),
        "SyscallTable.Write": MappingRule(
            mapping_kind="exact",
            linux_file="fs/read_write.c",
            linux_symbol="SYSCALL_DEFINE3(write)",
            anchor_pattern=r"\breturn\s+ksys_write\s*\(\s*fd\s*,\s*buf\s*,\s*count\s*\)\s*;",
            confidence="high",
            notes="Linux write syscall wrapper dispatches to ksys_write().",
        ),
        "SyscallTable.Writev": MappingRule(
            mapping_kind="exact",
            linux_file="fs/read_write.c",
            linux_symbol="SYSCALL_DEFINE3(writev)",
            anchor_pattern=r"\breturn\s+do_writev\s*\(\s*fd\s*,\s*vec\s*,\s*vlen\s*,\s*0\s*\)\s*;",
            confidence="high",
            notes="Linux writev syscall wrapper dispatches to do_writev().",
        ),
        "SyscallTable.Close": MappingRule(
            mapping_kind="exact",
            linux_file="fs/open.c",
            linux_symbol="SYSCALL_DEFINE1(close)",
            anchor_pattern=r"\bfile\s*=\s*file_close_fd\s*\(\s*fd\s*\)\s*;",
            confidence="high",
            notes="Linux close syscall wrapper removes the fd entry before flushing and fput handling.",
        ),
        "SyscallTable.NewFstatAt": MappingRule(
            mapping_kind="exact",
            linux_file="fs/stat.c",
            linux_symbol="SYSCALL_DEFINE4(newfstatat)",
            anchor_pattern=r"\berror\s*=\s*vfs_fstatat\s*\(\s*dfd\s*,\s*filename\s*,\s*&stat\s*,\s*flag\s*\)\s*;",
            confidence="high",
            notes="Linux newfstatat syscall wrapper dispatches to vfs_fstatat() before stat copyout.",
        ),
        "SyscallTable.SetTidAddress": MappingRule(
            mapping_kind="exact",
            linux_file="kernel/fork.c",
            linux_symbol="SYSCALL_DEFINE1(set_tid_address)",
            anchor_pattern=r"\bcurrent->clear_child_tid\s*=\s*tidptr\s*;",
            confidence="high",
            notes="Linux set_tid_address syscall wrapper records current->clear_child_tid and returns task_pid_vnr().",
        ),
        "SyscallTable.Clone": MappingRule(
            mapping_kind="exact",
            linux_file="kernel/fork.c",
            linux_symbol="kernel_clone",
            anchor_pattern=r"\bp\s*=\s*copy_process\s*\(\s*NULL\s*,\s*trace\s*,\s*NUMA_NO_NODE\s*,\s*args\s*\)\s*;",
            confidence="medium",
            notes="Mapped to shared kernel_clone() because the legacy clone syscall ABI wrapper is arch/config conditional on CONFIG_CLONE_BACKWARDS variants.",
        ),
        "SyscallTable.RtSigtimedwait": MappingRule(
            mapping_kind="exact",
            linux_file="kernel/signal.c",
            linux_symbol="SYSCALL_DEFINE4(rt_sigtimedwait)",
            anchor_pattern=r"\breturn\s+do_sigtimedwait\s*\(\s*&these\s*,\s*uinfo\s*,\s*uts\s*\)\s*;",
            confidence="high",
            notes="Linux rt_sigtimedwait syscall wrapper validates sigset size, copies the user mask, and dispatches to do_sigtimedwait().",
        ),
        "SyscallTable.Wait4": MappingRule(
            mapping_kind="exact",
            linux_file="kernel/exit.c",
            linux_symbol="SYSCALL_DEFINE4(wait4)",
            anchor_pattern=r"\blong\s+err\s*=\s*kernel_wait4\s*\(\s*upid\s*,\s*stat_addr\s*,\s*options\s*,\s*ru\s*\?\s*&r\s*:\s*NULL\s*\)\s*;",
            confidence="high",
            notes="Linux wait4 syscall wrapper dispatches to kernel_wait4().",
        ),
        "UserChild.ParentWaitResumed": MappingRule(
            mapping_kind="exact",
            linux_file="kernel/exit.c",
            linux_symbol="kernel_wait4",
            anchor_pattern=r"\bif\s*\(\s*ret\s*>\s*0\s*&&\s*stat_addr\s*&&\s*put_user\s*\(\s*wo\.wo_stat\s*,\s*stat_addr\s*\)\s*\)",
            confidence="medium",
            notes="Linux kernel_wait4() wait completion and status copyout boundary before returning the child pid to the parent.",
        ),
        "SyscallTable.Exit": MappingRule(
            mapping_kind="exact",
            linux_file="kernel/exit.c",
            linux_symbol="do_group_exit",
            anchor_pattern=r"\bdo_exit\s*\(\s*exit_code\s*\)\s*;",
            confidence="medium",
            notes="Checkpoint covers the first slice of exit/exit_group; this anchor is the shared exit_group path while plain sys_exit reaches adjacent do_exit().",
        ),
    }


def default_unmapped_notes() -> dict[str, str]:
    return {
        "EntryPreludePhase.Started": (
            "EntryPreludePhase starts before this mapping pass claims a portable "
            "Linux init/main.c anchor; RISC-V head.S entry mapping is left for a later pass."
        ),
        "EntryPreludePhase.Ready": (
            "EntryPreludePhase completes before this mapping pass claims a portable "
            "Linux init/main.c anchor; RISC-V head.S entry mapping is left for a later pass."
        ),
    }


def load_inventory(path: Path) -> list[CheckpointInventoryRecord]:
    raw_rows = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(raw_rows, list):
        raise LinuxCheckpointMappingError("checkpoint inventory must be a JSON array")

    records: list[CheckpointInventoryRecord] = []
    for row_number, row in enumerate(raw_rows, start=1):
        if not isinstance(row, dict):
            raise LinuxCheckpointMappingError(f"inventory row {row_number} must be an object")
        try:
            index = row["index"]
            variant = row["variant"]
            name = row["name"]
        except KeyError as exc:
            raise LinuxCheckpointMappingError(
                f"inventory row {row_number} missing field {exc.args[0]}"
            ) from exc
        if not isinstance(index, int) or not isinstance(variant, str) or not isinstance(name, str):
            raise LinuxCheckpointMappingError(
                f"inventory row {row_number} has invalid index/variant/name types"
            )
        records.append(CheckpointInventoryRecord(index=index, variant=variant, name=name))

    return records


def map_checkpoints(
    records: Iterable[CheckpointInventoryRecord],
    linux_tree: Path = DEFAULT_LINUX_TREE,
    rules: dict[str, MappingRule] | None = None,
) -> list[LinuxCheckpointMappingRecord]:
    active_rules = default_mapping_rules() if rules is None else rules
    unmapped_notes = default_unmapped_notes()
    linux_sources = LinuxSourceIndex(linux_tree)
    mapped: list[LinuxCheckpointMappingRecord] = []

    for record in records:
        rule = active_rules.get(record.name)
        if rule is None:
            mapped.append(
                _unmapped(
                    record,
                    unmapped_notes.get(
                        record.name,
                        "No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass.",
                    ),
                )
            )
            continue
        mapped.append(_resolve_rule(record, rule, linux_sources))

    return mapped


def _markdown_escape(value: str) -> str:
    return value.replace("|", r"\|")


def _cell(value: str | None) -> str:
    if value is None:
        return "null"
    return _markdown_escape(value)


def render_json(records: list[LinuxCheckpointMappingRecord]) -> str:
    json_rows = [asdict(record) for record in records]
    return json.dumps(json_rows, indent=2, ensure_ascii=False) + "\n"


def render_markdown(records: list[LinuxCheckpointMappingRecord]) -> str:
    counts = {kind: 0 for kind in ("exact", "range", "unmapped")}
    for record in records:
        counts[record.mapping_kind] = counts.get(record.mapping_kind, 0) + 1

    lines = [
        "# Linux Checkpoint Mapping",
        "",
        f"- exact: {counts.get('exact', 0)}",
        f"- range: {counts.get('range', 0)}",
        f"- unmapped: {counts.get('unmapped', 0)}",
        "",
        "| checkpoint_index | checkpoint_name | checkpoint_variant | mapping_kind | confidence | linux_file | linux_symbol | linux_anchor | notes |",
        "| ---: | --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    for record in records:
        lines.append(
            "| "
            + " | ".join(
                [
                    str(record.checkpoint_index),
                    _cell(record.checkpoint_name),
                    _cell(record.checkpoint_variant),
                    _cell(record.mapping_kind),
                    _cell(record.confidence),
                    _cell(record.linux_file),
                    _cell(record.linux_symbol),
                    _cell(record.linux_anchor),
                    _cell(record.notes),
                ]
            )
            + " |"
        )
    lines.append("")
    return "\n".join(lines)


def _expected_outputs(records: list[LinuxCheckpointMappingRecord]) -> dict[str, str]:
    return {
        JSON_NAME: render_json(records),
        MARKDOWN_NAME: render_markdown(records),
    }


def write_outputs(
    records: list[LinuxCheckpointMappingRecord],
    out_dir: Path = DEFAULT_OUT_DIR,
) -> tuple[Path, Path]:
    out_dir.mkdir(parents=True, exist_ok=True)
    json_path = out_dir / JSON_NAME
    markdown_path = out_dir / MARKDOWN_NAME

    outputs = _expected_outputs(records)
    json_path.write_text(outputs[JSON_NAME], encoding="utf-8")
    markdown_path.write_text(outputs[MARKDOWN_NAME], encoding="utf-8")
    return json_path, markdown_path


def check_outputs(
    records: list[LinuxCheckpointMappingRecord],
    out_dir: Path = DEFAULT_OUT_DIR,
) -> list[ArtifactDrift]:
    drift: list[ArtifactDrift] = []
    for name, expected in _expected_outputs(records).items():
        path = out_dir / name
        if not path.is_file():
            drift.append(ArtifactDrift(path, "missing"))
            continue
        actual = path.read_text(encoding="utf-8")
        if actual != expected:
            drift.append(ArtifactDrift(path, "content differs"))
    return drift


def _count_by_kind(records: Iterable[LinuxCheckpointMappingRecord]) -> dict[str, int]:
    counts: dict[str, int] = {}
    for record in records:
        counts[record.mapping_kind] = counts.get(record.mapping_kind, 0) + 1
    return counts


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Map arceos_ex checkpoints to read-only Linux source anchors.",
    )
    parser.add_argument(
        "--input",
        type=Path,
        default=DEFAULT_INVENTORY,
        help="arceos_ex checkpoint inventory JSON.",
    )
    parser.add_argument(
        "--linux-tree",
        type=Path,
        default=DEFAULT_LINUX_TREE,
        help="Read-only Linux reference source tree.",
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=DEFAULT_OUT_DIR,
        help="Directory for JSON and Markdown mapping outputs.",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Regenerate in memory and fail if tracked outputs have drifted.",
    )
    args = parser.parse_args(argv)

    if args.check and not args.linux_tree.is_dir():
        print(f"Linux reference tree is missing: {args.linux_tree}", file=sys.stderr)
        return 1

    inventory = load_inventory(args.input)
    records = map_checkpoints(inventory, args.linux_tree)
    if args.check:
        drift = check_outputs(records, args.out_dir)
        if drift:
            print("Linux checkpoint mapping artifact drift detected:", file=sys.stderr)
            for item in drift:
                print(f"{item.reason}: {item.path}", file=sys.stderr)
            return 1
        counts = _count_by_kind(records)
        print(
            f"Linux checkpoint mapping artifacts are current ({len(records)} mappings; "
            f"exact={counts.get('exact', 0)} range={counts.get('range', 0)} "
            f"unmapped={counts.get('unmapped', 0)})"
        )
        return 0

    json_path, markdown_path = write_outputs(records, args.out_dir)
    counts = _count_by_kind(records)
    print(f"wrote {len(records)} Linux checkpoint mappings")
    print(f"exact={counts.get('exact', 0)} range={counts.get('range', 0)} unmapped={counts.get('unmapped', 0)}")
    print(json_path)
    print(markdown_path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
