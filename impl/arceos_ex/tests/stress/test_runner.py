from __future__ import annotations

from datetime import datetime, timezone
import json
from pathlib import Path
import tempfile
import textwrap
import tomllib
import unittest
from unittest import mock

from . import runner


def stress_record(text: str) -> str:
    encoded = text.encode().hex()
    return (
        f"stress_mem: v=1 encoding=hex bytes={len(text.encode())} "
        f"total={len(text.encode())} overflow=0 dropped=0 data={encoded}\n"
    )


def basic_execution(
    test: str,
    artifact_dir: Path,
    text: str,
    *,
    completed: bool = True,
    expectations: bool | None = True,
    command_exit: int = 0,
) -> dict[str, object]:
    return {
        "command_exit_code": command_exit,
        "duration_seconds": 0.01,
        "artifact_dir": str(artifact_dir),
        "result_path": str(artifact_dir / "result.json"),
        "qemu_log_path": str(artifact_dir / "qemu.log"),
        "qemu_log": stress_record(text),
        "result": {
            "schema_version": 2,
            "test": test,
            "execution_status": "completed" if completed else "failed",
            "verdict": "inconclusive",
            "expectations": None if expectations is None else {"passed": expectations},
            "errors": [] if completed else ["failed"],
            "qemu": {"timed_out": False},
            "cleanup": {"private_disk_removed": True, "process_group_reaped": True},
        },
    }


def stress_case(name: str = "demo") -> dict[str, object]:
    return {
        "schema_version": 2,
        "name": name,
        "description": "demo",
        "mode": "stress",
        "runs": 1,
        "metadata": {},
        "case_path": Path(f"/tmp/{name}.toml"),
        "case_sha256": "case",
        "config_fingerprint": "config",
        "test": "user-smoke-native",
        "basic": {"test": "user-smoke-native", "config_sha256": "basic"},
        "classifier_path": Path("/tmp/classifier.toml"),
        "classifier_sha256": "classifier",
        "rules": [
            {"id": "panic", "result": "failure", "contains": ["panic"], "regex": []},
            {"id": "success", "result": "success", "contains": ["user exit status=0"], "regex": []},
        ],
    }


def difftest_case(name: str = "paired") -> dict[str, object]:
    return {
        "schema_version": 2,
        "name": name,
        "description": "paired",
        "mode": "difftest",
        "runs": 1,
        "metadata": {},
        "case_path": Path(f"/tmp/{name}.toml"),
        "case_sha256": "case",
        "config_fingerprint": "config",
        "left_test": "left-basic",
        "left_label": "left",
        "right_test": "right-basic",
        "right_label": "right",
        "left_basic": {"test": "left-basic", "config_sha256": "left"},
        "right_basic": {"test": "right-basic", "config_sha256": "right"},
        "checkpoint_scope": ["A", "B"],
        "checkpoint_scope_max_counts": {},
        "checkpoint_coverage": None,
        "observable_scope": [],
        "observable_patterns": {},
    }


class CompositeConfigTests(unittest.TestCase):
    def setUp(self) -> None:
        self.repo_root = Path(__file__).resolve().parents[4]

    def test_default_and_explicit_selection(self) -> None:
        default_paths = [path.resolve() for path in runner.DEFAULT_SUITE]
        self.assertEqual(runner._selected_case_paths([]), default_paths)
        self.assertEqual(
            [path.stem for path in default_paths],
            [
                "df-0001-user-boot",
                "df-0002-smoke-initcall",
                "df-0003-distro-sh-ls",
                "rc-local-native-timeout-focused",
                "df-0005-busybox-init-login-native",
                "df-0006-user-smoke-preempt-native",
                "df-0007-ltp-frontier-child-wait-native",
                "df-0008-user-smoke-fork-enqueue-linux-object",
                "df-0009-user-smoke-preempt-linux-object",
                "df-0010-ltp-frontier-post-read-runqueue-linux-object",
                "df-0011-user-smoke-preempt-finalize-linux-object",
                "df-0012-rc-local-direct-setup-native",
                "df-0013-fork-ofd-offset-child-return-native",
                "df-0014-checkpoint-cross-cpu-reentry-native",
                "df-0015-ltp-frontier-console-record-interleave-native",
            ],
        )
        path = Path("impl/arceos_ex/tests/stress/cases/df-0001-user-boot.toml")
        self.assertEqual(runner._selected_case_paths([path]), [path.resolve()])

    def test_all_checked_in_cases_are_v2_basic_references_without_commands(self) -> None:
        cases_dir = self.repo_root / "impl/arceos_ex/tests/stress/cases"
        forbidden = {
            "command", "setup_command", "working_directory", "env", "delayed_stdin",
            "private_disk", "build_command", "timeout_seconds", "default_runs", "paired",
        }
        loaded = []
        for path in sorted(cases_dir.glob("*.toml")):
            raw = tomllib.loads(path.read_text())
            self.assertEqual(raw["schema_version"], 2, path)
            self.assertFalse(forbidden.intersection(raw), path)
            self.assertNotIn("legacy-run", path.read_text())
            self.assertNotIn("qemu-system", path.read_text())
            self.assertNotIn("openrc", path.name.lower())
            loaded.append(runner._load_case(path, self.repo_root))
        self.assertEqual(len(loaded), 26)
        self.assertTrue(all(case["mode"] in {"stress", "difftest"} for case in loaded))

    def test_checked_in_difftest_scopes_use_exact_linux_mappings(self) -> None:
        mapping_path = self.repo_root / "tools/out/checkpoints/linux_checkpoint_mapping.json"
        mapping = json.loads(mapping_path.read_text())
        mapping_kinds = {
            row["checkpoint_name"]: row["mapping_kind"]
            for row in mapping
        }
        exact_total = sum(kind == "exact" for kind in mapping_kinds.values())
        cases_dir = self.repo_root / "impl/arceos_ex/tests/stress/cases"
        for path in sorted(cases_dir.glob("*difftest.toml")):
            raw = tomllib.loads(path.read_text())
            non_exact = {
                name: mapping_kinds.get(name)
                for name in raw.get("checkpoint_scope", [])
                if mapping_kinds.get(name) != "exact"
            }
            self.assertEqual(non_exact, {}, path)
            target = raw.get("metadata", {}).get("linux_exact_runtime_target_count")
            if target is not None:
                self.assertEqual(target, exact_total, path)

    def test_fork_ofd_offset_difftest_uses_exact_observable_sequence(self) -> None:
        path = (
            self.repo_root
            / "impl"
            / "arceos_ex"
            / "tests"
            / "stress"
            / "cases"
            / "fork-ofd-offset-difftest.toml"
        )
        case = runner._load_case(path, self.repo_root)
        self.assertEqual(case["runs"], 10)
        self.assertEqual(case["left_test"], "fork-ofd-offset-linux")
        self.assertEqual(case["right_test"], "fork-ofd-offset")
        self.assertEqual(case["checkpoint_scope"], [])
        self.assertEqual(
            case["observable_scope"],
            ["r1-shbang", "r2-bin", "r3-slash-sh"],
        )
        command_echo = "~ # printf R1=;dd bs=3 count=1 <&9 2>/dev/null;echo\n"
        self.assertEqual(
            runner._observable_sequence(
                command_echo + "R1=#!/\n",
                case["observable_scope"],
                case["observable_patterns"],
            ),
            ["r1-shbang"],
        )

    def test_difftest_effective_runs_are_capped_at_ten(self) -> None:
        case = difftest_case()
        runner._validate_effective_runs(case, 10)
        with self.assertRaisesRegex(
            runner.CompositeConfigError,
            "difftest runs must not exceed 10",
        ):
            runner._validate_effective_runs(case, 11)

    def test_rc_local_timeout_case_is_default_and_uses_canonical_basic(self) -> None:
        path = (
            self.repo_root
            / "impl"
            / "arceos_ex"
            / "tests"
            / "stress"
            / "cases"
            / "rc-local-native-timeout-focused.toml"
        )
        case = runner._load_case(path, self.repo_root)
        self.assertEqual(case["test"], "rc-local-native")
        self.assertEqual(case["runs"], 100)
        self.assertTrue(case["metadata"]["default_suite"])
        self.assertIn(path.resolve(), runner._selected_case_paths([]))
        success = next(rule for rule in case["rules"] if rule["result"] == "success")
        self.assertEqual(success["id"], "rc-local-success")
        self.assertEqual(
            runner._classify(
                "lkm-rc-local: begin\nlost+found\nlkm-rc-local: end status=0\n",
                0,
                False,
                case["rules"],
            )["id"],
            "rc-local-success",
        )

    def test_schema_v1_and_arbitrary_command_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "bad.toml"
            path.write_text('schema_version = 1\nname = "bad"\nmode = "stress"\n')
            with self.assertRaisesRegex(runner.CompositeConfigError, "schema v2"):
                runner._load_case(path, self.repo_root)
            path.write_text(textwrap.dedent("""
                schema_version = 2
                name = "bad"
                description = "bad"
                mode = "stress"
                test = "user-smoke-native"
                runs = 0
                classifier = "missing.toml"
                command = ["true"]
            """))
            with self.assertRaisesRegex(runner.CompositeConfigError, "forbidden or unknown.*command"):
                runner._load_case(path, self.repo_root)

    def test_runs_zero_validates_every_checked_in_reference_without_disk(self) -> None:
        cases = sorted((self.repo_root / "impl/arceos_ex/tests/stress/cases").glob("*.toml"))
        with mock.patch.object(runner, "_prepare_canonical_disk") as disk:
            status = runner.main([*(str(path) for path in cases), "--runs", "0", "--out-dir", "/tmp/lkm-composite-unit-dry"])
        self.assertEqual(status, 0)
        disk.assert_not_called()

    def test_df0003_keeps_historical_case_name_but_uses_scripted_shell_contract(self) -> None:
        path = (
            self.repo_root
            / "impl"
            / "arceos_ex"
            / "tests"
            / "stress"
            / "cases"
            / "df-0003-distro-sh-ls.toml"
        )
        case = runner._load_case(path, self.repo_root)
        self.assertEqual(case["name"], "df-0003-distro-sh-ls")
        self.assertEqual(case["test"], "scripted-shell")
        success = next(rule for rule in case["rules"] if rule["result"] == "success")
        self.assertEqual(success["id"], "distro-sh-ls-success")
        self.assertEqual(
            success["contains"],
            ["ld-musl-riscv64.so.1", "lost+found", "user exit status=0"],
        )
        complete_output = (
            '~ # echo "OK"\nOK\n'
            "~ # ls\nlost+found\n"
            "~ # ls /lib\nld-musl-riscv64.so.1\n"
            "~ # ls /\nlost+found\n"
            "~ # exit\nuser exit status=0"
        )
        self.assertEqual(
            runner._classify(complete_output, 0, False, case["rules"])["id"],
            "distro-sh-ls-success",
        )
        self.assertEqual(
            runner._classify(complete_output.replace("OK\n", "", 1), 0, False, case["rules"])["id"],
            "unknown-failure",
        )

    def test_busybox_login_focused_uses_wrapper_sync_acceptance(self) -> None:
        path = (
            self.repo_root
            / "impl"
            / "arceos_ex"
            / "tests"
            / "stress"
            / "cases"
            / "busybox-init-login-focused.toml"
        )
        case = runner._load_case(path, self.repo_root)
        complete_output = "\n".join(
            (
                "Welcome to Alpine!",
                "syscall trace nr=158 name=getgroups ret=1",
                "syscall trace nr=154 name=setpgid ret=0",
                "syscall trace nr=29 name=ioctl ret=0",
                "lost+found",
                "syscall trace nr=81 name=sync ret=0",
            )
        )
        self.assertEqual(
            runner._classify(complete_output, 0, False, case["rules"])["id"],
            "busybox-init-login-focused-success",
        )
        self.assertEqual(
            runner._classify(
                complete_output.replace("syscall trace nr=81 name=sync ret=0", ""),
                0,
                False,
                case["rules"],
            )["id"],
            "unknown-failure",
        )

    def test_df0005_is_default_and_uses_canonical_login_basic(self) -> None:
        path = (
            self.repo_root
            / "impl"
            / "arceos_ex"
            / "tests"
            / "stress"
            / "cases"
            / "df-0005-busybox-init-login-native.toml"
        )
        case = runner._load_case(path, self.repo_root)
        self.assertEqual(case["test"], "busybox-init-login-native")
        self.assertEqual(case["runs"], 100)
        self.assertEqual(case["metadata"]["defect"], "DF-0005")
        self.assertTrue(case["metadata"]["default_suite"])
        self.assertIn(path.resolve(), runner._selected_case_paths([]))
        self.assertEqual(
            runner._classify("irrelevant", 1, True, case["rules"])["id"],
            "timeout",
        )

    def test_df0006_is_default_and_uses_canonical_user_smoke_basic(self) -> None:
        path = (
            self.repo_root
            / "impl"
            / "arceos_ex"
            / "tests"
            / "stress"
            / "cases"
            / "df-0006-user-smoke-preempt-native.toml"
        )
        case = runner._load_case(path, self.repo_root)
        self.assertEqual(case["test"], "user-smoke-native")
        self.assertEqual(case["runs"], 100)
        self.assertEqual(case["metadata"]["defect"], "DF-0006")
        self.assertTrue(case["metadata"]["default_suite"])
        self.assertIn(path.resolve(), runner._selected_case_paths([]))
        self.assertEqual(
            runner._classify("irrelevant", 1, True, case["rules"])["id"],
            "timeout",
        )
        complete = "\n".join(
            (
                "user preempt coordinator begin",
                "user preempt children published",
                "user preempt child A writes complete",
                "user preempt child B writes complete",
                "user preempt bytes collected order=ABAABBAB",
                "user preempt children reaped",
                "user-smoke: case preempt: end status=0",
                "user exit status=0",
            )
        )
        self.assertEqual(
            runner._classify(complete, 0, False, case["rules"])["id"],
            "df-0006-user-smoke-preempt-success",
        )

    def test_df0007_is_default_and_has_explicit_provider_identities(self) -> None:
        path = (
            self.repo_root
            / "impl"
            / "arceos_ex"
            / "tests"
            / "stress"
            / "cases"
            / "df-0007-ltp-frontier-child-wait-native.toml"
        )
        case = runner._load_case(path, self.repo_root)
        self.assertEqual(case["test"], "ltp-frontier")
        self.assertEqual(case["runs"], 50)
        self.assertEqual(case["metadata"]["defect"], "DF-0007")
        self.assertTrue(case["metadata"]["default_suite"])
        self.assertIn(path.resolve(), runner._selected_case_paths([]))
        failure = "\n".join(
            (
                "first_failed=simulated-next-dispatch-preflight",
                "child wait handoff dispatch invariant failed",
                "failure_diagnostic phase=Scheduler step=SwitchTo.Preflight object=TaskPair",
            )
        )
        self.assertEqual(
            runner._classify(failure, 1, False, case["rules"])["id"],
            "df-0007-child-wait-dispatch-preflight",
        )
        detailed_failure = "\n".join(
            (
                "child wait handoff dispatch invariant failed",
                "failure_diagnostic phase=Scheduler step=SwitchTo.Preflight object=TaskPair "
                "first_failed=next-task-breakpoint-valid",
            )
        )
        self.assertEqual(
            runner._classify(detailed_failure, 1, False, case["rules"])["id"],
            "df-0007-child-wait-dispatch-preflight-detailed",
        )
        linux_object_path = path.with_name(
            "df-0007-ltp-frontier-child-wait-linux-object.toml"
        )
        linux_object_case = runner._load_case(linux_object_path, self.repo_root)
        self.assertEqual(linux_object_case["test"], "ltp-frontier-lo")
        self.assertEqual(linux_object_case["runs"], 50)
        self.assertEqual(linux_object_case["metadata"]["defect"], "DF-0007")
        self.assertEqual(linux_object_case["metadata"]["provider"], "linux-object")
        self.assertFalse(linux_object_case["metadata"]["default_suite"])
        self.assertNotIn(linux_object_path.resolve(), runner._selected_case_paths([]))

    def test_df0008_is_default_and_uses_linux_object_user_smoke(self) -> None:
        path = (
            self.repo_root
            / "impl"
            / "arceos_ex"
            / "tests"
            / "stress"
            / "cases"
            / "df-0008-user-smoke-fork-enqueue-linux-object.toml"
        )
        case = runner._load_case(path, self.repo_root)
        self.assertEqual(case["test"], "user-smoke-linux-object")
        self.assertEqual(case["runs"], 100)
        self.assertEqual(case["metadata"]["defect"], "DF-0008")
        self.assertTrue(case["metadata"]["default_suite"])
        self.assertIn(path.resolve(), runner._selected_case_paths([]))
        failure = "\n".join(
            (
                "clone_kind=plain_fork clone_plain stage=mark_enqueued",
                "declared child enqueue invariant failed",
            )
        )
        self.assertEqual(
            runner._classify(failure, 1, False, case["rules"])["id"],
            "df-0008-plain-fork-child-enqueue",
        )
        complete = "\n".join(
            (
                "user fork child private mm ok",
                "user fork parent COW unique fast path ok",
                "user fork COW reuse stress ok",
                "user-smoke: case fork_mm: end status=0",
                "user-smoke: end status=0",
                "user exit status=0",
            )
        )
        self.assertEqual(
            runner._classify(complete, 0, False, case["rules"])["id"],
            "df-0008-user-smoke-fork-enqueue-success",
        )

    def test_df0009_is_default_and_uses_linux_object_user_smoke(self) -> None:
        path = (
            self.repo_root
            / "impl"
            / "arceos_ex"
            / "tests"
            / "stress"
            / "cases"
            / "df-0009-user-smoke-preempt-linux-object.toml"
        )
        case = runner._load_case(path, self.repo_root)
        self.assertEqual(case["test"], "user-smoke-linux-object")
        self.assertEqual(case["runs"], 100)
        self.assertEqual(case["metadata"]["defect"], "DF-0009")
        self.assertTrue(case["metadata"]["default_suite"])
        self.assertIn(path.resolve(), runner._selected_case_paths([]))
        self.assertEqual(
            runner._classify("irrelevant", 1, True, case["rules"])["id"],
            "timeout",
        )
        complete = "\n".join(
            (
                "user preempt coordinator begin",
                "user preempt children published",
                "user preempt child A writes complete",
                "user preempt child B writes complete",
                "user preempt bytes collected order=ABAABBAB",
                "user preempt children reaped",
                "user-smoke: case preempt: end status=0",
                "user exit status=0",
            )
        )
        self.assertEqual(
            runner._classify(complete, 0, False, case["rules"])["id"],
            "df-0009-user-smoke-preempt-success",
        )

    def test_df0010_is_default_and_uses_post_frontier_read_diagnostic(self) -> None:
        path = (
            self.repo_root
            / "impl"
            / "arceos_ex"
            / "tests"
            / "stress"
            / "cases"
            / "df-0010-ltp-frontier-post-read-runqueue-linux-object.toml"
        )
        case = runner._load_case(path, self.repo_root)
        self.assertEqual(case["test"], "ltp-frontier-post-read-lo")
        self.assertEqual(case["runs"], 50)
        self.assertEqual(case["metadata"]["defect"], "DF-0010")
        self.assertTrue(case["metadata"]["default_suite"])
        self.assertIn(path.resolve(), runner._selected_case_paths([]))
        detailed_failure = "\n".join(
            (
                "declared child runqueue publish invariant failed",
                "failure_diagnostic phase=Scheduler step=Enqueue.Publish "
                "object=TaskRunqueue check=selected-task first_failed=runqueue-ready",
            )
        )
        self.assertEqual(
            runner._classify(detailed_failure, 1, False, case["rules"])["id"],
            "df-0010-child-runqueue-publish-detailed",
        )
        complete = "\n".join(
            (
                "Summary: TOTAL=4 PASS=4 FAIL=0 BROK=0 WARN=0 CONF=0",
                "lkm-ltp-post-read: before-second-hash",
                "c79278919640c1881a0bfe433c00a6e5d06eeec8d16f3dd212c86f363c0a53e7  "
                "/opt/ltp/run-syscalls.sh",
                "lkm-ltp-post-read: complete",
            )
        )
        self.assertEqual(
            runner._classify(complete, 0, False, case["rules"])["id"],
            "df-0010-ltp-frontier-post-read-success",
        )

    def test_df0011_is_default_and_classifies_post_validation_timeout(self) -> None:
        path = (
            self.repo_root
            / "impl"
            / "arceos_ex"
            / "tests"
            / "stress"
            / "cases"
            / "df-0011-user-smoke-preempt-finalize-linux-object.toml"
        )
        case = runner._load_case(path, self.repo_root)
        self.assertEqual(case["test"], "user-smoke-linux-object")
        self.assertEqual(case["runs"], 100)
        self.assertEqual(case["metadata"]["defect"], "DF-0011")
        self.assertTrue(case["metadata"]["default_suite"])
        self.assertIn(path.resolve(), runner._selected_case_paths([]))
        late_timeout = "\n".join(
            (
                "user preempt bytes collected order=ABAABBAB",
                "user preempt children reaped",
                "user timer preemption A-B-A round robin ok",
                "qemu-system-riscv64: terminating on signal 15 from pid 609270 (python3)",
            )
        )
        self.assertEqual(
            runner._classify(late_timeout, 1, True, case["rules"])["id"],
            "df-0011-preempt-finalize-timeout",
        )
        complete = late_timeout.replace(
            "qemu-system-riscv64: terminating on signal 15 from pid 609270 (python3)",
            "user-smoke: case preempt: end status=0\nuser exit status=0",
        )
        self.assertEqual(
            runner._classify(complete, 0, False, case["rules"])["id"],
            "df-0011-user-smoke-preempt-finalize-success",
        )

    def test_df0012_is_default_and_classifies_direct_setup_failure(self) -> None:
        path = (
            self.repo_root
            / "impl"
            / "arceos_ex"
            / "tests"
            / "stress"
            / "cases"
            / "df-0012-rc-local-direct-setup-native.toml"
        )
        case = runner._load_case(path, self.repo_root)
        self.assertEqual(case["test"], "rc-local-native")
        self.assertEqual(case["runs"], 500)
        self.assertEqual(case["metadata"]["defect"], "DF-0012")
        self.assertTrue(case["metadata"]["default_suite"])
        self.assertIn(path.resolve(), runner._selected_case_paths([]))
        failure = "\n".join(
            (
                "checkpoint: DmaCachePolicy.Ready task=BootTask",
                "arceos_ex boot init direct setup failed",
                "error=C event=S actual=P expected=P target=R",
            )
        )
        self.assertEqual(
            runner._classify(failure, 1, False, case["rules"])["id"],
            "df-0012-boot-direct-setup-after-dma-cache-policy",
        )
        complete = "\n".join(
            (
                "lkm-rc-local: begin",
                "lost+found",
                "lkm-rc-local: end status=0",
            )
        )
        self.assertEqual(
            runner._classify(complete, 0, False, case["rules"])["id"],
            "df-0012-rc-local-success",
        )

    def test_df0013_is_default_and_classifies_first_read_child_timeout(self) -> None:
        path = (
            self.repo_root
            / "impl"
            / "arceos_ex"
            / "tests"
            / "stress"
            / "cases"
            / "df-0013-fork-ofd-offset-child-return-native.toml"
        )
        case = runner._load_case(path, self.repo_root)
        self.assertEqual(case["test"], "fork-ofd-offset")
        self.assertEqual(case["runs"], 50)
        self.assertEqual(case["metadata"]["defect"], "DF-0013")
        self.assertTrue(case["metadata"]["default_suite"])
        self.assertIn(path.resolve(), runner._selected_case_paths([]))
        timeout = "\n".join(
            (
                "R1=",
                "#!/",
                "qemu-system-riscv64: terminating on signal 15 from pid 625733 (python3)",
            )
        )
        self.assertEqual(
            runner._classify(timeout, 1, True, case["rules"])["id"],
            "df-0013-first-read-child-return-timeout",
        )
        complete = "\n".join(
            (
                "R1=#!/",
                "R2=bin",
                "R3=/sh",
                "lkm-fork-ofd-offset: complete",
                "qemu-system-riscv64: terminating on signal 15 from pid 625734 (python3)",
            )
        )
        self.assertEqual(
            runner._classify(complete, 0, False, case["rules"])["id"],
            "df-0013-fork-ofd-offset-success",
        )

        linux_object_path = (
            self.repo_root
            / "impl"
            / "arceos_ex"
            / "tests"
            / "stress"
            / "cases"
            / "df-0013-fork-ofd-offset-child-return-linux-object.toml"
        )
        linux_object_case = runner._load_case(linux_object_path, self.repo_root)
        self.assertEqual(linux_object_case["test"], "fork-ofd-offset-lo")
        self.assertEqual(linux_object_case["runs"], 50)
        self.assertEqual(linux_object_case["metadata"]["defect"], "DF-0013")
        self.assertFalse(linux_object_case["metadata"]["default_suite"])
        self.assertNotIn(linux_object_path.resolve(), runner._selected_case_paths([]))
        self.assertEqual(linux_object_case["rules"], case["rules"])
        self.assertEqual(
            linux_object_case["classifier_sha256"], case["classifier_sha256"]
        )

    def test_df0014_is_default_and_classifies_cross_cpu_checkpoint_reentry(self) -> None:
        path = (
            self.repo_root
            / "impl"
            / "arceos_ex"
            / "tests"
            / "stress"
            / "cases"
            / "df-0014-checkpoint-cross-cpu-reentry-native.toml"
        )
        case = runner._load_case(path, self.repo_root)
        self.assertEqual(case["test"], "rc-local-native")
        self.assertEqual(case["runs"], 500)
        self.assertEqual(case["metadata"]["defect"], "DF-0014")
        self.assertTrue(case["metadata"]["default_suite"])
        self.assertIn(path.resolve(), runner._selected_case_paths([]))
        failure = "\n".join(
            (
                "checkpoint: Kernel.Online task=KernelInitTask",
                "lkm-rc-local: begin",
                "ccheckpoint reentry",
            )
        )
        self.assertEqual(
            runner._classify(failure, 1, False, case["rules"])["id"],
            "df-0014-checkpoint-cross-cpu-reentry",
        )
        complete = "\n".join(
            (
                "lkm-rc-local: begin",
                "lost+found",
                "lkm-rc-local: end status=0",
            )
        )
        self.assertEqual(
            runner._classify(complete, 0, False, case["rules"])["id"],
            "df-0014-rc-local-success",
        )

    def test_df0015_is_default_and_requires_atomic_ltp_pass_records(self) -> None:
        path = (
            self.repo_root
            / "impl"
            / "arceos_ex"
            / "tests"
            / "stress"
            / "cases"
            / "df-0015-ltp-frontier-console-record-interleave-native.toml"
        )
        case = runner._load_case(path, self.repo_root)
        self.assertEqual(case["test"], "ltp-frontier")
        self.assertEqual(case["runs"], 50)
        self.assertEqual(case["metadata"]["defect"], "DF-0015")
        self.assertTrue(case["metadata"]["default_suite"])
        self.assertIn(path.resolve(), runner._selected_case_paths([]))

        historical_failure = (
            "wait4 registry stage=recheck parent_slot=2 related_quiesced="
            "--- uname02: PASS (exit 0)"
        )
        self.assertEqual(
            runner._classify(historical_failure, 1, False, case["rules"])["id"],
            "df-0015-wait4-pass-record-interleave",
        )

        complete = "\n".join(
            (
                "--- uname01: PASS (exit 0)",
                "--- uname02: PASS (exit 0)",
                "--- getuid01: PASS (exit 0)",
                "--- geteuid01: PASS (exit 0)",
                "Summary: TOTAL=4 PASS=4 FAIL=0 BROK=0 WARN=0 CONF=0",
                "lkm-ltp: complete",
            )
        )
        self.assertEqual(
            runner._classify(complete, 0, False, case["rules"])["id"],
            "df-0015-ltp-frontier-records-atomic",
        )

        corrupted = complete.replace(
            "--- uname02: PASS (exit 0)",
            "wait4 registry stage=recheck related_quiesced="
            "--- uname02: PASS (exit 0)",
        )
        self.assertNotEqual(
            runner._classify(corrupted, 0, False, case["rules"])["id"],
            "df-0015-ltp-frontier-records-atomic",
        )

    def test_nonzero_suite_prepares_disk_exactly_once(self) -> None:
        first = stress_case("one")
        second = stress_case("two")
        result = {"summary": {"totals": {"success": 1, "failure": 0}}}
        with (
            mock.patch.object(runner, "_selected_case_paths", return_value=[Path("one"), Path("two")]),
            mock.patch.object(runner, "_load_case", side_effect=[first, second]),
            mock.patch.object(runner, "_prepare_canonical_disk") as disk,
            mock.patch.object(runner, "_run_case", side_effect=[result, result]),
            mock.patch.object(runner, "_print_suite_summary"),
        ):
            status = runner.main(["--runs", "1"])
        self.assertEqual(status, 0)
        disk.assert_called_once()


class BasicOrchestrationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        (self.root / "Makefile").write_text("all:\n")

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def test_stress_repeats_one_basic_at_fixed_nested_paths(self) -> None:
        calls: list[tuple[str, Path]] = []

        def execute(test: str, artifact: Path, repo_root: Path) -> dict[str, object]:
            calls.append((test, artifact))
            return basic_execution(test, artifact, "user exit status=0\n")

        case = stress_case()
        with mock.patch.object(runner, "_execute_basic", side_effect=execute):
            result = runner._run_case(case=case, repo_root=self.root, out_root=self.root / "out", runs=2, baseline=None)
        self.assertEqual(result["summary"]["totals"], {"success": 2, "failure": 0})
        self.assertEqual([test for test, _ in calls], ["user-smoke-native", "user-smoke-native"])
        self.assertTrue(str(calls[0][1]).endswith("runs/run-0001/basic"))
        self.assertTrue(str(calls[1][1]).endswith("runs/run-0002/basic"))

    def test_difftest_runs_left_then_right_and_propagates_basic_failure(self) -> None:
        calls: list[tuple[str, Path]] = []

        def execute(test: str, artifact: Path, repo_root: Path) -> dict[str, object]:
            calls.append((test, artifact))
            text = "checkpoint: A\ncheckpoint: B\n"
            return basic_execution(test, artifact, text, expectations=test != "left-basic")

        case = difftest_case()
        with mock.patch.object(runner, "_execute_basic", side_effect=execute):
            result = runner._execute_difftest_run(case, "run-0001", self.root / "run-0001", self.root)
        self.assertEqual([test for test, _ in calls], ["left-basic", "right-basic"])
        self.assertTrue(str(calls[0][1]).endswith("run-0001/left"))
        self.assertTrue(str(calls[1][1]).endswith("run-0001/right"))
        self.assertEqual(result["result"], "failure")
        self.assertFalse(result["left"]["gate_passed"])
        self.assertTrue(result["right"]["gate_passed"])
        self.assertTrue(result["paired_diff"]["passed"])

    def test_difftest_passes_only_after_both_basic_gates_and_diff(self) -> None:
        def execute(test: str, artifact: Path, repo_root: Path) -> dict[str, object]:
            return basic_execution(test, artifact, "checkpoint: A\ncheckpoint: B\n")

        with mock.patch.object(runner, "_execute_basic", side_effect=execute):
            result = runner._execute_difftest_run(difftest_case(), "run-0001", self.root / "run", self.root)
        self.assertEqual(result["result"], "success")
        self.assertTrue(result["paired_diff"]["passed"])

    def test_difftest_observable_sequence_reports_missing_and_command_echo(self) -> None:
        case = {
            **difftest_case(),
            "checkpoint_scope": [],
            "observable_scope": ["r1", "r2", "complete"],
            "observable_patterns": {
                "r1": r"(?m)^R1=#!/$",
                "r2": r"(?m)^R2=bin$",
                "complete": r"(?m)^lkm-fork-ofd-offset: complete$",
            },
        }

        def execute(test: str, artifact: Path, repo_root: Path) -> dict[str, object]:
            if test == "left-basic":
                text = (
                    "~ # printf R1=;dd bs=3 count=1 <&9 2>/dev/null;echo\n"
                    "R1=#!/\n"
                )
                return basic_execution(test, artifact, text, expectations=False)
            return basic_execution(
                test,
                artifact,
                "R1=#!/\nR2=bin\nlkm-fork-ofd-offset: complete\n",
            )

        with mock.patch.object(runner, "_execute_basic", side_effect=execute):
            result = runner._execute_difftest_run(case, "run-0001", self.root / "run", self.root)
        observable = result["paired_diff"]["observable"]
        self.assertFalse(result["paired_diff"]["passed"])
        self.assertEqual(observable["left_sequence"], ["r1"])
        self.assertEqual(observable["missing_from_left"], ["r2", "complete"])
        self.assertEqual(observable["first_divergence"], {"index": 1, "left": None, "right": "r2"})

    def test_difftest_runs_right_when_left_runner_does_not_produce_a_result(self) -> None:
        calls: list[str] = []

        def execute(test: str, artifact: Path, repo_root: Path) -> dict[str, object]:
            calls.append(test)
            if test == "left-basic":
                raise RuntimeError("missing left result")
            return basic_execution(test, artifact, "checkpoint: A\ncheckpoint: B\n")

        with mock.patch.object(runner, "_execute_basic", side_effect=execute):
            result = runner._execute_difftest_run(
                difftest_case(), "run-0001", self.root / "run", self.root
            )
        self.assertEqual(calls, ["left-basic", "right-basic"])
        self.assertEqual(result["result"], "failure")
        self.assertEqual(result["left"]["execution_status"], "failed")
        self.assertIn("missing left result", result["left"]["errors"])
        self.assertTrue(result["right"]["gate_passed"])

    def test_stress_basic_expectation_failure_overrides_success_classifier(self) -> None:
        case = stress_case()
        execution = basic_execution("user-smoke-native", self.root / "basic", "user exit status=0\n", expectations=False)
        with mock.patch.object(runner, "_execute_basic", return_value=execution):
            result = runner._execute_stress_run(case, "run-0001", self.root / "run", self.root)
        self.assertEqual(result["result"], "failure")
        self.assertEqual(result["class_id"], "nonzero-exit")


class EventAndDiffTests(unittest.TestCase):
    def test_kernel_online_follows_lower_enable_process_and_precedes_handoff(self) -> None:
        events = runner._extract_events(
            "RGTDOI\n"
            "checkpoint: Scheduler.Schedule task=BootTask\n"
            "checkpoint: PayloadHandoffPreparePhase.Online task=KernelInitTask\n"
            "checkpoint: Kernel.Online task=KernelInitTask\n"
            "checkpoint: KernelInitFlow.PayloadHandoffCommitted task=KernelInitTask\n"
            "checkpoint: UserAppRuntime.EnterUserMode task=KernelInitTask\n"
        )
        names = [event["name"] for event in events]
        ordered = [
            "Kernel.Started",
            "PhysicalDirect.ActivatedOnCpu",
            "BootInitFlow.Started",
            "Scheduler.Schedule",
            "PayloadHandoffPreparePhase.Online",
            "Kernel.Online",
            "KernelInitFlow.PayloadHandoffCommitted",
            "UserAppRuntime.EnterUserMode",
        ]
        self.assertEqual([name for name in names if name in ordered], ordered)
        self.assertEqual(names.count("Kernel.Online"), 1)
        self.assertEqual(names.count("BootTask.OnCpu"), 1)

    def test_complete_stress_record_is_decoded_but_partial_is_not(self) -> None:
        text = "checkpoint: SyscallTable.Wait4\n"
        record = stress_record(text)
        observed, metadata = runner._observed_text(record)
        self.assertEqual(observed, text)
        self.assertEqual(metadata["bytes"], len(text.encode()))
        partial = record[:-3]
        observed, metadata = runner._observed_text(partial)
        self.assertEqual(observed, partial)
        self.assertIsNone(metadata)

    def test_extracts_checkpoint_user_exit_and_smoke_events(self) -> None:
        events = runner._extract_events(
            "checkpoint: KernelInitFlow.PayloadHandoffCommitted\nuser exit status=0\nresult: ok. passed=2 failed=0 total=2\n"
        )
        self.assertEqual(
            [event["name"] for event in events],
            ["KernelInitFlow.PayloadHandoffCommitted", "UserExitStatus", "SmokeResult"],
        )

    def test_failure_classifier_precedes_success(self) -> None:
        rules = [
            {"id": "failure", "result": "failure", "contains": ["bad"], "regex": []},
            {"id": "success", "result": "success", "contains": ["ok"], "regex": []},
        ]
        self.assertEqual(runner._classify("ok bad", 0, False, rules)["id"], "failure")

    def test_classifier_timeout_predicate_uses_structured_basic_state(self) -> None:
        rules = [
            {
                "id": "timeout-shape",
                "result": "failure",
                "contains": ["terminating on signal 15"],
                "regex": [],
                "timed_out": True,
            },
            {
                "id": "marker-success",
                "result": "success",
                "contains": ["complete"],
                "regex": [],
            },
        ]
        marker_exit = "complete\nterminating on signal 15"
        self.assertEqual(
            runner._classify(marker_exit, 0, False, rules)["id"], "marker-success"
        )
        self.assertEqual(
            runner._classify(marker_exit, 1, True, rules)["id"], "timeout-shape"
        )

    def test_checkpoint_diff_reports_counts_and_first_divergence(self) -> None:
        left = runner._extract_events("checkpoint: A\ncheckpoint: A\ncheckpoint: B\n")
        right = runner._extract_events("checkpoint: A\ncheckpoint: B\n")
        limited = runner._paired_checkpoint_diff(
            left, right, ["A", "B"], checkpoint_scope_max_counts={"A": 1}
        )
        self.assertTrue(limited["passed"])
        self.assertEqual(limited["observed_but_not_compared"]["left"][0]["excluded_reason"], "scope_count_limit")
        divergent = runner._paired_checkpoint_diff(
            runner._extract_events("checkpoint: A\ncheckpoint: B\n"),
            runner._extract_events("checkpoint: B\ncheckpoint: A\n"),
            ["A", "B"],
        )
        self.assertFalse(divergent["passed"])
        self.assertEqual(divergent["first_divergence"], {"index": 0, "left": "A", "right": "B"})

    def test_duplicate_sequence_clusters_once(self) -> None:
        sequences: dict[tuple[str, str, str], dict[str, object]] = {}
        run = {
            "result": "success", "class_id": "ok", "sequence_hash": "abc",
            "sequence_tokens": ["checkpoint:A"], "run_id": "run-0001",
        }
        runner._record_sequence(sequences, run)
        runner._record_sequence(sequences, {**run, "run_id": "run-0002"})
        self.assertEqual(len(sequences), 1)
        self.assertEqual(next(iter(sequences.values()))["count"], 2)


class BaselineTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        (self.root / "runs/run-0001").mkdir(parents=True)
        (self.root / "sequences/success/ok").mkdir(parents=True)
        self.case = stress_case()
        manifest = {
            "schema_version": 2,
            "case": "demo",
            "mode": "stress",
            "test": "user-smoke-native",
            "config_fingerprint": "config",
            "classifier_sha256": "classifier",
        }
        summary = {
            "schema_version": 2,
            "case": "demo",
            "completed_runs": 1,
            "totals": {"success": 1, "failure": 0},
            "classes": [{"result": "success", "class_id": "ok"}],
            "sequences": [{"result": "success", "class_id": "ok", "sequence_hash": "old"}],
        }
        (self.root / "manifest.json").write_text(json.dumps(manifest))
        (self.root / "summary.json").write_text(json.dumps(summary))
        (self.root / "runs/run-0001/result.json").write_text(json.dumps({"sequence_tokens": ["A", "B"]}))
        (self.root / "sequences/success/ok/old.json").write_text("{}")

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def test_compatible_baseline_is_hashed_and_differences_are_nonblocking(self) -> None:
        runner._validate_baseline(self.root, self.case)
        frozen = runner._baseline_manifest(self.root)
        self.assertEqual(frozen["path"], str(self.root.resolve()))
        self.assertEqual(len(frozen["content_sha256"]), 64)
        self.assertIn("runs/run-0001/result.json", [item["path"] for item in frozen["files"]])
        current = {
            "schema_version": 2,
            "completed_runs": 1,
            "totals": {"success": 0, "failure": 1},
            "classes": [{"result": "failure", "class_id": "new"}],
            "sequences": [{"result": "failure", "class_id": "new", "sequence_hash": "new"}],
        }
        comparison = runner._compare_baseline(
            self.root, self.case, current, [{"sequence_tokens": ["A", "C"]}]
        )
        self.assertEqual(comparison["failure_rate_delta"], 1.0)
        self.assertFalse(comparison["affects_exit_status"])
        self.assertEqual(comparison["recent_sequence_first_divergence"]["index"], 1)

    def test_v1_or_identity_mismatch_baseline_is_rejected(self) -> None:
        manifest_path = self.root / "manifest.json"
        manifest = json.loads(manifest_path.read_text())
        manifest["schema_version"] = 1
        manifest_path.write_text(json.dumps(manifest))
        with self.assertRaisesRegex(runner.CompositeConfigError, "schema v2"):
            runner._validate_baseline(self.root, self.case)


class CoverageTests(unittest.TestCase):
    def test_coverage_requires_explicit_accounting(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            mapping = Path(temporary) / "mapping.json"
            mapping.write_text(json.dumps([
                {"checkpoint_name": "A", "mapping_kind": "exact"},
                {"checkpoint_name": "B", "mapping_kind": "exact"},
                {"checkpoint_name": "C", "mapping_kind": "range"},
            ]))
            with self.assertRaises(runner.CheckpointCoverageError):
                runner._audit_checkpoint_coverage(
                    mapping_path=mapping,
                    required_mapping_kinds=["exact"],
                    checkpoint_scope=["A"],
                    accounted_outside_scope={},
                    mode="explicit-accounting",
                )
            audit = runner._audit_checkpoint_coverage(
                mapping_path=mapping,
                required_mapping_kinds=["exact"],
                checkpoint_scope=["A"],
                accounted_outside_scope={"B": "outside hard scope"},
                mode="explicit-accounting",
            )
            self.assertEqual(audit["unaccounted"], 0)
            self.assertEqual(audit["required_total"], 2)


class SummaryTests(unittest.TestCase):
    def test_summary_reports_failure_success_analysis_and_zero_run_average(self) -> None:
        now = datetime.now(timezone.utc)
        empty = runner._build_summary("demo", 0, [], {}, dry_run=True, started=now, ended=now, duration_seconds=0)
        self.assertIsNone(empty["average_run_seconds"])
        self.assertEqual(empty["totals"], {"success": 0, "failure": 0})
        sequences = {
            ("success", "ok", "s"): {"tokens": ["A", "B"], "sequence_hash": "s", "count": 1, "first_run": "run-0001", "last_run": "run-0001"},
            ("failure", "bad", "f"): {"tokens": ["A", "C"], "sequence_hash": "f", "count": 1, "first_run": "run-0002", "last_run": "run-0002"},
        }
        runs = [
            {"result": "success", "class_id": "ok", "duration_seconds": 1.0},
            {"result": "failure", "class_id": "bad", "duration_seconds": 3.0},
        ]
        summary = runner._build_summary("demo", 2, runs, sequences, dry_run=False, started=now, ended=now, duration_seconds=4)
        self.assertEqual(summary["average_run_seconds"], 2.0)
        self.assertEqual(summary["failure_vs_success"][0]["common_prefix_length"], 1)


if __name__ == "__main__":
    unittest.main()
