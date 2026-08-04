from __future__ import annotations

import unittest
from pathlib import Path

from codegen_tool.linker import LinkerProfile, generate_riscv64_linker_script


class LinkerCodegenTests(unittest.TestCase):
    def test_generates_current_arceos_ex_linker_script(self) -> None:
        repo = Path(__file__).resolve().parents[3]
        generated = generate_riscv64_linker_script(_valid_tools2_model(), _profile())
        expected = (repo / "impl/arceos_ex/linker/riscv64.lds").read_text(
            encoding="utf-8"
        )
        self.assertEqual(generated, expected)

    def test_rejects_missing_model_contract(self) -> None:
        model = _valid_tools2_model()
        model["model"]["systems"]["Lds"]["states"]["Online"]["invariant"] = []
        with self.assertRaisesRegex(ValueError, "Lds.Online is missing"):
            generate_riscv64_linker_script(model, _profile())

    def test_rejects_v10_protocol(self) -> None:
        model = _valid_tools2_model()
        model["version"] = 10
        with self.assertRaisesRegex(ValueError, "protocol v11"):
            generate_riscv64_linker_script(model, _profile())


def _profile() -> LinkerProfile:
    return LinkerProfile(
        kernel_link_addr="0xffffffff80000000",
        page_size="4K",
        boot_stack_size="16K",
    )


def _valid_tools2_model() -> dict:
    config_attrs = sorted({"boot_stack_size", "kernel_link_addr", "page_size"})
    lds_attrs = sorted(
        {
            "boot_stack_size", "bss_end", "bss_start", "global_pointer",
            "head_text_range", "init_stack_end", "init_stack_start", "kernel_end",
            "kernel_start", "per_cpu_end", "per_cpu_load", "per_cpu_start",
            "data_end", "data_start", "rodata_end", "rodata_start", "text_end",
            "text_start",
        }
    )
    invariants = sorted(
        {
            "boot_stack_size == Config.boot_stack_size",
            "init_stack_end - init_stack_start == boot_stack_size",
            "text_start == kernel_start",
            "inside(text_start, text_end, kernel_start, kernel_end)",
            "inside(rodata_start, rodata_end, kernel_start, kernel_end)",
            "inside(data_start, data_end, kernel_start, kernel_end)",
            "entry_head_text_layout_ready(Lds)",
            "per_cpu_static_image_layout_ready(Lds)",
        }
    )
    return {
        "schema": "lkm.spec.model",
        "version": 11,
        "producer": "tools2",
        "model": {
            "systems": {
                "Config": {
                    "fields": {"attrs": [{"name": name} for name in config_attrs]},
                    "states": {},
                },
                "Lds": {
                    "fields": {"attrs": [{"name": name} for name in lds_attrs]},
                    "states": {
                        "Online": {
                            "invariant": [{"text": text} for text in invariants]
                        }
                    },
                },
            }
        },
    }
