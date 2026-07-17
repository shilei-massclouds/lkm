from __future__ import annotations

import json
import os
from pathlib import Path
import stat
import tempfile
import textwrap
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


class BasicRunnerLifecycleTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.kernel_dir = self.root / "impl" / "arceos_ex"
        self.kernel_dir.mkdir(parents=True)
        self.cases = self.root / "cases"
        self.cases.mkdir()
        (self.root / "disk.raw").write_bytes(b"external")
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
        with mock.patch.dict(os.environ, self.environment, clear=False):
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
        self.assertEqual(result["status"], "success")
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
        self.assertEqual((output / "qemu.log").read_bytes(), b"")

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
        self.assertEqual(result["status"], "failure")
        self.assertFalse(result["expectations"]["passed"])
        self.assertTrue((output / "post-ran").exists())


class CanonicalRootfsTests(unittest.TestCase):
    def test_parse_size(self) -> None:
        self.assertEqual(rootfs_builder.parse_size("64M"), 64 * 1024 * 1024)
        with self.assertRaises(ValueError):
            rootfs_builder.parse_size("0M")

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


if __name__ == "__main__":
    unittest.main()
