from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[3]
TOOL_PATH = REPO_ROOT / "tools" / "checkpoints" / "map_linux_checkpoints.py"

spec = importlib.util.spec_from_file_location("map_linux_checkpoints", TOOL_PATH)
assert spec is not None
map_linux_checkpoints = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = map_linux_checkpoints
spec.loader.exec_module(map_linux_checkpoints)


MAIN_C = """
void setup_arch(char **cmdline) {}
void mm_core_init(void) {}
void sched_init(void) {}
void rest_init(void) {}
int try_to_run_init_process(const char *init_filename) { return 0; }

void start_kernel(void)
{
    char *command_line;

    setup_arch(&command_line);
    mm_core_init();
    sched_init();
    rest_init();
}

static noinline void __ref __noreturn rest_init(void)
{
    schedule_preempt_disabled();
}

static int __ref kernel_init(void *unused)
{
    do_sysctl_args();
    if (!try_to_run_init_process("/sbin/init"))
        return 0;
    return -1;
}

static noinline void __init kernel_init_freeable(void)
{
    prepare_namespace();
}
"""


MM_INIT_C = """
void __init mm_core_init(void)
{
    mem_init();
}
"""


SCHED_CORE_C = """
void __init sched_init(void)
{
    init_rt_bandwidth();
}
"""


DO_MOUNTS_C = """
void __init prepare_namespace(void)
{
    mount_root();
}
"""


HEAD_S = """
__HEAD
SYM_CODE_START(_start)
    j _start_kernel

    .global relocate_enable_mmu
relocate_enable_mmu:
    la a2, 1f
    csrw CSR_TVEC, a2
    csrw CSR_SATP, a0
1:
    load_global_pointer
    csrw CSR_SATP, a2
    ret

.Lsetup_trap_vector:
    la a0, handle_exception
    csrw CSR_TVEC, a0
    ret
SYM_CODE_END(_start)

SYM_CODE_START(_start_kernel)
    la a3, __bss_start
    la a4, __bss_stop
    ble a4, a3, .Lclear_bss_done
.Lclear_bss:
    REG_S zero, (a3)
    add a3, a3, RISCV_SZPTR
    blt a3, a4, .Lclear_bss
.Lclear_bss_done:
    mv a0, a1
    la a3, .Lsecondary_park
    csrw CSR_TVEC, a3
    call setup_vm
    call relocate_enable_mmu
    call .Lsetup_trap_vector
    tail start_kernel
SYM_CODE_END(_start_kernel)
"""


RISCV_MM_INIT_C = """
static void __init create_fdt_early_page_table(uintptr_t fix_fdt_va,
                                               uintptr_t dtb_pa)
{
    create_pmd_mapping(fixmap_pmd, fix_fdt_va, dtb_pa, PMD_SIZE, PAGE_KERNEL);
    dtb_early_va = (void *)fix_fdt_va + (dtb_pa & (PMD_SIZE - 1));
    dtb_early_pa = dtb_pa;
}

asmlinkage void __init setup_vm(uintptr_t dtb_pa)
{
    kernel_map.virt_addr = KERNEL_LINK_ADDR + kernel_map.virt_offset;
    kernel_map.phys_addr = (uintptr_t)(&_start);
    kernel_map.size = (uintptr_t)(&_end) - kernel_map.phys_addr;

    pt_ops_set_early();

    /* Setup early PGD for fixmap */
    create_pgd_mapping(early_pg_dir, FIXADDR_START,
                       fixmap_pgd_next, PGDIR_SIZE, PAGE_TABLE);
    create_pmd_mapping(fixmap_pmd, FIXADDR_START,
                       (uintptr_t)fixmap_pte, PMD_SIZE, PAGE_TABLE);

    /* Setup trampoline PGD and PMD */
    create_pgd_mapping(trampoline_pg_dir, kernel_map.virt_addr,
                       trampoline_pgd_next, PGDIR_SIZE, PAGE_TABLE);
    create_pmd_mapping(trampoline_pmd, kernel_map.virt_addr,
                       kernel_map.phys_addr, PMD_SIZE, PAGE_KERNEL_EXEC);

    create_kernel_page_table(early_pg_dir, true);
    create_fdt_early_page_table(__fix_to_virt(FIX_FDT), dtb_pa);
}
"""


class MapLinuxCheckpointsTests(unittest.TestCase):
    def _write_linux_fixture(self, tmp: str) -> Path:
        root = Path(tmp) / "linux"
        (root / "init").mkdir(parents=True)
        (root / "mm").mkdir(parents=True)
        (root / "kernel" / "sched").mkdir(parents=True)
        (root / "arch" / "riscv" / "kernel").mkdir(parents=True)
        (root / "arch" / "riscv" / "mm").mkdir(parents=True)
        (root / "init" / "main.c").write_text(MAIN_C, encoding="utf-8")
        (root / "mm" / "mm_init.c").write_text(MM_INIT_C, encoding="utf-8")
        (root / "kernel" / "sched" / "core.c").write_text(SCHED_CORE_C, encoding="utf-8")
        (root / "init" / "do_mounts.c").write_text(DO_MOUNTS_C, encoding="utf-8")
        (root / "arch" / "riscv" / "kernel" / "head.S").write_text(HEAD_S, encoding="utf-8")
        (root / "arch" / "riscv" / "mm" / "init.c").write_text(
            RISCV_MM_INIT_C,
            encoding="utf-8",
        )
        return root

    def test_fixture_maps_exact_rules_and_preserves_order(self) -> None:
        records = [
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=8,
                variant="EntryPreludePhaseStarted",
                name="EntryPreludePhase.Started",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=0,
                variant="StartupTimelineStarted",
                name="StartupTimeline.Started",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=108,
                variant="MmCoreInitPhaseReady",
                name="MmCoreInitPhase.Ready",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=352,
                variant="RootfsPhaseReady",
                name="RootfsPhase.Ready",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=401,
                variant="PayloadPhaseOnline",
                name="PayloadPhase.Online",
            ),
        ]
        with tempfile.TemporaryDirectory() as tmp:
            mapped = map_linux_checkpoints.map_checkpoints(
                records,
                linux_tree=self._write_linux_fixture(tmp),
            )

        self.assertEqual([record.checkpoint_index for record in mapped], [8, 0, 108, 352, 401])
        self.assertEqual(mapped[0].mapping_kind, "exact")
        self.assertEqual(mapped[0].linux_file, "arch/riscv/kernel/head.S")
        self.assertEqual(mapped[0].linux_symbol, "_start")
        self.assertEqual(mapped[1].linux_file, "init/main.c")
        self.assertEqual(mapped[1].linux_symbol, "start_kernel")
        self.assertEqual(mapped[1].mapping_kind, "exact")
        self.assertEqual(mapped[2].linux_file, "mm/mm_init.c")
        self.assertEqual(mapped[3].linux_symbol, "prepare_namespace")
        self.assertIn('try_to_run_init_process("/sbin/init")', mapped[4].linux_anchor)

    def test_range_rule_requires_ordered_anchors(self) -> None:
        record = map_linux_checkpoints.CheckpointInventoryRecord(
            index=10,
            variant="DemoRange",
            name="Demo.Range",
        )
        ordered = map_linux_checkpoints.MappingRule(
            mapping_kind="range",
            linux_file="init/main.c",
            linux_symbol="start_kernel",
            confidence="medium",
            notes="fixture range",
            start_anchor_pattern=r"\bsetup_arch\s*\(",
            end_anchor_pattern=r"\bsched_init\s*\(",
        )
        reversed_rule = map_linux_checkpoints.MappingRule(
            mapping_kind="range",
            linux_file="init/main.c",
            linux_symbol="start_kernel",
            confidence="medium",
            notes="fixture range",
            start_anchor_pattern=r"\bsched_init\s*\(",
            end_anchor_pattern=r"\bsetup_arch\s*\(",
        )

        with tempfile.TemporaryDirectory() as tmp:
            linux_tree = self._write_linux_fixture(tmp)
            mapped = map_linux_checkpoints.map_checkpoints(
                [record],
                linux_tree=linux_tree,
                rules={"Demo.Range": ordered},
            )[0]
            unmapped = map_linux_checkpoints.map_checkpoints(
                [record],
                linux_tree=linux_tree,
                rules={"Demo.Range": reversed_rule},
            )[0]

        self.assertEqual(mapped.mapping_kind, "range")
        self.assertIn("setup_arch", mapped.linux_anchor)
        self.assertIn("sched_init", mapped.linux_anchor)
        self.assertEqual(unmapped.mapping_kind, "unmapped")
        self.assertIn("out of order", unmapped.notes)

    def test_range_rule_with_missing_anchor_is_unmapped(self) -> None:
        record = map_linux_checkpoints.CheckpointInventoryRecord(
            index=11,
            variant="DemoMissingRange",
            name="Demo.MissingRange",
        )
        missing = map_linux_checkpoints.MappingRule(
            mapping_kind="range",
            linux_file="init/main.c",
            linux_symbol="start_kernel",
            confidence="medium",
            notes="fixture range",
            start_anchor_pattern=r"\bsetup_arch\s*\(",
            end_anchor_pattern=r"\bdoes_not_exist\s*\(",
        )

        with tempfile.TemporaryDirectory() as tmp:
            mapped = map_linux_checkpoints.map_checkpoints(
                [record],
                linux_tree=self._write_linux_fixture(tmp),
                rules={"Demo.MissingRange": missing},
            )[0]

        self.assertEqual(mapped.mapping_kind, "unmapped")
        self.assertIn("were not both found", mapped.notes)

    def test_assembly_fixture_maps_symbols_labels_and_instruction_anchors(self) -> None:
        records = [
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=8,
                variant="EntryPreludePhaseStarted",
                name="EntryPreludePhase.Started",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=9,
                variant="EntryPreludePhaseReady",
                name="EntryPreludePhase.Ready",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=31,
                variant="TrampolineVmOnline",
                name="TrampolineVm.Online",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=37,
                variant="EarlyVmReady",
                name="EarlyVm.Ready",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=38,
                variant="EarlyVmOnline",
                name="EarlyVm.Online",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=43,
                variant="EventStreamReady",
                name="EventStream.Ready",
            ),
        ]

        with tempfile.TemporaryDirectory() as tmp:
            mapped = map_linux_checkpoints.map_checkpoints(
                records,
                linux_tree=self._write_linux_fixture(tmp),
            )

        by_name = {record.checkpoint_name: record for record in mapped}
        self.assertEqual(by_name["EntryPreludePhase.Started"].linux_symbol, "_start")
        self.assertIn("definition line", by_name["EntryPreludePhase.Started"].linux_anchor)
        self.assertIn("tail start_kernel", by_name["EntryPreludePhase.Ready"].linux_anchor)
        self.assertEqual(by_name["TrampolineVm.Online"].linux_symbol, "relocate_enable_mmu")
        self.assertIn("csrw CSR_SATP, a0", by_name["TrampolineVm.Online"].linux_anchor)
        self.assertIn("create_kernel_page_table", by_name["EarlyVm.Ready"].linux_anchor)
        self.assertIn("csrw CSR_SATP, a2", by_name["EarlyVm.Online"].linux_anchor)
        self.assertIn("handle_exception", by_name["EventStream.Ready"].linux_anchor)

    def test_write_outputs_uses_fixed_record_fields(self) -> None:
        records = [
            map_linux_checkpoints.LinuxCheckpointMappingRecord(
                checkpoint_index=1,
                checkpoint_name="Demo.Ready",
                checkpoint_variant="DemoReady",
                linux_file="init/main.c",
                linux_symbol="start_kernel",
                linux_anchor="start_kernel() definition line 1",
                mapping_kind="exact",
                confidence="high",
                notes="fixture",
            )
        ]

        with tempfile.TemporaryDirectory() as tmp:
            json_path, markdown_path = map_linux_checkpoints.write_outputs(records, Path(tmp))
            rows = json.loads(json_path.read_text(encoding="utf-8"))
            markdown = markdown_path.read_text(encoding="utf-8")

        self.assertEqual(
            list(rows[0].keys()),
            [
                "checkpoint_index",
                "checkpoint_name",
                "checkpoint_variant",
                "linux_file",
                "linux_symbol",
                "linux_anchor",
                "mapping_kind",
                "confidence",
                "notes",
            ],
        )
        self.assertIn("| 1 | Demo.Ready | DemoReady | exact | high |", markdown)

    @unittest.skipUnless(
        map_linux_checkpoints.DEFAULT_LINUX_TREE.is_dir()
        and map_linux_checkpoints.DEFAULT_INVENTORY.is_file(),
        "default ../linux-6.12 tree or generated checkpoint inventory is not available",
    )
    def test_real_linux_tree_smoke_locates_expected_symbols(self) -> None:
        inventory = map_linux_checkpoints.load_inventory(map_linux_checkpoints.DEFAULT_INVENTORY)
        mapped = map_linux_checkpoints.map_checkpoints(inventory)
        by_name = {record.checkpoint_name: record for record in mapped}

        self.assertEqual(by_name["EntryPreludePhase.Started"].linux_file, "arch/riscv/kernel/head.S")
        self.assertEqual(by_name["EntryPreludePhase.Started"].linux_symbol, "_start")
        self.assertIn("tail start_kernel", by_name["EntryPreludePhase.Ready"].linux_anchor)
        self.assertNotEqual(by_name["EarlyVm.Ready"].mapping_kind, "unmapped")
        self.assertNotEqual(by_name["TrampolineVm.Ready"].mapping_kind, "unmapped")
        self.assertNotEqual(by_name["RawDtb.Ready"].mapping_kind, "unmapped")
        self.assertNotEqual(by_name["FixMap.Ready"].mapping_kind, "unmapped")
        self.assertEqual(by_name["StartupTimeline.Started"].linux_symbol, "start_kernel")
        self.assertEqual(by_name["MmCoreInitPhase.Ready"].linux_file, "mm/mm_init.c")
        self.assertEqual(by_name["SchedInitPhase.Ready"].linux_file, "kernel/sched/core.c")
        self.assertEqual(by_name["BootInitRestInitPhase.Ready"].linux_symbol, "rest_init")
        self.assertEqual(by_name["RootfsPhase.Ready"].linux_symbol, "prepare_namespace")
        self.assertEqual(by_name["PayloadPhase.Online"].linux_symbol, "kernel_init")


if __name__ == "__main__":
    unittest.main()
