from __future__ import annotations

import unittest
from pathlib import Path

from parse_tool.parser import parse_file
from model_tool.builder import build_model
from common.model_types import ObjectDef, ObjectModel, StateDef
from common.spec_ast import Block, ObjectDecl, SourceSpan, StateDecl
from codegen_tool.linker import LinkerProfile, generate_riscv64_linker_script


class LinkerCodegenTests(unittest.TestCase):
    def test_generates_current_arceos_ex_linker_script(self) -> None:
        repo = Path(__file__).resolve().parents[3]
        document = parse_file(repo / "spec/model/main.spec")
        result = build_model(document)
        self.assertTrue(result.ok)

        generated = generate_riscv64_linker_script(
            result.model,
            LinkerProfile(
                kernel_link_addr="0xffffffff80000000",
                page_size="4K",
                boot_stack_size="16K",
            ),
        )

        expected = (repo / "impl/arceos_ex/linker/riscv64.lds").read_text(
            encoding="utf-8"
        )
        self.assertEqual(generated, expected)

    def test_rejects_missing_model_contract(self) -> None:
        model = _minimal_model_without_config_driven_lds_invariant()

        with self.assertRaisesRegex(ValueError, "Lds.Online is missing"):
            generate_riscv64_linker_script(
                model,
                LinkerProfile(
                    kernel_link_addr="0xffffffff80000000",
                    page_size="4K",
                    boot_stack_size="16K",
                ),
            )


def _minimal_model_without_config_driven_lds_invariant() -> ObjectModel:
    span = SourceSpan(1, 1)
    config_decl = ObjectDecl(
        name="Config",
        kind="PrepareObject",
        span=span,
        initial_state="Online",
    )
    config = ObjectDef(
        name="Config",
        kind="PrepareObject",
        decl=config_decl,
        initial_state="Online",
        parent=None,
        attrs={
            "boot_stack_size": "Size",
            "kernel_link_addr": "VirtAddr<KernelImage>",
            "page_size": "Size",
        },
    )

    lds_online_decl = StateDecl(
        name="Online",
        span=span,
        invariants=[
            Block(
                kind="invariant",
                body="""
                    init_stack_end - init_stack_start == boot_stack_size;
                    text_start == kernel_start;
                    inside(text_start, text_end, kernel_start, kernel_end);
                    inside(rodata_start, rodata_end, kernel_start, kernel_end);
                    inside(data_start, data_end, kernel_start, kernel_end);
                    entry_head_text_layout_ready(Lds);
                """,
                span=span,
            )
        ],
    )
    lds_online = StateDef(
        name="Online",
        object_name="Lds",
        decl=lds_online_decl,
    )
    lds_decl = ObjectDecl(
        name="Lds",
        kind="PrepareObject",
        span=span,
        initial_state="Online",
    )
    lds = ObjectDef(
        name="Lds",
        kind="PrepareObject",
        decl=lds_decl,
        initial_state="Online",
        parent=None,
        states={"Online": lds_online},
        attrs={
            "boot_stack_size": "Size",
            "bss_end": "SymbolAddr",
            "bss_start": "SymbolAddr",
            "global_pointer": "SymbolAddr",
            "head_text_range": "AddrRange",
            "init_stack_end": "SymbolAddr",
            "init_stack_start": "SymbolAddr",
            "kernel_end": "SymbolAddr",
            "kernel_start": "SymbolAddr",
            "data_end": "SymbolAddr",
            "data_start": "SymbolAddr",
            "rodata_end": "SymbolAddr",
            "rodata_start": "SymbolAddr",
            "text_end": "SymbolAddr",
            "text_start": "SymbolAddr",
        },
    )

    return ObjectModel(
        enums={},
        functions={},
        predicates={},
        types={},
        objects={"Config": config, "Lds": lds},
        children={},
    )
