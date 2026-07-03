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


class MapLinuxCheckpointsTests(unittest.TestCase):
    def _write_linux_fixture(self, tmp: str) -> Path:
        root = Path(tmp) / "linux"
        (root / "init").mkdir(parents=True)
        (root / "mm").mkdir(parents=True)
        (root / "kernel" / "sched").mkdir(parents=True)
        (root / "init" / "main.c").write_text(MAIN_C, encoding="utf-8")
        (root / "mm" / "mm_init.c").write_text(MM_INIT_C, encoding="utf-8")
        (root / "kernel" / "sched" / "core.c").write_text(SCHED_CORE_C, encoding="utf-8")
        (root / "init" / "do_mounts.c").write_text(DO_MOUNTS_C, encoding="utf-8")
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
        self.assertEqual(mapped[0].mapping_kind, "unmapped")
        self.assertIn("RISC-V head.S entry mapping", mapped[0].notes)
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

        self.assertEqual(by_name["StartupTimeline.Started"].linux_symbol, "start_kernel")
        self.assertEqual(by_name["MmCoreInitPhase.Ready"].linux_file, "mm/mm_init.c")
        self.assertEqual(by_name["SchedInitPhase.Ready"].linux_file, "kernel/sched/core.c")
        self.assertEqual(by_name["BootInitRestInitPhase.Ready"].linux_symbol, "rest_init")
        self.assertEqual(by_name["RootfsPhase.Ready"].linux_symbol, "prepare_namespace")
        self.assertEqual(by_name["PayloadPhase.Online"].linux_symbol, "kernel_init")


if __name__ == "__main__":
    unittest.main()
