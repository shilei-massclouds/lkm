from __future__ import annotations

import contextlib
import io
import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[3]
TOOL_PATH = REPO_ROOT / "tools" / "checkpoints" / "plan_linux_instrumentation.py"

spec = importlib.util.spec_from_file_location("plan_linux_instrumentation", TOOL_PATH)
assert spec is not None
plan_linux_instrumentation = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = plan_linux_instrumentation
spec.loader.exec_module(plan_linux_instrumentation)


MAIN_C = """
void setup_arch(char **cmdline) {}
void mm_core_init(void) {}
void sched_init(void) {}
void rest_init(void) {}

void start_kernel(void)
{
    char *command_line;

    setup_arch(&command_line);
    mm_core_init();
    sched_init();
    rest_init();
}
"""


class PlanLinuxInstrumentationTests(unittest.TestCase):
    def _write_linux_fixture(self, tmp: str) -> Path:
        root = Path(tmp) / "linux"
        (root / "init").mkdir(parents=True)
        (root / "init" / "main.c").write_text(MAIN_C, encoding="utf-8")
        return root

    def _line_for(self, path: Path, needle: str) -> int:
        for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
            if needle in line:
                return line_number
        raise AssertionError(f"missing fixture line containing {needle!r}")

    def _exact_row(
        self,
        linux_tree: Path,
        index: int,
        name: str,
        variant: str,
        anchor_text: str,
        confidence: str = "high",
    ) -> dict[str, object]:
        line = self._line_for(linux_tree / "init" / "main.c", anchor_text)
        return {
            "checkpoint_index": index,
            "checkpoint_name": name,
            "checkpoint_variant": variant,
            "linux_file": "init/main.c",
            "linux_symbol": "start_kernel",
            "linux_anchor": f"start_kernel() line {line}: {anchor_text}",
            "mapping_kind": "exact",
            "confidence": confidence,
            "notes": "fixture",
        }

    def _write_mapping(self, tmp: str, rows: list[dict[str, object]]) -> Path:
        path = Path(tmp) / "linux_checkpoint_mapping.json"
        path.write_text(json.dumps(rows), encoding="utf-8")
        return path

    def _write_plan_outputs(
        self,
        tmp: str,
        plan: list[plan_linux_instrumentation.LinuxInstrumentationPlanEntry],
    ) -> Path:
        out_dir = Path(tmp) / "out"
        plan_linux_instrumentation.write_outputs(plan, out_dir)
        return out_dir

    def test_exact_mapping_generates_plan_and_skips_range_unmapped(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            linux_tree = self._write_linux_fixture(tmp)
            exact = self._exact_row(
                linux_tree,
                42,
                "Demo.Started",
                "DemoStarted",
                "setup_arch(&command_line);",
            )
            rows = [
                exact,
                {
                    **self._exact_row(
                        linux_tree,
                        43,
                        "Demo.Range",
                        "DemoRange",
                        "mm_core_init();",
                        confidence="medium",
                    ),
                    "mapping_kind": "range",
                },
                {
                    "checkpoint_index": 44,
                    "checkpoint_name": "Demo.Unmapped",
                    "checkpoint_variant": "DemoUnmapped",
                    "linux_file": None,
                    "linux_symbol": None,
                    "linux_anchor": None,
                    "mapping_kind": "unmapped",
                    "confidence": "none",
                    "notes": "fixture",
                },
            ]
            records = plan_linux_instrumentation.load_mapping(
                self._write_mapping(tmp, rows)
            )
            plan = plan_linux_instrumentation.build_plan(records, linux_tree)
            json_rows = json.loads(plan_linux_instrumentation.render_json(plan))

        self.assertEqual(len(plan), 1)
        self.assertEqual(plan[0].checkpoint_index, 42)
        self.assertEqual(plan[0].checkpoint_name, "Demo.Started")
        self.assertEqual(plan[0].checkpoint_variant, "DemoStarted")
        self.assertEqual(plan[0].linux_file, "init/main.c")
        self.assertEqual(plan[0].linux_symbol, "start_kernel")
        self.assertEqual(plan[0].confidence, "high")
        self.assertTrue(plan[0].anchor_fingerprint.startswith("sha256:"))
        self.assertEqual(len(plan[0].anchor_fingerprint), len("sha256:") + 64)
        self.assertEqual(
            list(json_rows[0].keys()),
            [
                "checkpoint_index",
                "checkpoint_name",
                "checkpoint_variant",
                "linux_file",
                "linux_symbol",
                "linux_anchor",
                "confidence",
                "marker",
                "anchor_fingerprint",
            ],
        )

    def test_marker_identity_uses_name_and_variant_not_index(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            linux_tree = self._write_linux_fixture(tmp)
            rows = [
                self._exact_row(
                    linux_tree,
                    99,
                    "Demo.Started",
                    "DemoStarted",
                    "setup_arch(&command_line);",
                )
            ]
            records = plan_linux_instrumentation.load_mapping(
                self._write_mapping(tmp, rows)
            )
            entry = plan_linux_instrumentation.build_plan(records, linux_tree)[0]

        self.assertIn("name=Demo.Started", entry.marker)
        self.assertIn("variant=DemoStarted", entry.marker)
        self.assertIn(f"fingerprint={entry.anchor_fingerprint}", entry.marker)
        self.assertNotIn("checkpoint_index", entry.marker)
        self.assertNotIn("index=", entry.marker)

    def test_anchor_fingerprint_is_stable_and_ignores_marker_lines(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            linux_tree = self._write_linux_fixture(tmp)
            rows = [
                self._exact_row(
                    linux_tree,
                    1,
                    "Demo.Started",
                    "DemoStarted",
                    "setup_arch(&command_line);",
                )
            ]
            mapping_path = self._write_mapping(tmp, rows)
            records = plan_linux_instrumentation.load_mapping(mapping_path)
            entry = plan_linux_instrumentation.build_plan(records, linux_tree)[0]

            main_c = linux_tree / "init" / "main.c"
            lines = main_c.read_text(encoding="utf-8").splitlines()
            anchor_line = self._line_for(main_c, "setup_arch(&command_line);")
            lines.insert(anchor_line - 1, "    " + entry.marker)
            main_c.write_text("\n".join(lines) + "\n", encoding="utf-8")

            recomputed = plan_linux_instrumentation.build_plan(
                plan_linux_instrumentation.load_mapping(mapping_path),
                linux_tree,
            )[0]

        self.assertEqual(recomputed.anchor_fingerprint, entry.anchor_fingerprint)

    def test_anchor_fingerprint_ignores_runtime_instrumentation_lines(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            linux_tree = self._write_linux_fixture(tmp)
            rows = [
                self._exact_row(
                    linux_tree,
                    1,
                    "Demo.Started",
                    "DemoStarted",
                    "setup_arch(&command_line);",
                )
            ]
            mapping_path = self._write_mapping(tmp, rows)
            entry = plan_linux_instrumentation.build_plan(
                plan_linux_instrumentation.load_mapping(mapping_path),
                linux_tree,
            )[0]

            main_c = linux_tree / "init" / "main.c"
            lines = main_c.read_text(encoding="utf-8").splitlines()
            lines.insert(0, "#include <linux/lkm_checkpoints.h>")
            lines[1:1] = (
                "#ifdef CONFIG_LKM_CHECKPOINTS",
                "#ifndef CONFIG_RISCV_M_MODE",
                "#define LKM_RUNTIME_CHECKPOINT(id) do { } while (0)",
                "#endif",
                "#endif",
                "",
            )
            anchor_line = next(
                index
                for index, line in enumerate(lines)
                if "setup_arch(&command_line);" in line
            )
            lines.insert(anchor_line, "    LKM_RUNTIME_CHECKPOINT(DEMO_STARTED);")
            main_c.write_text("\n".join(lines) + "\n", encoding="utf-8")

            recomputed = plan_linux_instrumentation.build_plan(
                plan_linux_instrumentation.load_mapping(mapping_path),
                linux_tree,
            )[0]

        self.assertEqual(recomputed.anchor_fingerprint, entry.anchor_fingerprint)

    def test_check_mode_accepts_current_outputs(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            linux_tree = self._write_linux_fixture(tmp)
            mapping_path = self._write_mapping(
                tmp,
                [
                    self._exact_row(
                        linux_tree,
                        1,
                        "Demo.Started",
                        "DemoStarted",
                        "setup_arch(&command_line);",
                    )
                ],
            )
            plan = plan_linux_instrumentation.build_plan(
                plan_linux_instrumentation.load_mapping(mapping_path),
                linux_tree,
            )
            out_dir = Path(tmp) / "out"
            plan_linux_instrumentation.write_outputs(plan, out_dir)

            stdout = io.StringIO()
            with contextlib.redirect_stdout(stdout):
                rc = plan_linux_instrumentation.main(
                    [
                        "--input",
                        str(mapping_path),
                        "--linux-tree",
                        str(linux_tree),
                        "--out-dir",
                        str(out_dir),
                        "--check",
                    ]
                )

        self.assertEqual(rc, 0)
        self.assertIn("artifacts are current", stdout.getvalue())

    def test_check_mode_reports_drift_without_rewriting_outputs(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            linux_tree = self._write_linux_fixture(tmp)
            mapping_path = self._write_mapping(
                tmp,
                [
                    self._exact_row(
                        linux_tree,
                        1,
                        "Demo.Started",
                        "DemoStarted",
                        "setup_arch(&command_line);",
                    )
                ],
            )
            plan = plan_linux_instrumentation.build_plan(
                plan_linux_instrumentation.load_mapping(mapping_path),
                linux_tree,
            )
            out_dir = Path(tmp) / "out"
            _json_path, markdown_path = plan_linux_instrumentation.write_outputs(
                plan,
                out_dir,
            )
            stale_markdown = "# stale\n"
            markdown_path.write_text(stale_markdown, encoding="utf-8")

            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                rc = plan_linux_instrumentation.main(
                    [
                        "--input",
                        str(mapping_path),
                        "--linux-tree",
                        str(linux_tree),
                        "--out-dir",
                        str(out_dir),
                        "--check",
                    ]
                )
            markdown_after_check = markdown_path.read_text(encoding="utf-8")

        self.assertEqual(rc, 1)
        self.assertEqual(markdown_after_check, stale_markdown)
        self.assertIn("content differs", stderr.getvalue())

    def test_check_markers_reports_missing_stale_mismatch_and_accepts_clean(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            linux_tree = self._write_linux_fixture(tmp)
            rows = [
                self._exact_row(
                    linux_tree,
                    1,
                    "Demo.Clean",
                    "DemoClean",
                    "setup_arch(&command_line);",
                ),
                self._exact_row(
                    linux_tree,
                    2,
                    "Demo.Missing",
                    "DemoMissing",
                    "mm_core_init();",
                ),
                self._exact_row(
                    linux_tree,
                    3,
                    "Demo.Moved",
                    "DemoMoved",
                    "sched_init();",
                ),
            ]
            mapping_path = self._write_mapping(tmp, rows)
            plan = plan_linux_instrumentation.build_plan(
                plan_linux_instrumentation.load_mapping(mapping_path),
                linux_tree,
            )
            by_name = {entry.checkpoint_name: entry for entry in plan}
            stale_marker = plan_linux_instrumentation.marker_for(
                "Demo.Stale",
                "DemoStale",
                "sha256:" + "1" * 64,
            )
            mismatch_marker = plan_linux_instrumentation.marker_for(
                "Demo.Moved",
                "DemoMoved",
                "sha256:" + "0" * 64,
            )
            main_c = linux_tree / "init" / "main.c"
            main_c.write_text(
                main_c.read_text(encoding="utf-8")
                + "\n"
                + by_name["Demo.Clean"].marker
                + "\n"
                + mismatch_marker
                + "\n"
                + stale_marker
                + "\n",
                encoding="utf-8",
            )

            problems = plan_linux_instrumentation.check_markers(
                plan_linux_instrumentation.build_plan(
                    plan_linux_instrumentation.load_mapping(mapping_path),
                    linux_tree,
                ),
                linux_tree,
            )
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                rc = plan_linux_instrumentation.main(
                    [
                        "--input",
                        str(mapping_path),
                        "--linux-tree",
                        str(linux_tree),
                        "--check-markers",
                    ]
                )

        formatted = "\n".join(
            plan_linux_instrumentation.format_marker_problem(problem)
            for problem in problems
        )
        self.assertEqual(
            [problem.kind for problem in problems],
            ["fingerprint mismatch", "missing marker", "stale marker"],
        )
        self.assertIn("Demo.Moved", formatted)
        self.assertIn("Demo.Missing", formatted)
        self.assertIn("Demo.Stale", formatted)
        self.assertNotIn("Demo.Clean", formatted)
        self.assertEqual(rc, 1)
        self.assertIn("fingerprint mismatch", stderr.getvalue())
        self.assertIn("missing marker", stderr.getvalue())
        self.assertIn("stale marker", stderr.getvalue())
        self.assertIn("1 missing / 1 stale / 1 mismatch", stderr.getvalue())

    def test_emit_marker_patch_only_inserts_exact_plan_markers(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            linux_tree = self._write_linux_fixture(tmp)
            exact = self._exact_row(
                linux_tree,
                42,
                "Demo.Started",
                "DemoStarted",
                "setup_arch(&command_line);",
            )
            rows = [
                exact,
                {
                    **self._exact_row(
                        linux_tree,
                        43,
                        "Demo.Range",
                        "DemoRange",
                        "mm_core_init();",
                    ),
                    "mapping_kind": "range",
                },
                {
                    "checkpoint_index": 44,
                    "checkpoint_name": "Demo.Unmapped",
                    "checkpoint_variant": "DemoUnmapped",
                    "linux_file": None,
                    "linux_symbol": None,
                    "linux_anchor": None,
                    "mapping_kind": "unmapped",
                    "confidence": "none",
                    "notes": "fixture",
                },
            ]
            plan = plan_linux_instrumentation.build_plan(
                plan_linux_instrumentation.load_mapping(self._write_mapping(tmp, rows)),
                linux_tree,
            )
            out_dir = self._write_plan_outputs(tmp, plan)
            patch_path = Path(tmp) / "markers.patch"
            main_c = linux_tree / "init" / "main.c"
            before = main_c.read_text(encoding="utf-8")

            stdout = io.StringIO()
            with contextlib.redirect_stdout(stdout):
                rc = plan_linux_instrumentation.main(
                    [
                        "--linux-tree",
                        str(linux_tree),
                        "--out-dir",
                        str(out_dir),
                        "--emit-marker-patch",
                        str(patch_path),
                    ]
                )
            after = main_c.read_text(encoding="utf-8")
            patch = patch_path.read_text(encoding="utf-8")

        self.assertEqual(rc, 0)
        self.assertEqual(after, before)
        self.assertIn("--- a/init/main.c", patch)
        self.assertIn("+++ b/init/main.c", patch)
        self.assertIn("Demo.Started", patch)
        self.assertIn("+    /* LKM_CHECKPOINT", patch)
        self.assertNotIn("Demo.Range", patch)
        self.assertNotIn("Demo.Unmapped", patch)
        self.assertIn("1 insertions", stdout.getvalue())

    def test_emit_marker_patch_sorts_same_anchor_by_checkpoint_index(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            linux_tree = self._write_linux_fixture(tmp)
            rows = [
                self._exact_row(
                    linux_tree,
                    20,
                    "Demo.Second",
                    "DemoSecond",
                    "setup_arch(&command_line);",
                ),
                self._exact_row(
                    linux_tree,
                    10,
                    "Demo.First",
                    "DemoFirst",
                    "setup_arch(&command_line);",
                ),
            ]
            plan = plan_linux_instrumentation.build_plan(
                plan_linux_instrumentation.load_mapping(self._write_mapping(tmp, rows)),
                linux_tree,
            )
            out_dir = self._write_plan_outputs(tmp, plan)
            patch_path = Path(tmp) / "markers.patch"

            stdout = io.StringIO()
            with contextlib.redirect_stdout(stdout):
                rc = plan_linux_instrumentation.main(
                    [
                        "--linux-tree",
                        str(linux_tree),
                        "--out-dir",
                        str(out_dir),
                        "--emit-marker-patch",
                        str(patch_path),
                    ]
                )
            patch = patch_path.read_text(encoding="utf-8")

        added_markers = [
            line for line in patch.splitlines()
            if line.startswith("+") and "LKM_CHECKPOINT" in line
        ]
        self.assertEqual(rc, 0)
        self.assertEqual(len(added_markers), 2)
        self.assertIn("Demo.First", added_markers[0])
        self.assertIn("Demo.Second", added_markers[1])

    def test_emit_marker_patch_skips_existing_clean_marker(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            linux_tree = self._write_linux_fixture(tmp)
            rows = [
                self._exact_row(
                    linux_tree,
                    1,
                    "Demo.Started",
                    "DemoStarted",
                    "setup_arch(&command_line);",
                )
            ]
            mapping_path = self._write_mapping(tmp, rows)
            plan = plan_linux_instrumentation.build_plan(
                plan_linux_instrumentation.load_mapping(mapping_path),
                linux_tree,
            )
            out_dir = self._write_plan_outputs(tmp, plan)
            entry = plan[0]
            main_c = linux_tree / "init" / "main.c"
            lines = main_c.read_text(encoding="utf-8").splitlines()
            anchor_line = self._line_for(main_c, "setup_arch(&command_line);")
            lines.insert(anchor_line - 1, "    " + entry.marker)
            main_c.write_text("\n".join(lines) + "\n", encoding="utf-8")
            before = main_c.read_text(encoding="utf-8")
            patch_path = Path(tmp) / "markers.patch"

            stdout = io.StringIO()
            with contextlib.redirect_stdout(stdout):
                rc = plan_linux_instrumentation.main(
                    [
                        "--linux-tree",
                        str(linux_tree),
                        "--out-dir",
                        str(out_dir),
                        "--emit-marker-patch",
                        str(patch_path),
                    ]
                )
            after = main_c.read_text(encoding="utf-8")
            patch = patch_path.read_text(encoding="utf-8")

        self.assertEqual(rc, 0)
        self.assertEqual(after, before)
        self.assertEqual(patch, "")
        self.assertIn("0 insertions", stdout.getvalue())
        self.assertIn("1 existing", stdout.getvalue())

    def test_emit_marker_patch_rejects_stale_marker_without_writing_patch(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            linux_tree = self._write_linux_fixture(tmp)
            rows = [
                self._exact_row(
                    linux_tree,
                    1,
                    "Demo.Started",
                    "DemoStarted",
                    "setup_arch(&command_line);",
                )
            ]
            plan = plan_linux_instrumentation.build_plan(
                plan_linux_instrumentation.load_mapping(self._write_mapping(tmp, rows)),
                linux_tree,
            )
            out_dir = self._write_plan_outputs(tmp, plan)
            stale_marker = plan_linux_instrumentation.marker_for(
                "Demo.Stale",
                "DemoStale",
                "sha256:" + "1" * 64,
            )
            main_c = linux_tree / "init" / "main.c"
            before = main_c.read_text(encoding="utf-8")
            main_c.write_text(before + stale_marker + "\n", encoding="utf-8")
            patch_path = Path(tmp) / "markers.patch"

            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                rc = plan_linux_instrumentation.main(
                    [
                        "--linux-tree",
                        str(linux_tree),
                        "--out-dir",
                        str(out_dir),
                        "--emit-marker-patch",
                        str(patch_path),
                    ]
                )

        self.assertEqual(rc, 1)
        self.assertFalse(patch_path.exists())
        self.assertIn("stale marker", stderr.getvalue())

    def test_emit_marker_patch_rejects_same_identity_fingerprint_mismatch(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            linux_tree = self._write_linux_fixture(tmp)
            rows = [
                self._exact_row(
                    linux_tree,
                    1,
                    "Demo.Started",
                    "DemoStarted",
                    "setup_arch(&command_line);",
                )
            ]
            plan = plan_linux_instrumentation.build_plan(
                plan_linux_instrumentation.load_mapping(self._write_mapping(tmp, rows)),
                linux_tree,
            )
            out_dir = self._write_plan_outputs(tmp, plan)
            mismatch_marker = plan_linux_instrumentation.marker_for(
                "Demo.Started",
                "DemoStarted",
                "sha256:" + "0" * 64,
            )
            main_c = linux_tree / "init" / "main.c"
            main_c.write_text(
                main_c.read_text(encoding="utf-8") + mismatch_marker + "\n",
                encoding="utf-8",
            )
            patch_path = Path(tmp) / "markers.patch"

            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                rc = plan_linux_instrumentation.main(
                    [
                        "--linux-tree",
                        str(linux_tree),
                        "--out-dir",
                        str(out_dir),
                        "--emit-marker-patch",
                        str(patch_path),
                    ]
                )

        self.assertEqual(rc, 1)
        self.assertFalse(patch_path.exists())
        self.assertIn("fingerprint mismatch", stderr.getvalue())


if __name__ == "__main__":
    unittest.main()
