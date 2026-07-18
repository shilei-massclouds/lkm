from __future__ import annotations

import io
import json
import os
from pathlib import Path
import pty
import stat
import tempfile
import textwrap
import threading
import time
import unittest
from unittest import mock

from . import rootfs_builder
from . import runner


class BasicRunnerConfigTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.cases = self.root / "cases"
        self.cases.mkdir()
        (self.root / "disk.raw").write_bytes(b"canonical")

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def write_case(self, body: str, name: str = "demo") -> Path:
        path = self.cases / f"{name}.toml"
        path.write_text(textwrap.dedent(body))
        return path

    def base_case(self, *, disk: str = 'mode = "external"\npath = "disk.raw"\nreadonly = false', extra: str = "") -> str:
        return f"""
            schema_version = 1
            name = "demo"
            timeout_seconds = 2
            {extra}

            [kernel]
            app = "hello"
            provider = "native"
            probe = []
            profile = "release"

            [disk]
            {disk}

            [qemu]
            memory_mb = 64
            smp = 2
            kernel_cmdline = "earlycon=sbi"
            rng = false
            exit_policy = "guest-shutdown"

            [expect]
            process_exit = 0
            guest_exit_status = 0
            markers = ["SUCCESS"]
        """

    def test_unknown_field_is_rejected(self) -> None:
        path = self.write_case(self.base_case(extra="mystery = 1"))
        with self.assertRaisesRegex(runner.ConfigError, "unknown top level"):
            runner.load_config(path, self.root)

    def test_illegal_disk_mode_is_rejected(self) -> None:
        path = self.write_case(self.base_case(disk='mode = "nearby"'))
        with self.assertRaisesRegex(runner.ConfigError, "disk.mode"):
            runner.load_config(path, self.root)

    def test_mode_specific_disk_fields_are_rejected(self) -> None:
        path = self.write_case(self.base_case(disk='mode = "none"\npath = "disk.raw"'))
        with self.assertRaisesRegex(runner.ConfigError, "disk mode none"):
            runner.load_config(path, self.root)

    def test_manifest_derives_structured_build_disk_and_qemu_arguments(self) -> None:
        path = self.write_case(self.base_case())
        config = runner.load_config(path, self.root)
        manifest = runner.freeze_manifest(config, path, self.root, self.root / "artifacts")
        self.assertIn("APP=hello", manifest["build_command"])
        self.assertIn("PLIC_PROVIDER=native", manifest["build_command"])
        self.assertIn("-drive", manifest["qemu"]["command"])
        self.assertNotIn("readonly=on", " ".join(manifest["qemu"]["command"]))
        self.assertEqual(manifest["disk"]["mode"], "external")

    def test_v1_template_modes_and_interaction_have_explicit_compatibility_mappings(self) -> None:
        path = self.write_case(self.base_case(disk='mode = "canonical-readonly"'))
        config = runner.load_config(path, self.root)
        self.assertEqual(config["schema_version"], 2)
        self.assertEqual(config["source_schema_version"], 1)
        self.assertEqual(config["disk"]["mode"], "template-readonly")
        self.assertEqual(config["disk"]["profile"], "canonical")
        self.assertEqual(config["qemu"]["interaction"], "none")
        self.assertTrue(config["compatibility_mappings"])

    def test_v2_requires_purpose_template_profile_and_interaction(self) -> None:
        body = self.base_case(disk='mode = "private-copy"').replace("schema_version = 1", "schema_version = 2")
        path = self.write_case(body)
        with self.assertRaisesRegex(runner.ConfigError, "purpose"):
            runner.load_config(path, self.root)

        body = body.replace('name = "demo"', 'name = "demo"\npurpose = "acceptance"')
        path.write_text(textwrap.dedent(body))
        with self.assertRaisesRegex(runner.ConfigError, "profile"):
            runner.load_config(path, self.root)

        body = body.replace('mode = "private-copy"', 'mode = "private-copy"\nprofile = "canonical"')
        path.write_text(textwrap.dedent(body))
        with self.assertRaisesRegex(runner.ConfigError, "interaction"):
            runner.load_config(path, self.root)

    def test_v2_interaction_step_rules_are_mutually_exclusive(self) -> None:
        base = self.base_case().replace("schema_version = 1", "schema_version = 2")
        base = base.replace('name = "demo"', 'name = "demo"\npurpose = "acceptance"')
        base = base.replace('exit_policy = "guest-shutdown"', 'exit_policy = "guest-shutdown"\ninteraction = "scripted"')
        path = self.write_case(base)
        with self.assertRaisesRegex(runner.ConfigError, "requires stdin_steps"):
            runner.load_config(path, self.root)

        path.write_text(textwrap.dedent(base.replace('interaction = "scripted"', 'interaction = "terminal"\nstdin_steps = [{ ready_marker = "x", payload = "y" }]')))
        with self.assertRaisesRegex(runner.ConfigError, "forbids stdin_steps"):
            runner.load_config(path, self.root)

    def test_all_checked_in_cases_are_schema_v2_and_aliases_have_no_duplicate_toml(self) -> None:
        repo_root = Path(__file__).resolve().parents[4]
        cases = repo_root / "impl" / "arceos_ex" / "tests" / "basic" / "cases"
        loaded = [runner.load_config(path, repo_root) for path in sorted(cases.glob("*.toml"))]
        self.assertTrue(loaded)
        self.assertTrue(all(config["source_schema_version"] == 2 for config in loaded))
        self.assertFalse((cases / "kunit-native.toml").exists())
        self.assertEqual(runner.COMPATIBILITY_ALIASES["kunit-native"], "checkpoint-kunit-native")

    def test_v2_rejects_unknown_template_profile(self) -> None:
        body = self.base_case(disk='mode = "private-copy"\nprofile = "nearby"')
        body = body.replace("schema_version = 1", "schema_version = 2")
        body = body.replace('name = "demo"', 'name = "demo"\npurpose = "acceptance"')
        body = body.replace('exit_policy = "guest-shutdown"', 'exit_policy = "guest-shutdown"\ninteraction = "none"')
        path = self.write_case(body)
        with self.assertRaisesRegex(runner.ConfigError, "disk.profile"):
            runner.load_config(path, self.root)


class BasicRunnerLifecycleTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.kernel_dir = self.root / "impl" / "arceos_ex"
        self.kernel_dir.mkdir(parents=True)
        self.cases = self.root / "cases"
        self.cases.mkdir()
        (self.root / "disk.raw").write_bytes(b"external")
        (self.root / "canonical.raw").write_bytes(b"canonical")
        self.fake_make = self.root / "fake-make.py"
        self.fake_qemu = self.root / "fake-qemu.py"
        self._write_executable(
            self.fake_make,
            """
            #!/usr/bin/env python3
            import pathlib
            import sys

            args = sys.argv[1:]
            kernel = pathlib.Path(args[args.index("-C") + 1])
            if "disk" in args:
                image_arg = next(arg for arg in args if arg.startswith("CANONICAL_ROOTFS_IMAGE="))
                image = pathlib.Path(image_arg.split("=", 1)[1])
                image.parent.mkdir(parents=True, exist_ok=True)
                image.write_bytes(b"canonical")
            if "build" in args:
                app = next(arg.split("=", 1)[1] for arg in args if arg.startswith("APP="))
                provider = next(arg.split("=", 1)[1] for arg in args if arg.startswith("PLIC_PROVIDER="))
                profile = next(arg.split("=", 1)[1] for arg in args if arg.startswith("PROFILE="))
                suffix = "" if provider == "native" else f"/plic-{provider}"
                image = kernel / f"build/riscv64imac-unknown-none-elf/{profile}/{app}{suffix}/arceos_ex.bin"
                image.parent.mkdir(parents=True, exist_ok=True)
                image.write_bytes(b"kernel")
            """,
        )
        self._write_executable(
            self.fake_qemu,
            """
            #!/usr/bin/env python3
            import os
            import pathlib
            import sys
            import time

            if path := os.environ.get("FAKE_QEMU_RAN"):
                pathlib.Path(path).write_text("ran")
            if path := os.environ.get("FAKE_QEMU_PID"):
                pathlib.Path(path).write_text(str(os.getpid()))
            mode = os.environ.get("FAKE_QEMU_MODE", "success")
            if mode == "timeout":
                time.sleep(30)
            elif mode == "marker":
                print("STOP", flush=True)
                time.sleep(30)
            else:
                if os.environ.get("FAKE_QEMU_STDIN"):
                    print("READY", flush=True)
                    sys.stdin.readline()
                print("SUCCESS", flush=True)
                print("user exit status=0", flush=True)
            """,
        )
        self.environment = {
            "MAKE": str(self.fake_make),
            "QEMU": str(self.fake_qemu),
            "CANONICAL_ROOTFS_IMAGE": str(self.root / "canonical.raw"),
        }

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def _write_executable(self, path: Path, body: str) -> None:
        path.write_text(textwrap.dedent(body).lstrip())
        path.chmod(path.stat().st_mode | stat.S_IXUSR)

    def write_case(
        self,
        *,
        name: str = "demo",
        timeout: int = 2,
        disk: str = 'mode = "external"\npath = "disk.raw"',
        top: str = "",
        qemu: str = 'exit_policy = "guest-shutdown"',
        stdin: str = "",
        expect: str = 'process_exit = 0\nguest_exit_status = 0\nmarkers = ["SUCCESS"]',
    ) -> Path:
        path = self.cases / f"{name}.toml"
        path.write_text(
            textwrap.dedent(
                f"""
                schema_version = 1
                name = "{name}"
                timeout_seconds = {timeout}
                {top}

                [kernel]
                app = "hello"
                provider = "native"
                probe = []
                profile = "release"

                [disk]
                {disk}

                [qemu]
                memory_mb = 64
                smp = 2
                kernel_cmdline = "earlycon=sbi"
                rng = false
                {qemu}
                {stdin}

                [expect]
                {expect}
                """
            )
        )
        return path

    def run_case(self, path: Path, output_name: str) -> tuple[int, Path, dict]:
        output = self.root / output_name
        with mock.patch.dict(os.environ, self.environment, clear=False), mock.patch.object(
            runner,
            "validate_template",
            return_value=(True, "template and input manifest are current"),
        ):
            status = runner.run_pipeline(
                command="run",
                config_path=path,
                repo_root=self.root,
                out_root=self.root / "out",
                output_dir=output,
            )
        result = json.loads((output / "result.json").read_text())
        return status, output, result

    def test_success_writes_structured_result_and_qemu_log(self) -> None:
        status, output, result = self.run_case(self.write_case(), "success")
        self.assertEqual(status, 0)
        self.assertEqual(result["schema_version"], 2)
        self.assertEqual(result["execution_status"], "completed")
        self.assertEqual(result["verdict"], "passed")
        self.assertEqual(result["qemu"]["exit_code"], 0)
        self.assertTrue(result["expectations"]["passed"])
        self.assertIn("SUCCESS", (output / "qemu.log").read_text())
        self.assertEqual(set(result["stages"]), set(runner.STAGES))

    def test_build_only_does_not_prepare_disk_or_start_qemu(self) -> None:
        path = self.write_case()
        ran = self.root / "qemu-ran"
        output = self.root / "build-only"
        environment = {**self.environment, "FAKE_QEMU_RAN": str(ran)}
        with mock.patch.dict(os.environ, environment, clear=False):
            status = runner.run_pipeline(
                command="build",
                config_path=path,
                repo_root=self.root,
                out_root=self.root / "out",
                output_dir=output,
            )
        result = json.loads((output / "result.json").read_text())
        self.assertEqual(status, 0)
        self.assertFalse(ran.exists())
        self.assertEqual(result["stages"]["disk-prepare"]["status"], "skipped")
        self.assertEqual(result["stages"]["qemu"]["status"], "skipped")
        self.assertEqual(result["execution_status"], "completed")
        self.assertEqual(result["verdict"], "inconclusive")

    def test_no_disk_mode_does_not_add_drive(self) -> None:
        path = self.write_case(disk='mode = "none"')
        status, output, _ = self.run_case(path, "no-disk")
        self.assertEqual(status, 0)
        manifest = json.loads((output / "manifest.json").read_text())
        self.assertNotIn("-drive", manifest["qemu"]["command"])
        self.assertIn("disk mode: none", (output / "disk-prepare.log").read_text())

    def test_config_failure_still_writes_result_and_empty_qemu_log(self) -> None:
        path = self.cases / "invalid.toml"
        path.write_text('schema_version = 1\nname = "invalid"\nunknown = true\n')
        output = self.root / "config-failure"
        with mock.patch.dict(os.environ, self.environment, clear=False):
            status = runner.run_pipeline(
                command="run",
                config_path=path,
                repo_root=self.root,
                out_root=self.root / "out",
                output_dir=output,
            )
        result = json.loads((output / "result.json").read_text())
        self.assertEqual(status, 1)
        self.assertEqual(result["stages"]["config"]["status"], "failed")
        self.assertEqual(result["execution_status"], "failed")
        self.assertEqual(result["verdict"], "inconclusive")
        self.assertEqual((output / "qemu.log").read_bytes(), b"")

    def test_invalid_requested_name_writes_early_config_failure_result(self) -> None:
        output = self.root / "early-config-failure"
        status = runner.main(
            [
                "run",
                "invalid/name",
                "--repo-root",
                str(self.root),
                "--cases-dir",
                str(self.cases),
                "--output-dir",
                str(output),
            ]
        )

        result = json.loads((output / "result.json").read_text())
        self.assertEqual(status, 1)
        self.assertEqual(result["request"]["test"], "invalid/name")
        self.assertEqual(result["stages"]["config"]["status"], "failed")
        self.assertEqual(result["execution_status"], "failed")
        self.assertEqual((output / "qemu.log").read_bytes(), b"")

    def test_compatibility_alias_is_recorded_without_duplicate_config(self) -> None:
        path = self.write_case()
        output = self.root / "compatibility-alias"
        with mock.patch.dict(os.environ, self.environment, clear=False):
            status = runner.run_pipeline(
                command="build",
                config_path=path,
                repo_root=self.root,
                out_root=self.root / "out",
                output_dir=output,
                requested_test="kunit-native",
                compatibility_alias="kunit-native",
            )

        result = json.loads((output / "result.json").read_text())
        self.assertEqual(status, 0)
        self.assertEqual(result["request"]["test"], "kunit-native")
        self.assertEqual(result["request"]["canonical_test"], "demo")
        self.assertEqual(result["request"]["compatibility_alias"], "kunit-native")

    def test_scripts_receive_fixed_environment_and_post_runs_after_success(self) -> None:
        pre = self.cases / "pre.sh"
        post = self.cases / "post.sh"
        self._write_executable(
            pre,
            """
            #!/bin/sh
            test -z "${APP+x}"
            test -n "$LKM_TEST_KERNEL"
            printf pre > "$LKM_TEST_ARTIFACTS/pre-ran"
            """,
        )
        self._write_executable(post, '#!/bin/sh\nprintf post > "$LKM_TEST_ARTIFACTS/post-ran"\n')
        path = self.write_case(top='pre_script = "pre.sh"\npost_script = "post.sh"')
        status, output, _ = self.run_case(path, "scripts")
        self.assertEqual(status, 0)
        self.assertEqual((output / "pre-ran").read_text(), "pre")
        self.assertEqual((output / "post-ran").read_text(), "post")

    def test_pre_script_failure_prevents_qemu_and_post(self) -> None:
        pre = self.cases / "pre-fail.sh"
        post = self.cases / "post.sh"
        self._write_executable(pre, "#!/bin/sh\nexit 7\n")
        self._write_executable(post, '#!/bin/sh\nprintf post > "$LKM_TEST_ARTIFACTS/post-ran"\n')
        ran = self.root / "qemu-ran"
        self.environment["FAKE_QEMU_RAN"] = str(ran)
        path = self.write_case(top='pre_script = "pre-fail.sh"\npost_script = "post.sh"')
        status, output, result = self.run_case(path, "pre-failure")
        self.assertEqual(status, 1)
        self.assertFalse(ran.exists())
        self.assertFalse((output / "post-ran").exists())
        self.assertEqual(result["stages"]["qemu"]["status"], "skipped")

    def test_post_script_failure_sets_unified_nonzero_exit(self) -> None:
        post = self.cases / "post-fail.sh"
        self._write_executable(post, "#!/bin/sh\nexit 9\n")
        path = self.write_case(top='post_script = "post-fail.sh"')
        status, _, result = self.run_case(path, "post-failure")
        self.assertEqual(status, 1)
        self.assertEqual(result["stages"]["qemu"]["status"], "success")
        self.assertEqual(result["stages"]["post-script"]["status"], "failed")

    def test_timeout_reaps_qemu_and_still_runs_post(self) -> None:
        post = self.cases / "post.sh"
        self._write_executable(post, '#!/bin/sh\nprintf post > "$LKM_TEST_ARTIFACTS/post-ran"\n')
        pid_file = self.root / "qemu-pid"
        self.environment.update(FAKE_QEMU_MODE="timeout", FAKE_QEMU_PID=str(pid_file))
        path = self.write_case(timeout=1, top='post_script = "post.sh"')
        status, output, result = self.run_case(path, "timeout")
        self.assertEqual(status, 1)
        self.assertEqual(result["stages"]["qemu"]["status"], "timed_out")
        self.assertTrue(result["qemu"]["timed_out"])
        self.assertTrue(result["cleanup"]["process_group_reaped"])
        self.assertTrue((output / "post-ran").exists())
        pid = int(pid_file.read_text())
        with self.assertRaises(ProcessLookupError):
            os.kill(pid, 0)

    def test_marker_exit_policy_terminates_process_group(self) -> None:
        self.environment["FAKE_QEMU_MODE"] = "marker"
        path = self.write_case(
            qemu='exit_policy = "marker"\nexit_marker = "STOP"',
            expect='markers = ["STOP"]',
        )
        status, _, result = self.run_case(path, "marker")
        self.assertEqual(status, 0)
        self.assertTrue(result["qemu"]["terminated_after_marker"])

    def test_stdin_step_is_sent_after_ready_marker(self) -> None:
        self.environment["FAKE_QEMU_STDIN"] = "1"
        path = self.write_case(stdin='stdin_steps = [{ ready_marker = "READY", payload = "go\\n" }]')
        status, _, result = self.run_case(path, "stdin")
        self.assertEqual(status, 0)
        self.assertTrue(result["qemu"]["stdin_steps"][0]["sent"])

    def test_private_copy_is_removed_and_template_is_unchanged(self) -> None:
        path = self.write_case(disk='mode = "private-copy"')
        status, output, result = self.run_case(path, "private")
        self.assertEqual(status, 0)
        self.assertTrue(result["cleanup"]["private_disk_removed"])
        self.assertFalse((output / "private-disk.raw").exists())
        self.assertEqual((self.root / "canonical.raw").read_bytes(), b"canonical")

    def test_expectation_failure_sets_unified_nonzero_exit(self) -> None:
        post = self.cases / "post.sh"
        self._write_executable(post, '#!/bin/sh\nprintf post > "$LKM_TEST_ARTIFACTS/post-ran"\n')
        path = self.write_case(top='post_script = "post.sh"', expect='process_exit = 0\nmarkers = ["MISSING"]')
        status, output, result = self.run_case(path, "expect-failure")
        self.assertEqual(status, 1)
        self.assertEqual(result["execution_status"], "completed")
        self.assertEqual(result["verdict"], "failed")
        self.assertFalse(result["expectations"]["passed"])
        self.assertTrue((output / "post-ran").exists())

    def test_missing_or_stale_template_fails_without_invoking_builder(self) -> None:
        path = self.write_case(disk='mode = "private-copy"')
        output = self.root / "stale-template"
        with mock.patch.dict(os.environ, self.environment, clear=False), mock.patch.object(
            runner,
            "validate_template",
            return_value=(False, "template inputs changed after construction"),
        ):
            status = runner.run_pipeline(
                command="run",
                config_path=path,
                repo_root=self.root,
                out_root=self.root / "out",
                output_dir=output,
            )
        self.assertEqual(status, 1)
        self.assertIn("make disk ROOTFS=canonical", (output / "result.json").read_text())

    def test_diagnostic_normal_completion_is_inconclusive(self) -> None:
        path = self.write_case()
        body = path.read_text().replace("schema_version = 1", "schema_version = 2")
        body = body.replace('name = "demo"', 'name = "demo"\npurpose = "diagnostic"')
        body = body.replace('exit_policy = "guest-shutdown"', 'exit_policy = "guest-shutdown"\ninteraction = "none"')
        path.write_text(body)
        status, _, result = self.run_case(path, "diagnostic")
        self.assertEqual(status, 0)
        self.assertEqual(result["execution_status"], "completed")
        self.assertEqual(result["verdict"], "inconclusive")

    def test_terminal_without_tty_fails_before_kernel_build(self) -> None:
        path = self.write_case()
        body = path.read_text().replace("schema_version = 1", "schema_version = 2")
        body = body.replace('name = "demo"', 'name = "demo"\npurpose = "diagnostic"')
        body = body.replace('exit_policy = "guest-shutdown"', 'exit_policy = "guest-shutdown"\ninteraction = "terminal"')
        path.write_text(body)
        output = self.root / "terminal-no-tty"
        with mock.patch.dict(os.environ, self.environment, clear=False), mock.patch.object(
            runner,
            "_terminal_available",
            return_value=False,
        ):
            status = runner.run_pipeline(
                command="run",
                config_path=path,
                repo_root=self.root,
                out_root=self.root / "out",
                output_dir=output,
            )
        result = json.loads((output / "result.json").read_text())
        self.assertEqual(status, 1)
        self.assertEqual(result["stages"]["kernel-build"]["status"], "skipped")
        self.assertIn("real stdin/stdout TTY", result["errors"][0])

    def test_terminal_pty_forwards_input_logs_output_and_restores_attributes(self) -> None:
        input_master, input_slave = pty.openpty()

        class FdInput:
            def fileno(self) -> int:
                return input_slave

            def isatty(self) -> bool:
                return True

        class CapturedOutput:
            def __init__(self) -> None:
                self.buffer = io.BytesIO()

            def flush(self) -> None:
                pass

            def isatty(self) -> bool:
                return True

        output = CapturedOutput()
        before = runner.termios.tcgetattr(input_slave)
        log_path = self.root / "terminal-qemu.log"
        manifest = {
            "timeout_seconds": 3,
            "qemu": {
                "interaction": "terminal",
                "command": [str(self.fake_qemu)],
                "exit_policy": "guest-shutdown",
                "exit_marker": None,
            },
        }
        outcome = runner.QemuOutcome(stdin_steps=[])

        def send_input() -> None:
            time.sleep(0.2)
            os.write(input_master, b"go\n")

        sender = threading.Thread(target=send_input)
        sender.start()
        environment = {**self.environment, "FAKE_QEMU_STDIN": "1"}
        try:
            with mock.patch.dict(os.environ, environment, clear=False), mock.patch.object(
                runner.sys,
                "stdin",
                FdInput(),
            ), mock.patch.object(runner.sys, "stdout", output):
                runner._run_qemu_terminal(manifest, self.root, log_path, outcome)
        finally:
            sender.join()
            after = runner.termios.tcgetattr(input_slave)
            os.close(input_master)
            os.close(input_slave)

        self.assertEqual(before, after)
        self.assertTrue(outcome.terminal_restored)
        self.assertTrue(outcome.process_group_reaped)
        self.assertIn("SUCCESS", log_path.read_text())
        self.assertIn(b"SUCCESS", output.buffer.getvalue())


class CanonicalRootfsTests(unittest.TestCase):
    def test_parse_size(self) -> None:
        self.assertEqual(rootfs_builder.parse_size("64M"), 64 * 1024 * 1024)
        with self.assertRaises(ValueError):
            rootfs_builder.parse_size("0M")

    def test_tool_signature_includes_executable_content_hash(self) -> None:
        signature = rootfs_builder.command_signature("/bin/true")
        self.assertEqual(signature["executable"], "/bin/true")
        self.assertRegex(signature["file"]["sha256"], r"^[0-9a-f]{64}$")

    def test_tree_signature_changes_with_input(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source"
            source.mkdir()
            item = source / "item"
            item.write_text("one")
            first = rootfs_builder.tree_signature(source, content=True)
            time.sleep(0.001)
            item.write_text("two")
            second = rootfs_builder.tree_signature(source, content=True)
            self.assertNotEqual(first["sha256"], second["sha256"])

    def test_rebuild_policy_covers_reuse_change_and_force(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            image = root / "canonical.raw"
            metadata = root / "canonical.raw.inputs.json"
            image.write_bytes(b"image")
            metadata.write_text(json.dumps({"fingerprint": "same"}))
            self.assertFalse(rootfs_builder.should_rebuild(image, metadata, "same", False))
            self.assertTrue(rootfs_builder.should_rebuild(image, metadata, "changed", False))
            self.assertTrue(rootfs_builder.should_rebuild(image, metadata, "same", True))

    def test_canonical_configuration_preserves_inittab_locks_root_and_merges_test(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            staging = root / "staging"
            config = root / "config"
            destination = staging / "opt" / "lkm" / "tests"
            (staging / "etc").mkdir(parents=True)
            destination.mkdir(parents=True)
            config.mkdir()
            (staging / "etc" / "inittab").write_text("::sysinit:/sbin/openrc sysinit\n")
            (staging / "etc" / "passwd").write_text("root:x:0:0:root:/root:/bin/sh\n")
            (staging / "etc" / "shadow").write_text("root:*::0:::::\n")
            (config / "passwd.entry").write_text("test:hash:1000:100:test:/:/bin/sh\n")
            (config / "shadow.entry").write_text("test:hash:0:::::\n")
            (config / "rc-local.sh").write_text("#!/bin/sh\nexit 0\n")

            rootfs_builder._configure_canonical(staging, config, destination)

            self.assertEqual((staging / "etc" / "inittab").read_text(), "::sysinit:/sbin/openrc sysinit\n")
            self.assertIn("root:*:", (staging / "etc" / "shadow").read_text())
            self.assertEqual(sum(line.startswith("test:") for line in (staging / "etc" / "passwd").read_text().splitlines()), 1)
            self.assertEqual(sum(line.startswith("test:") for line in (staging / "etc" / "shadow").read_text().splitlines()), 1)
            self.assertTrue(os.access(destination / "rc-local.sh", os.X_OK))

    def test_template_validation_detects_current_and_changed_inputs(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            tarball = root / "alpine.tar.gz"
            tarball.write_bytes(b"tarball")
            ltp = root / "ltp-root" / "opt" / "ltp"
            ltp.mkdir(parents=True)
            runner_script = ltp / "run-syscalls.sh"
            runner_script.write_text("#!/bin/sh\n")
            runner_script.chmod(0o755)
            fixtures = root / "fixtures"
            fixtures.mkdir()
            fixture = fixtures / "fixture.c"
            fixture.write_text("one")
            config = root / "config"
            config.mkdir()
            for name in ("passwd.entry", "shadow.entry", "rc-local.sh"):
                (config / name).write_text(name)
            image = root / "canonical.raw"
            image.write_bytes(b"image")
            metadata = Path(f"{image}.inputs.json")
            manifest = rootfs_builder.input_manifest(
                repo_root=root,
                tarball=tarball,
                ltp_dir=root / "ltp-root",
                user_dir=fixtures,
                config_dir=config,
                size="1M",
                mkfs_feature_args="",
                ext2_block_size="",
                gnu_cc="/bin/true",
                musl_cc="/bin/true",
            )
            manifest["fingerprint"] = rootfs_builder.manifest_fingerprint(manifest)
            metadata.write_text(json.dumps(manifest))

            valid, _ = rootfs_builder.validate_template(image, metadata, root)
            self.assertTrue(valid)
            fixture.write_text("two")
            valid, reason = rootfs_builder.validate_template(image, metadata, root)
            self.assertFalse(valid)
            self.assertIn("inputs changed", reason)

    def test_failed_pair_publication_restores_previous_template(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            image = root / "canonical.raw"
            metadata = root / "canonical.raw.inputs.json"
            image_tmp = root / ".canonical.raw.tmp"
            metadata_tmp = root / ".canonical.raw.inputs.json.tmp"
            image.write_bytes(b"old-image")
            metadata.write_bytes(b"old-metadata")
            image_tmp.write_bytes(b"new-image")
            metadata_tmp.write_bytes(b"new-metadata")
            original_replace = Path.replace

            def replace_with_failure(path: Path, target: Path) -> Path:
                if path == metadata_tmp:
                    raise OSError("publish failure")
                return original_replace(path, target)

            with mock.patch.object(Path, "replace", replace_with_failure):
                with self.assertRaisesRegex(OSError, "publish failure"):
                    rootfs_builder._publish_pair(image_tmp, metadata_tmp, image, metadata)

            self.assertEqual(image.read_bytes(), b"old-image")
            self.assertEqual(metadata.read_bytes(), b"old-metadata")


if __name__ == "__main__":
    unittest.main()
