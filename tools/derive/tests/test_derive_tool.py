from __future__ import annotations

import contextlib
import io
import tempfile
import unittest
from pathlib import Path

from common import DERIVE_SCHEMA, DERIVE_VERSION, read_json
from derive_tool.__main__ import main as derive_main
from model_tool.__main__ import main as model_main
from parse_tool.__main__ import main as parse_main


class DeriveToolTests(unittest.TestCase):
    def setUp(self) -> None:
        self.spec = (
            Path(__file__).resolve().parents[3]
            / "spec"
            / "entry-prelude-object-model.spec"
        )

    def _build_model_json(self, tmp: str) -> Path:
        ast = Path(tmp) / "entry-prelude-object-model.ast.json"
        model = Path(tmp) / "entry-prelude-object-model.model.json"
        self.assertEqual(parse_main([str(self.spec), "-o", str(ast)]), 0)
        self.assertEqual(model_main([str(ast), "-o", str(model)]), 0)
        return model

    def test_derive_writes_derive_json(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            model = self._build_model_json(tmp)
            derive = Path(tmp) / "entry-prelude-object-model.derive.json"

            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exit_code = derive_main([str(model), "-o", str(derive)])

            self.assertEqual(exit_code, 0)
            data = read_json(derive)
            self.assertEqual(data["schema"], DERIVE_SCHEMA)
            self.assertEqual(data["version"], DERIVE_VERSION)
            self.assertTrue(data["target"]["reached"])
            self.assertTrue(data["summary"]["ok"])
            self.assertEqual(data["summary"]["blocked"], 0)
            self.assertEqual(data["summary"]["contradiction"], 0)
            self.assertEqual(data["states"]["StartupTimeline"], "Ready")

    def test_derive_json_contains_records_transitions_and_trace(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            model = self._build_model_json(tmp)
            derive = Path(tmp) / "entry-prelude-object-model.derive.json"
            self.assertEqual(derive_main([str(model), "-o", str(derive)]), 0)

            data = read_json(derive)
            self.assertEqual(data["summary"]["transitions"], 29)
            self.assertEqual(data["summary"]["obligation"], 0)
            self.assertIn("obligation_categories", data["summary"])
            self.assertNotIn(
                "assumption_candidate", data["summary"]["obligation_categories"]
            )
            self.assertNotIn("auto_candidate", data["summary"]["obligation_categories"])
            self.assertNotIn("spec_gap", data["summary"]["obligation_categories"])
            self.assertNotIn("unknown", data["summary"]["obligation_categories"])
            self.assertTrue(
                any(
                    transition["object"] == "StartupTimeline"
                    and transition["event"] == "Setup"
                    for transition in data["transitions"]
                )
            )
            self.assertEqual(len(data["trace"]), 1)
            root = data["trace"][0]
            self.assertEqual(root["object"], "StartupTimeline")
            self.assertEqual(root["event"], "Setup")
            self.assertEqual(root["source_state"], "Base")
            self.assertEqual(root["target_state"], "Ready")
            self.assertEqual(root["status"], "proved")
            self.assertGreater(len(root["children"]), 0)
            self.assertTrue(
                any(record["span"] is not None for record in data["records"])
            )
            obligations = [
                record for record in data["records"] if record["status"] == "obligation"
            ]
            proved = [record for record in data["records"] if record["status"] == "proved"]
            self.assertFalse(
                any(record["proof_class"] == "register_effect" for record in obligations)
            )
            self.assertFalse(
                any(
                    record["predicate"] in ("valid_task_ref", "valid_stack_pointer")
                    for record in obligations
                )
            )
            self.assertNotIn(
                "auto_candidate", data["summary"]["obligation_categories"]
            )
            self.assertFalse(
                any(record["proof_provider"] == "builtin_candidate" for record in obligations)
            )
            self.assertFalse(obligations)
            self.assertFalse(
                any(record["proof_provider"] == "derived_candidate" for record in obligations)
            )
            self.assertFalse(
                any(
                    record["predicate"]
                    in ("linear_map_area_reserved", "fixmap_adjacent_to_linear_map")
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "linear_map_area_reserved"
                    and record["proof_class"] == "address_layout"
                    and record["proof_provider"] == "config_address_layout"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "fixmap_adjacent_to_linear_map"
                    and record["proof_class"] == "address_layout"
                    and record["proof_provider"] == "config_address_layout"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "has_slot"
                    and record["proof_class"] == "config_structure"
                    and record["proof_provider"] == "builtin"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "readonly"
                    and record["proof_class"] == "object_attribute"
                    and record["proof_provider"] == "builtin"
                    for record in proved
                )
            )
            self.assertFalse(
                any(record["predicate"] == "disjoint" for record in obligations)
            )
            self.assertTrue(
                any(
                    record["predicate"] == "disjoint"
                    and record["proof_class"] == "platform_memory_layout"
                    and record["proof_provider"] == "fdt_memory_layout"
                    for record in proved
                )
            )
            self.assertFalse(
                any(
                    record["predicate"]
                    in ("kernel_fpu_disabled", "kernel_vector_disabled")
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "kernel_fpu_disabled"
                    and record["proof_class"] == "riscv_status_register"
                    and record["proof_provider"] == "event_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "kernel_vector_disabled"
                    and record["proof_class"] == "riscv_status_register"
                    and record["proof_provider"] == "event_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "contains"
                    and record["proof_class"] == "dtb_header_range"
                    and record["proof_provider"] == "opensbi_firmware"
                    and record["expression"] == "contains(PhysicalMemory.ram, header_range)"
                    for record in proved
                )
            )
            self.assertFalse(
                any(
                    record["expression"]
                    in ("boot_hartid == Riscv64.a0", "dtb_pa == Riscv64.a1")
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "boot_hartid == Riscv64.a0"
                    and record["proof_class"] == "boot_arguments"
                    and record["proof_provider"] == "riscv_boot_protocol"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "dtb_pa == Riscv64.a1"
                    and record["proof_class"] == "boot_arguments"
                    and record["proof_provider"] == "riscv_boot_protocol"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "header_range.start == BootArgs.dtb_pa"
                    and record["proof_class"] == "dtb_header_range"
                    and record["proof_provider"] == "event_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "range.end == BootArgs.dtb_pa + header.total_size"
                    and record["proof_class"] == "dtb_range"
                    and record["proof_provider"] == "event_ensures"
                    for record in proved
                )
            )
            self.assertFalse(
                any(
                    record["expression"] == "fdt_slot == Config.fixmap.fdt"
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "fdt_slot == Config.fixmap.fdt"
                    and record["proof_class"] == "fixmap_layout"
                    and record["proof_provider"] == "event_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "Riscv64.stvec == phys_addr(StaticObjects.early_event_entry)"
                    and record["proof_class"] == "register_effect"
                    and record["proof_provider"] == "event_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "Riscv64.tp == virt_addr(StaticObjects.init_task, EarlyVm, KernelImageMap)"
                    and record["proof_class"] == "register_effect"
                    and record["proof_provider"] == "event_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "Riscv64.sp == phys_addr(Lds.init_stack_end - Config.pt_size_on_stack)"
                    and record["proof_class"] == "register_effect"
                    and record["proof_provider"] == "event_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "Riscv64.satp == satp_of(StaticObjects.early_pg_dir, Config.satp_mode)"
                    and record["proof_class"] == "register_effect"
                    and record["proof_provider"] == "event_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "kernel_image_va_window_size >= pmd_size"
                    and record["proof_class"] == "configuration"
                    and record["proof_provider"] == "config_source"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "valid_satp_mode"
                    and record["proof_class"] == "configuration"
                    and record["proof_provider"] == "config_source"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "kernel_end > kernel_start"
                    and record["proof_class"] == "linker_layout"
                    and record["proof_provider"] == "linux_linker_script"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "segments.bss.range == range(Lds.bss_start, Lds.bss_end)"
                    and record["proof_class"] == "linker_layout"
                    and record["proof_provider"] == "linux_linker_script"
                    for record in proved
                )
            )
            self.assertFalse(
                any(
                    record["expression"] == "Lds.init_stack_end - Lds.init_stack_start >= Config.page_size"
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "Lds.init_stack_end - Lds.init_stack_start >= Config.page_size"
                    and record["proof_class"] == "stack_layout"
                    and record["proof_provider"] == "config_and_linker"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "boot_cpu_hartid == BootArgs.boot_hartid"
                    and record["proof_class"] == "boot_hart_identity"
                    and record["proof_provider"] == "event_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "valid_dtb_magic"
                    and record["proof_class"] == "boot_input"
                    and record["proof_provider"] == "event_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "valid_dtb_header"
                    and record["proof_class"] == "boot_input"
                    and record["proof_provider"] == "event_ensures"
                    for record in proved
                )
            )
            self.assertFalse(
                any(record["predicate"] == "valid_hart_id" for record in obligations)
            )
            self.assertTrue(
                any(
                    record["expression"] == "platform_hart_id_valid(BootArgs.boot_hartid)"
                    and record["object"] == "PlatformCpuInfo"
                    and record["proof_class"] == "platform_cpu_description"
                    and record["proof_provider"] == "fdt_cpu_description"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "platform_hart_id_valid(boot_cpu_hartid)"
                    and record["proof_class"] == "platform_cpu_description"
                    and record["proof_provider"] == "prior_derivation_facts"
                    for record in proved
                )
            )
            self.assertFalse(
                any(
                    record["predicate"]
                    in (
                        "context_is",
                        "interrupt_concurrency_closed",
                        "task_concurrency_closed",
                    )
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "context_is"
                    and record["proof_class"] == "phase_context"
                    and record["proof_provider"] == "prior_derivation_facts"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "interrupt_concurrency_closed"
                    and record["proof_class"] == "system_exclusive_context"
                    and record["proof_provider"] == "prior_derivation_facts"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "task_concurrency_closed"
                    and record["proof_class"] == "system_exclusive_context"
                    and record["proof_provider"] == "prior_derivation_facts"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "sbi_hsm_available"
                    and record["proof_class"] == "sbi_hsm"
                    and record["proof_provider"] == "riscv_sbi_spec"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "primary_hart_sie_clear_at_kernel_entry"
                    and record["proof_class"] == "firmware_entry_state"
                    and record["proof_provider"] == "opensbi_firmware"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "primary_hart_only_at_kernel_entry"
                    and record["proof_class"] == "firmware_entry_state"
                    and record["proof_provider"] == "opensbi_firmware"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "ordered_booting_enabled"
                    and record["proof_class"] == "firmware_boot_policy"
                    and record["proof_provider"] == "opensbi_firmware"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "firmware_dtb_blob_in_ram_at_kernel_entry"
                    and record["proof_class"] == "firmware_entry_state"
                    and record["proof_provider"] == "opensbi_firmware"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "contains"
                    and record["proof_class"] == "physical_memory_membership"
                    and record["proof_provider"] == "opensbi_firmware"
                    and record["expression"] == "contains(PhysicalMemory.ram, range)"
                    for record in proved
                )
            )
            self.assertFalse(
                any(
                    record["predicate"] == "slot_contains"
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "slot_contains(fdt_slot, RawDtb)"
                    and record["proof_class"] == "fixmap_slot_content"
                    and record["proof_provider"] == "event_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "slot_contains(FixMap.fdt_slot, RawDtb)"
                    and record["proof_class"] == "fixmap_slot_content"
                    and record["proof_provider"] == "event_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"] == "slot_contains(FixMap.fdt_slot, RawDtb)"
                    and record["proof_class"] == "fixmap_slot_content"
                    and record["proof_provider"] == "prior_derivation_facts"
                    for record in proved
                )
            )
            self.assertFalse(
                any(
                    record["predicate"] == "fits_in_kernel_image_map"
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "fits_in_kernel_image_map"
                    and record["proof_class"] == "address_mapping"
                    and record["proof_provider"] == "config_and_linker"
                    for record in proved
                )
            )
            self.assertFalse(
                any(
                    record["predicate"] == "fits_in_fixmap_slot"
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "fits_in_fixmap_slot"
                    and record["proof_class"] == "address_mapping"
                    and record["proof_provider"] == "riscv_fixmap_layout"
                    for record in proved
                )
            )
            self.assertFalse(
                any(
                    record["predicate"]
                    in (
                        "kernel_image_mapping_ready",
                        "fixmap_slot_mapping_ready",
                    )
                    for record in obligations
                )
            )
            self.assertFalse(
                any(
                    record["predicate"] == "kernel_image_accessible"
                    for record in obligations
                )
            )
            self.assertFalse(
                any(
                    record["predicate"] == "fixmap_slot_accessible"
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "kernel_image_accessible"
                    and record["proof_class"] == "address_mapping"
                    and record["proof_provider"] == "event_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "fixmap_slot_accessible"
                    and record["proof_class"] == "address_mapping"
                    and record["proof_provider"] == "event_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "valid_virt_addr"
                    and record["proof_class"] == "address_mapping"
                    and record["proof_provider"] == "config_source"
                    for record in proved
                )
            )
            self.assertFalse(
                any(
                    record["predicate"] == "trampoline_mapping_ready"
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "trampoline_mapping_ready"
                    and record["proof_class"] == "address_mapping"
                    and record["proof_provider"] == "event_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "trampoline_mapping_ready"
                    and record["proof_class"] == "address_mapping"
                    and record["proof_provider"] == "prior_derivation_facts"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "kernel_image_mapping_ready"
                    and record["proof_class"] == "address_mapping"
                    and record["proof_provider"] == "event_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "fixmap_slot_mapping_ready"
                    and record["proof_class"] == "address_mapping"
                    and record["proof_provider"] == "event_ensures"
                    for record in proved
                )
            )
            self.assertFalse(
                any(
                    record["predicate"] == "valid_trampoline_map"
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "valid_trampoline_map"
                    and record["proof_class"] == "address_mapping"
                    and record["proof_provider"] == "config_and_linker"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "valid_object_storage"
                    and record["proof_class"] == "object_storage"
                    and record["proof_provider"] == "linux_static_objects"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "valid_function_symbol"
                    and record["proof_class"] == "linker_symbol"
                    and record["proof_provider"] == "linux_static_objects"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "valid_page_table_storage"
                    and record["proof_class"] == "object_storage"
                    and record["proof_provider"] == "linux_static_objects"
                    for record in proved
                )
            )
            self.assertFalse(
                any(
                    record["predicate"] == "valid_phys_range_set"
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "valid_phys_range_set"
                    and record["proof_class"] == "platform"
                    and record["proof_provider"] == "fdt_memory_layout"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "valid_task_storage"
                    and record["proof_class"] == "object_storage"
                    and record["proof_provider"] == "prior_derivation_facts"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "valid_task_ref"
                    and record["proof_class"] == "object_storage"
                    and record["proof_provider"] == "prior_derivation_facts"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "valid_stack_pointer"
                    and record["proof_class"] == "architecture_state"
                    and record["proof_provider"] == "prior_derivation_facts"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["expression"]
                    == "inside(Riscv64.sp, Lds.init_stack_end, Lds.init_stack_start, Lds.init_stack_end)"
                    and record["proof_class"] == "stack_layout"
                    and record["proof_provider"] == "prior_derivation_facts"
                    for record in proved
                )
            )
            self.assertFalse(
                any(
                    record["predicate"] == "soc_early_platform_ready"
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "soc_early_platform_ready"
                    and record["proof_class"] == "platform"
                    and record["proof_provider"] == "event_ensures"
                    for record in proved
                )
            )
            self.assertFalse(
                any(
                    record["predicate"] == "phys_to_virt_transition_completed"
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "valid_segment_set"
                    and record["proof_class"] == "linker_layout"
                    and record["proof_provider"] == "linux_linker_script"
                    for record in proved
                )
            )
            self.assertFalse(
                any(record["predicate"] == "memory_zeroed" for record in obligations)
            )
            self.assertTrue(
                any(
                    record["predicate"] == "memory_zeroed"
                    and record["proof_class"] == "memory_content"
                    and record["proof_provider"] == "event_ensures"
                    for record in proved
                )
            )
            self.assertFalse(
                any(
                    record["predicate"] == "gp_relative_access_ready"
                    for record in obligations
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "phys_to_virt_transition_completed"
                    and record["proof_class"] == "architecture_state"
                    and record["proof_provider"] == "event_ensures"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "gp_relative_access_ready"
                    and record["proof_class"] == "architecture_state"
                    and record["proof_provider"] == "event_ensures"
                    for record in proved
                )
            )
            attrs_providers = {
                record["object"]: (record["proof_class"], record["proof_provider"])
                for record in obligations
                if record["predicate"] == "attrs_accessible"
            }
            self.assertNotIn("BootArgs", attrs_providers)
            self.assertTrue(
                any(
                    record["object"] == "BootArgs"
                    and record["predicate"] == "attrs_accessible"
                    and record["proof_class"] == "boot_arguments"
                    and record["proof_provider"] == "riscv_boot_protocol"
                    for record in proved
                )
            )
            self.assertNotIn("Config", attrs_providers)
            self.assertTrue(
                any(
                    record["object"] == "Config"
                    and record["predicate"] == "attrs_accessible"
                    and record["proof_class"] == "config_attributes"
                    and record["proof_provider"] == "config_source"
                    for record in proved
                )
            )
            self.assertNotIn("FixMap", attrs_providers)
            self.assertTrue(
                any(
                    record["object"] == "FixMap"
                    and record["predicate"] == "attrs_accessible"
                    and record["proof_class"] == "fixmap_layout"
                    and record["proof_provider"] == "event_ensures"
                    for record in proved
                )
            )
            self.assertNotIn("Lds", attrs_providers)
            self.assertTrue(
                any(
                    record["object"] == "Lds"
                    and record["predicate"] == "attrs_accessible"
                    and record["proof_class"] == "linker_layout"
                    and record["proof_provider"] == "linux_linker_script"
                    for record in proved
                )
            )
            self.assertNotIn("PhysicalMemory", attrs_providers)
            self.assertTrue(
                any(
                    record["object"] == "PhysicalMemory"
                    and record["predicate"] == "attrs_accessible"
                    and record["proof_class"] == "platform_memory_layout"
                    and record["proof_provider"] == "fdt_memory_layout"
                    for record in proved
                )
            )
            self.assertNotIn("Riscv64", attrs_providers)
            self.assertTrue(
                any(
                    record["object"] == "Riscv64"
                    and record["predicate"] == "attrs_accessible"
                    and record["proof_class"] == "architecture_register_file"
                    and record["proof_provider"] == "riscv_isa_spec"
                    for record in proved
                )
            )
            self.assertNotIn("StaticObjects", attrs_providers)
            self.assertTrue(
                any(
                    record["object"] == "StaticObjects"
                    and record["predicate"] == "attrs_accessible"
                    and record["proof_class"] == "static_object_layout"
                    and record["proof_provider"] == "linux_static_objects"
                    for record in proved
                )
            )
            self.assertTrue(
                any(
                    record["predicate"] == "no_service"
                    and record["proof_class"] == "state_alias"
                    and record["proof_provider"] == "builtin"
                    for record in proved
                )
            )

    def test_invalid_model_schema_returns_usage_error_code(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            model = Path(tmp) / "bad.model.json"
            derive = Path(tmp) / "bad.derive.json"
            model.write_text('{"schema": "wrong", "version": 1}\n', encoding="utf-8")

            stdout = io.StringIO()
            stderr = io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exit_code = derive_main([str(model), "-o", str(derive)])

            self.assertEqual(exit_code, 2)
            self.assertIn("error: invalid model JSON", stderr.getvalue())

    def test_derive_tool_does_not_import_pyveri(self) -> None:
        source_root = Path(__file__).resolve().parents[1] / "src" / "derive_tool"

        for path in source_root.rglob("*.py"):
            text = path.read_text(encoding="utf-8")
            self.assertNotIn("from pyveri", text, str(path))
            self.assertNotIn("import pyveri", text, str(path))


if __name__ == "__main__":
    unittest.main()
