from __future__ import annotations

import io
import json
import os
from pathlib import Path
import pty
import socket
import stat
import subprocess
import tempfile
import textwrap
import threading
import time
import tomllib
import unittest
from unittest import mock

from . import rootfs_builder
from . import runner


class BasicMakeSelectionTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.capture = self.root / "selection.txt"
        self.fake_python = self.root / "fake-python"
        self.fake_python.write_text(
            "#!/bin/sh\n"
            'printf "%s\\n" "$@" > "$BASIC_TEST_SELECTION_CAPTURE"\n'
        )
        self.fake_python.chmod(self.fake_python.stat().st_mode | stat.S_IXUSR)
        self.repo_root = Path(__file__).resolve().parents[4]

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def run_make(
        self,
        *arguments: str,
        environment: dict[str, str] | None = None,
    ) -> subprocess.CompletedProcess[str]:
        env = os.environ.copy()
        for name in ("APP", "TEST", "MAKEFLAGS", "MFLAGS", "MAKEOVERRIDES"):
            env.pop(name, None)
        env["BASIC_TEST_SELECTION_CAPTURE"] = str(self.capture)
        env.update(environment or {})
        return subprocess.run(
            [
                "make",
                "--no-print-directory",
                "build",
                f"PYTHON={self.fake_python}",
                *arguments,
            ],
            cwd=self.repo_root,
            env=env,
            check=False,
            capture_output=True,
            text=True,
        )

    def captured_request(self) -> str:
        arguments = self.capture.read_text().splitlines()
        self.assertEqual(arguments[1], "build")
        return arguments[2]

    def test_app_make_argument_aliases_full_test_namespace(self) -> None:
        for test_name in ("shell", "shell-lo"):
            for variable in ("APP", "TEST"):
                selector = f"{variable}={test_name}"
                with self.subTest(selector=selector):
                    self.capture.unlink(missing_ok=True)
                    completed = self.run_make(selector)
                    self.assertEqual(completed.returncode, 0, completed.stderr)
                    self.assertEqual(self.captured_request(), test_name)

    def test_app_process_environment_aliases_test(self) -> None:
        completed = self.run_make(environment={"APP": "busybox-init-login-native"})
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertEqual(self.captured_request(), "busybox-init-login-native")

    def test_test_process_environment_is_formal_selector(self) -> None:
        completed = self.run_make(environment={"TEST": "busybox-init-login-native"})
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertEqual(self.captured_request(), "busybox-init-login-native")

    def test_explicit_test_and_app_from_mixed_sources_are_rejected(self) -> None:
        completed = self.run_make(
            "TEST=user-smoke-native",
            environment={"APP": "busybox-init-login-native"},
        )
        self.assertEqual(completed.returncode, 2)
        self.assertIn("selection is ambiguous", completed.stderr)
        self.assertFalse(self.capture.exists())


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

    def test_stress_mem_capacity_is_validated_before_build(self) -> None:
        body = self.base_case().replace(
            'profile = "release"',
            'profile = "release"\nstress_mem_bytes = 524288',
        )
        path = self.write_case(body)
        self.assertEqual(runner.load_config(path, self.root)["kernel"]["stress_mem_bytes"], 524288)
        path.write_text(body.replace("524288", "524289"))
        with self.assertRaisesRegex(runner.ConfigError, "stress_mem_bytes must be one of"):
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
        self.assertEqual(runner.COMPATIBILITY_ALIASES["hello"], "hello-native")
        self.assertEqual(runner.COMPATIBILITY_ALIASES["smoke"], "kernel-smoke-native")
        self.assertNotIn("user-boot", runner.COMPATIBILITY_ALIASES)
        self.assertIn("user-boot", runner.RETIRED_TEST_NAMES)
        for alias in runner.COMPATIBILITY_ALIASES:
            self.assertFalse((cases / f"{alias}.toml").exists())
        self.assertFalse((cases / "user-boot.toml").exists())
        for removed_name in ("shell-native", "shell-linux-object"):
            self.assertFalse((cases / f"{removed_name}.toml").exists())
            self.assertNotIn(removed_name, runner.COMPATIBILITY_ALIASES)
            self.assertNotIn(removed_name, runner.RETIRED_TEST_NAMES)
        for removed_name in (
            "ltp-shell-manual-native",
            "ltp-shell-manual-linux-object",
        ):
            self.assertFalse((cases / f"{removed_name}.toml").exists())
            self.assertNotIn(removed_name, runner.COMPATIBILITY_ALIASES)
            self.assertNotIn(removed_name, runner.RETIRED_TEST_NAMES)
        for removed_name in (
            "distro-sh-native",
            "distro-sh-linux-object",
            "script-shell",
            "script-shell-lo",
        ):
            self.assertFalse((cases / f"{removed_name}.toml").exists())
            self.assertNotIn(removed_name, runner.COMPATIBILITY_ALIASES)
            self.assertNotIn(removed_name, runner.RETIRED_TEST_NAMES)

    def test_terminal_shell_cases_have_scripted_dual_provider_counterparts(self) -> None:
        repo_root = Path(__file__).resolve().parents[4]
        cases = repo_root / "impl" / "arceos_ex" / "tests" / "basic" / "cases"
        loaded = {
            name: runner.load_config(cases / f"{name}.toml", repo_root)
            for name in (
                "shell",
                "shell-lo",
                "scripted-shell",
                "scripted-shell-lo",
            )
        }

        counterpart_names = {
            "native": ("shell", "scripted-shell"),
            "linux-object": ("shell-lo", "scripted-shell-lo"),
        }
        shared_qemu_fields = (
            "memory_mb",
            "smp",
            "kernel_cmdline",
            "rng",
            "exit_policy",
        )
        for provider, (terminal_name, scripted_name) in counterpart_names.items():
            with self.subTest(provider=provider):
                terminal = loaded[terminal_name]
                scripted = loaded[scripted_name]

                self.assertEqual(terminal["kernel"], scripted["kernel"])
                self.assertEqual(terminal["kernel"]["app"], "user-boot")
                self.assertEqual(terminal["kernel"]["provider"], provider)
                self.assertEqual(terminal["kernel"]["profile"], "release")
                self.assertEqual(terminal["disk"]["profile"], "canonical")
                self.assertEqual(scripted["disk"]["profile"], "canonical")
                self.assertEqual(
                    {field: terminal["qemu"][field] for field in shared_qemu_fields},
                    {field: scripted["qemu"][field] for field in shared_qemu_fields},
                )
                self.assertEqual(terminal["qemu"]["memory_mb"], 128)
                self.assertEqual(terminal["qemu"]["smp"], 8)
                self.assertEqual(
                    terminal["qemu"]["kernel_cmdline"],
                    "earlycon=sbi init=/bin/sh",
                )
                self.assertTrue(terminal["qemu"]["rng"])
                self.assertEqual(terminal["qemu"]["exit_policy"], "guest-shutdown")

                self.assertEqual(terminal["purpose"], "diagnostic")
                self.assertEqual(terminal["timeout_seconds"], 3600)
                self.assertEqual(
                    terminal["disk"],
                    {"mode": "private-copy", "profile": "canonical"},
                )
                self.assertEqual(terminal["qemu"]["interaction"], "terminal")
                self.assertEqual(terminal["qemu"]["stdin_steps"], [])
                self.assertIn(
                    "arceos_ex panic", terminal["expect"]["forbidden_markers"]
                )

                self.assertEqual(scripted["purpose"], "acceptance")
                self.assertEqual(scripted["timeout_seconds"], 120)
                self.assertEqual(
                    scripted["disk"],
                    {"mode": "template-readonly", "profile": "canonical"},
                )
                self.assertEqual(scripted["qemu"]["interaction"], "scripted")
                self.assertEqual(
                    scripted["qemu"]["stdin_steps"],
                    [
                        {
                            "ready_marker": "~ #",
                            "payload": 'echo "OK"\nls\nls /lib\nls /\nexit\n',
                        }
                    ],
                )
                self.assertEqual(scripted["expect"]["guest_exit_status"], 0)
                self.assertEqual(
                    scripted["expect"]["marker_counts"],
                    [
                        {"marker": "\nOK\n", "at_least": None, "exactly": 1},
                        {
                            "marker": "ld-musl-riscv64.so.1",
                            "at_least": None,
                            "exactly": 1,
                        },
                        {"marker": "lost+found", "at_least": None, "exactly": 2},
                    ],
                )
                self.assertEqual(
                    scripted["expect"]["forbidden_markers"],
                    [
                        "Function not implemented",
                        "arceos_ex panic",
                        "ext2 block read failure",
                    ],
                )

    def test_scripted_shell_ok_assertion_requires_a_standalone_output_line(self) -> None:
        qemu_log = self.root / "qemu.log"
        expected = {
            "process_exit": 0,
            "guest_exit_status": 0,
            "markers": [],
            "forbidden_markers": ["Function not implemented", "arceos_ex panic"],
            "marker_counts": [
                {"marker": "\nOK\n", "at_least": None, "exactly": 1},
                {
                    "marker": "ld-musl-riscv64.so.1",
                    "at_least": None,
                    "exactly": 1,
                },
                {"marker": "lost+found", "at_least": None, "exactly": 2},
            ],
        }
        outcome = runner.QemuOutcome(exit_code=0)
        echoed_only = (
            '~ # echo "OK"\n'
            "~ # ls\nlost+found\n"
            "~ # ls /lib\nld-musl-riscv64.so.1\n"
            "~ # ls /\nlost+found\n"
            "~ # exit\nuser exit status=0\n"
        )
        qemu_log.write_text(echoed_only)
        result = runner.evaluate_expectations(expected, qemu_log, outcome)
        self.assertFalse(result["passed"])
        ok_check = next(
            check
            for check in result["checks"]
            if check.get("marker") == "\nOK\n"
        )
        self.assertEqual(ok_check["actual"], 0)

        qemu_log.write_text(echoed_only.replace('~ # echo "OK"\n', '~ # echo "OK"\nOK\n'))
        self.assertTrue(runner.evaluate_expectations(expected, qemu_log, outcome)["passed"])

    def test_ltp_supported_and_frontier_cases_pin_three_distinct_targets(self) -> None:
        repo_root = Path(__file__).resolve().parents[4]
        cases = repo_root / "impl" / "arceos_ex" / "tests" / "basic" / "cases"
        expected = {
            "ltp": ("acceptance", "arceos_ex", "native", "supported"),
            "ltp-lo": ("acceptance", "arceos_ex", "linux-object", "supported"),
            "ltp-linux": ("acceptance", "linux", None, "supported"),
            "ltp-frontier": ("diagnostic", "arceos_ex", "native", "frontier"),
            "ltp-frontier-lo": ("diagnostic", "arceos_ex", "linux-object", "frontier"),
            "ltp-frontier-linux": ("diagnostic", "linux", None, "frontier"),
        }
        for name, (purpose, target, provider, selection) in expected.items():
            with self.subTest(name=name):
                config = runner.load_config(cases / f"{name}.toml", repo_root)
                self.assertEqual(config["purpose"], purpose)
                self.assertEqual(config["kernel"]["target"], target)
                if provider is not None:
                    self.assertEqual(config["kernel"]["app"], "user-boot")
                    self.assertEqual(config["kernel"]["provider"], provider)
                self.assertEqual(config["disk"], {"mode": "private-copy", "profile": "canonical"})
                self.assertEqual(config["qemu"]["smp"], 8)
                self.assertIn("init=/bin/sh", config["qemu"]["kernel_cmdline"])
                self.assertEqual(config["qemu"]["exit_policy"], "marker")
                self.assertEqual(config["qemu"]["interaction"], "scripted")
                payload = "".join(
                    step["payload"] for step in config["qemu"]["stdin_steps"]
                )
                self.assertEqual(payload, f"/opt/lkm/tests/ltp-init.sh {selection}\n")
                self.assertNotIn(config["qemu"]["exit_marker"], payload)

        supported_counts = {
            item["marker"]: item["exactly"]
            for item in runner.load_config(cases / "ltp.toml", repo_root)["expect"]["marker_counts"]
        }
        for entry in ("uname01", "uname02", "getuid01", "geteuid01"):
            self.assertEqual(supported_counts[f"--- {entry}: PASS (exit 0)"], 1)
        self.assertEqual(
            supported_counts["Summary: TOTAL=4 PASS=4 FAIL=0 BROK=0 WARN=0 CONF=0"],
            1,
        )
        selection_dir = repo_root / "impl" / "arceos_ex" / "tests" / "rootfs" / "canonical"
        self.assertEqual(
            (selection_dir / "ltp-supported").read_text().splitlines(),
            ["uname01", "uname02", "getuid01", "geteuid01"],
        )
        self.assertEqual(
            (selection_dir / "ltp-frontier").read_text().splitlines(),
            ["getgid03", "getegid02", "getresuid01", "getresgid01"],
        )

    def test_default_automation_pins_user_smoke_and_df0001_to_explicit_test_names(self) -> None:
        repo_root = Path(__file__).resolve().parents[4]
        summary = (repo_root / "tools" / "test_summary.sh").read_text()
        self.assertIn('"$make_cmd" run TEST="user-smoke-$provider"', summary)
        self.assertNotIn("APP=user-boot", summary)
        for terminal_name in ("shell", "shell-lo"):
            self.assertNotIn(f'TEST="{terminal_name}"', summary)
            self.assertNotIn(f'APP="{terminal_name}"', summary)
        self.assertIn('run_command_case "scripted shell $provider"', summary)
        self.assertIn('native) scripted_shell_test=scripted-shell', summary)
        self.assertIn('linux-object) scripted_shell_test=scripted-shell-lo', summary)
        self.assertIn('"$tmpdir/scripted-shell-$provider.log"', summary)
        self.assertIn('run TEST="$scripted_shell_test"', summary)
        self.assertNotIn('distro-sh-$provider', summary)
        self.assertNotIn('run_command_case "distro sh $provider"', summary)
        self.assertNotIn('run TEST="ltp"', summary)
        self.assertNotIn('run TEST="ltp-lo"', summary)
        makefile = (repo_root / "Makefile").read_text()
        self.assertIn("test-ltp:", makefile)
        self.assertIn("test-ltp-stress:", makefile)
        self.assertIn("$(MAKE) run TEST=ltp-linux", makefile)

        df0001_path = (
            repo_root
            / "impl"
            / "arceos_ex"
            / "tests"
            / "stress"
            / "cases"
            / "df-0001-user-boot.toml"
        )
        df0001 = tomllib.loads(df0001_path.read_text())
        self.assertEqual(df0001["schema_version"], 2)
        self.assertEqual(df0001["mode"], "stress")
        self.assertEqual(df0001["test"], "user-smoke-native")
        self.assertNotIn("command", df0001)

    def test_v2_rejects_unknown_template_profile(self) -> None:
        body = self.base_case(disk='mode = "private-copy"\nprofile = "nearby"')
        body = body.replace("schema_version = 1", "schema_version = 2")
        body = body.replace('name = "demo"', 'name = "demo"\npurpose = "acceptance"')
        body = body.replace('exit_policy = "guest-shutdown"', 'exit_policy = "guest-shutdown"\ninteraction = "none"')
        path = self.write_case(body)
        with self.assertRaisesRegex(runner.ConfigError, "disk.profile"):
            runner.load_config(path, self.root)

    def linux_case(self, *, kernel_extra: str = "", qemu_extra: str = "") -> str:
        return f"""
            schema_version = 2
            name = "demo"
            purpose = "diagnostic"
            timeout_seconds = 2

            [kernel]
            target = "linux"
            {kernel_extra}

            [disk]
            mode = "external"
            path = "disk.raw"
            readonly = false

            [qemu]
            memory_mb = 128
            smp = 2
            kernel_cmdline = "earlycon=sbi root=/dev/vda rw console=ttyS0"
            rng = false
            exit_policy = "stress-mem"
            interaction = "none"
            {qemu_extra}

            [expect]
            markers = ["SyscallTable.Wait4"]
        """

    def test_linux_kernel_target_is_a_strict_union(self) -> None:
        path = self.write_case(self.linux_case())
        config = runner.load_config(path, self.root)
        self.assertEqual(config["kernel"], {"target": "linux"})

        path.write_text(textwrap.dedent(self.linux_case(kernel_extra='app = "user-boot"')))
        with self.assertRaisesRegex(runner.ConfigError, "kernel target linux"):
            runner.load_config(path, self.root)

    def test_linux_manifest_builds_checkpoint_image_and_structured_network(self) -> None:
        linux = self.root / "linux"
        (linux / "arch" / "riscv" / "boot").mkdir(parents=True)
        path = self.write_case(
            self.linux_case(
                qemu_extra=(
                    "user_network = true\n"
                    "host_forwards = [{ protocol = \"tcp\", host_port = 5555, guest_port = 5555 }]"
                )
            )
        )
        with mock.patch.dict(os.environ, {"LINUX_PROVIDER_DIR": str(linux)}, clear=False):
            config = runner.load_config(path, self.root)
            manifest = runner.freeze_manifest(config, path, self.root, self.root / "artifacts")
        self.assertEqual(manifest["kernel"]["image"], str((linux / "arch/riscv/boot/Image").resolve()))
        self.assertIn("KCPPFLAGS=-DCONFIG_LKM_CHECKPOINTS", manifest["build_command"])
        self.assertIn("Image", manifest["build_command"])
        command = manifest["qemu"]["command"]
        self.assertIn("-bios", command)
        self.assertIn("virtio-net-device,netdev=net0", command)
        self.assertIn("hostfwd=tcp::5555-:5555", " ".join(command))
        self.assertIn("-qmp", command)
        self.assertIn("server=on,wait=off", " ".join(command))
        self.assertTrue(manifest["qemu"]["qmp_socket"].startswith("/tmp/lkm-qmp-"))

    def test_host_forward_requires_user_network(self) -> None:
        path = self.write_case(
            self.linux_case(
                qemu_extra=(
                    "host_forwards = [{ protocol = \"tcp\", host_port = 5555, guest_port = 5555 }]"
                )
            )
        )
        with self.assertRaisesRegex(runner.ConfigError, "require qemu.user_network"):
            runner.load_config(path, self.root)

    def test_stress_mem_parser_requires_complete_declared_hex_payload(self) -> None:
        text = "checkpoint: SyscallTable.Wait4\n"
        encoded = text.encode().hex()
        header = (
            f"stress_mem: v=1 encoding=hex bytes={len(text.encode())} "
            f"total={len(text.encode())} overflow=0 dropped=0 data="
        )
        self.assertIsNone(runner._parse_stress_mem(header + encoded[:-2]))
        parsed = runner._parse_stress_mem(header + encoded + "\n")
        self.assertIsNotNone(parsed)
        assert parsed is not None
        self.assertEqual(parsed[0], text)
        self.assertEqual(parsed[1]["bytes"], len(text.encode()))

    def test_stress_mem_observation_keeps_serial_and_decoded_payload(self) -> None:
        payload = "checkpoint: SyscallTable.Wait4\n"
        record = (
            f"stress_mem: v=1 encoding=hex bytes={len(payload.encode())} "
            f"total={len(payload.encode())} overflow=0 dropped=0 data={payload.encode().hex()}\n"
        )
        observed, metadata = runner._observed_text("login:\n" + record)
        self.assertIn("login:", observed)
        self.assertIn("checkpoint: SyscallTable.Wait4", observed)
        self.assertIsNotNone(metadata)


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
            import select
            import sys
            import time

            if path := os.environ.get("FAKE_QEMU_RAN"):
                pathlib.Path(path).write_text("ran")
            if path := os.environ.get("FAKE_QEMU_PID"):
                pathlib.Path(path).write_text(str(os.getpid()))
            if os.environ.get("FAKE_QEMU_CURSOR_QUERY"):
                sys.stdout.write("\\x1b[6n")
                sys.stdout.flush()
            mode = os.environ.get("FAKE_QEMU_MODE", "success")
            if mode == "timeout":
                time.sleep(30)
            elif mode == "marker":
                print("STOP", flush=True)
                time.sleep(30)
            elif mode == "stress-mem":
                text = "checkpoint: SyscallTable.Wait4\\n"
                encoded = text.encode().hex()
                print(
                    f"stress_mem: v=1 encoding=hex bytes={len(text.encode())} "
                    f"total={len(text.encode())} overflow=0 dropped=0 data={encoded}",
                    flush=True,
                )
                time.sleep(30)
            else:
                if os.environ.get("FAKE_QEMU_REPEAT_READY"):
                    print("READY", flush=True)
                    first = sys.stdin.readline().strip()
                    time.sleep(0.2)
                    if select.select([sys.stdin], [], [], 0)[0]:
                        print("PREMATURE-SECOND-STEP", flush=True)
                        raise SystemExit(9)
                    print(f"ACK:{first}", flush=True)
                    print("READY", flush=True)
                    second = sys.stdin.readline().strip()
                    print(f"ACK:{second}", flush=True)
                elif os.environ.get("FAKE_QEMU_STDIN"):
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

    def test_retired_user_boot_writes_directional_schema_v2_failure(self) -> None:
        for command in ("build", "run"):
            with self.subTest(command=command):
                output = self.root / f"retired-user-boot-{command}"
                status = runner.main(
                    [
                        command,
                        "user-boot",
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
                self.assertEqual(result["schema_version"], 2)
                self.assertEqual(result["request"]["test"], "user-boot")
                self.assertEqual(result["request"]["canonical_test"], "user-boot")
                self.assertIsNone(result["request"]["compatibility_alias"])
                self.assertEqual(result["execution_status"], "failed")
                self.assertEqual(result["verdict"], "inconclusive")
                self.assertEqual(result["stages"]["kernel-build"]["status"], "skipped")
                self.assertIn("use 'shell'", result["errors"][0])
                self.assertEqual((output / "qemu.log").read_bytes(), b"")

    def test_removed_shell_names_write_ordinary_schema_v2_config_failures(self) -> None:
        for command in ("build", "run"):
            for removed_name in ("shell-native", "shell-linux-object"):
                with self.subTest(command=command, removed_name=removed_name):
                    output = self.root / f"removed-{removed_name}-{command}"
                    status = runner.main(
                        [
                            command,
                            removed_name,
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
                    self.assertEqual(result["schema_version"], 2)
                    self.assertEqual(result["request"]["test"], removed_name)
                    self.assertEqual(result["request"]["canonical_test"], removed_name)
                    self.assertIsNone(result["request"]["compatibility_alias"])
                    self.assertEqual(result["execution_status"], "failed")
                    self.assertEqual(result["stages"]["kernel-build"]["status"], "skipped")
                    self.assertIn(f"{removed_name}.toml", result["errors"][0])
                    self.assertNotIn("retired", result["errors"][0])
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
        self.assertEqual(result["qemu"]["timeout_diagnostics"]["status"], "failed")
        self.assertTrue((output / "qemu-timeout-diagnostics.json").is_file())
        self.assertTrue(result["cleanup"]["process_group_reaped"])
        self.assertTrue(result["cleanup"]["qmp_socket_removed"])
        self.assertTrue((output / "post-ran").exists())
        pid = int(pid_file.read_text())
        with self.assertRaises(ProcessLookupError):
            os.kill(pid, 0)

    def test_timeout_diagnostics_capture_fixed_qmp_snapshot(self) -> None:
        qmp_socket = self.root / "qmp.sock"
        diagnostic_path = self.root / "qemu-timeout-diagnostics.json"
        listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        listener.bind(str(qmp_socket))
        listener.listen(1)

        def serve() -> None:
            stopped = False
            connection, _ = listener.accept()
            with connection, connection.makefile("rb") as stream:
                connection.sendall(
                    b'{"QMP":{"version":{"qemu":{"major":9,"minor":0,"micro":0},'
                    b'"package":""},"capabilities":[]}}\r\n'
                )
                while line := stream.readline():
                    request = json.loads(line)
                    command = request["execute"]
                    command_id = request["id"]
                    if command == "stop":
                        stopped = True
                        connection.sendall(b'{"event":"STOP","data":{}}\r\n')
                        result: object = {}
                    elif command == "query-status":
                        result = {"status": "paused" if stopped else "running"}
                    elif command == "query-cpus-fast":
                        result = [{"cpu-index": 0, "thread-id": 101}]
                    elif command == "human-monitor-command":
                        result = f"snapshot: {request['arguments']['command-line']}"
                    else:
                        result = {}
                    response = {"return": result, "id": command_id}
                    connection.sendall((json.dumps(response) + "\r\n").encode())

        server = threading.Thread(target=serve)
        server.start()
        manifest = {
            "test": "demo",
            "kernel": {"image": str(self.root / "kernel.bin")},
            "qemu": {
                "qmp_socket": str(qmp_socket),
                "timeout_diagnostics": str(diagnostic_path),
            },
        }
        try:
            summary = runner._capture_timeout_diagnostics(manifest)
        finally:
            server.join(timeout=3)
            listener.close()
        self.assertFalse(server.is_alive())
        self.assertEqual(summary["status"], "captured")
        diagnostic = json.loads(diagnostic_path.read_text())
        self.assertEqual(diagnostic["queries"]["status_before_stop"]["status"], "running")
        self.assertEqual(diagnostic["queries"]["status_after_stop"]["status"], "paused")
        self.assertEqual(diagnostic["queries"]["cpus"][0]["cpu-index"], 0)
        self.assertIn("info registers -a", diagnostic["queries"]["registers"])

    def test_marker_exit_policy_terminates_process_group(self) -> None:
        self.environment["FAKE_QEMU_MODE"] = "marker"
        path = self.write_case(
            qemu='exit_policy = "marker"\nexit_marker = "STOP"',
            expect='markers = ["STOP"]',
        )
        status, _, result = self.run_case(path, "marker")
        self.assertEqual(status, 0)
        self.assertTrue(result["qemu"]["terminated_after_marker"])

    def test_stress_mem_exit_policy_terminates_only_after_complete_record(self) -> None:
        self.environment["FAKE_QEMU_MODE"] = "stress-mem"
        path = self.write_case(
            qemu='exit_policy = "stress-mem"',
            expect='markers = ["SyscallTable.Wait4"]',
        )
        status, _, result = self.run_case(path, "stress-mem")
        self.assertEqual(status, 0)
        self.assertTrue(result["qemu"]["terminated_after_stress_mem"])
        self.assertGreater(result["qemu"]["stress_mem"]["bytes"], 0)
        self.assertTrue(result["expectations"]["passed"])

    def test_stdin_step_is_sent_after_ready_marker(self) -> None:
        self.environment["FAKE_QEMU_STDIN"] = "1"
        path = self.write_case(stdin='stdin_steps = [{ ready_marker = "READY", payload = "go\\n" }]')
        status, _, result = self.run_case(path, "stdin")
        self.assertEqual(status, 0)
        self.assertTrue(result["qemu"]["stdin_steps"][0]["sent"])

    def test_repeated_ready_marker_requires_new_output_after_each_step(self) -> None:
        environment = {**self.environment, "FAKE_QEMU_REPEAT_READY": "1"}
        path = self.write_case(
            stdin=(
                'stdin_steps = ['
                '{ ready_marker = "READY", payload = "one\\n" }, '
                '{ ready_marker = "READY", payload = "two\\n" }'
                ']'
            )
        )
        with mock.patch.dict(os.environ, environment, clear=False):
            status, output, result = self.run_case(path, "repeat-ready")
        self.assertEqual(status, 0)
        self.assertTrue(all(step["sent"] for step in result["qemu"]["stdin_steps"]))
        log = (output / "qemu.log").read_text()
        self.assertNotIn("PREMATURE-SECOND-STEP", log)
        self.assertLess(log.index("ACK:one"), log.index("ACK:two"))

    def test_nonterminal_presentation_filters_cursor_query_but_log_keeps_it(self) -> None:
        self.environment["FAKE_QEMU_CURSOR_QUERY"] = "1"
        rendered = bytearray()
        with mock.patch.object(runner, "_console_write", side_effect=rendered.extend):
            status, output, _ = self.run_case(self.write_case(), "cursor-query")

        self.assertEqual(status, 0)
        self.assertIn(b"\x1b[6n", (output / "qemu.log").read_bytes())
        self.assertNotIn(b"\x1b[6n", rendered)
        self.assertIn(b"SUCCESS", rendered)

    def test_nonterminal_presentation_filters_query_across_read_boundaries(self) -> None:
        rendered = bytearray()
        presentation = runner._NonTerminalConsolePresentation()
        with mock.patch.object(runner, "_console_write", side_effect=rendered.extend):
            presentation.write(b"prompt \x1b")
            presentation.write(b"[6")
            presentation.write(b"n/bin/ls\n")
            presentation.finish()

        self.assertEqual(rendered, b"prompt /bin/ls\n")

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
                "qmp_socket": str(self.root / "terminal-qmp.sock"),
                "timeout_diagnostics": str(self.root / "terminal-timeout.json"),
            },
        }
        outcome = runner.QemuOutcome(stdin_steps=[])

        def send_input() -> None:
            time.sleep(0.2)
            os.write(input_master, b"go\n")

        sender = threading.Thread(target=send_input)
        sender.start()
        environment = {
            **self.environment,
            "FAKE_QEMU_CURSOR_QUERY": "1",
            "FAKE_QEMU_STDIN": "1",
        }
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
        self.assertIn(b"\x1b[6n", log_path.read_bytes())
        self.assertIn(b"\x1b[6n", output.buffer.getvalue())
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

    def test_canonical_configuration_installs_busybox_inittab_locks_root_and_merges_test(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            staging = root / "staging"
            config = root / "config"
            destination = staging / "opt" / "lkm" / "tests"
            (staging / "etc").mkdir(parents=True)
            destination.mkdir(parents=True)
            config.mkdir()
            (staging / "etc" / "inittab").write_text("distribution input\n")
            (staging / "etc" / "passwd").write_text("root:x:0:0:root:/root:/bin/sh\n")
            (staging / "etc" / "shadow").write_text("root:*::0:::::\n")
            (config / "passwd.entry").write_text("test:hash:1000:100:test:/:/bin/sh\n")
            (config / "shadow.entry").write_text("test:hash:0:::::\n")
            (config / "rc-local.sh").write_text("#!/bin/sh\nexit 0\n")
            (config / "ltp-supported").write_text("uname01\n")
            (config / "ltp-frontier").write_text("getuid01\n")
            (config / "ltp-select.sh").write_text("#!/bin/sh\nexit 0\n")
            (config / "ltp-init.sh").write_text("#!/bin/sh\nexit 0\n")
            (config / "ltp-proc-meminfo").write_text(
                "MemAvailable: 65536 kB\nSwapFree: 0 kB\n"
            )
            configured_inittab = (
                "tty1::respawn:/bin/sh -c getty-tty1-and-poweroff\n"
                "ttyS0::respawn:/bin/sh -c getty-ttyS0-and-poweroff\n"
            )
            (config / "inittab").write_text(configured_inittab)

            rootfs_builder._configure_canonical(staging, config, destination)

            self.assertEqual((staging / "etc" / "inittab").read_text(), configured_inittab)
            self.assertNotIn("/sbin/openrc", (staging / "etc" / "inittab").read_text())
            self.assertIn("root:*:", (staging / "etc" / "shadow").read_text())
            self.assertEqual(sum(line.startswith("test:") for line in (staging / "etc" / "passwd").read_text().splitlines()), 1)
            self.assertEqual(sum(line.startswith("test:") for line in (staging / "etc" / "shadow").read_text().splitlines()), 1)
            self.assertTrue(os.access(destination / "rc-local.sh", os.X_OK))
            self.assertEqual((destination / "ltp-supported").read_text(), "uname01\n")
            self.assertEqual((destination / "ltp-frontier").read_text(), "getuid01\n")
            self.assertTrue(os.access(destination / "ltp-select.sh", os.X_OK))
            self.assertTrue(os.access(destination / "ltp-init.sh", os.X_OK))
            self.assertEqual(
                (staging / "proc" / "meminfo").read_text(),
                "MemAvailable: 65536 kB\nSwapFree: 0 kB\n",
            )
            self.assertEqual(
                stat.S_IMODE((staging / "proc" / "meminfo").stat().st_mode), 0o444
            )

    def test_ltp_selection_requires_exact_unique_known_runtest_entries(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            ltp = root / "ltp"
            config = root / "config"
            (ltp / "runtest").mkdir(parents=True)
            config.mkdir()
            runtest = ltp / "runtest" / "syscalls"
            runtest.write_text("uname01 uname01\ngetuid01 getuid01\n")
            supported = config / "ltp-supported"
            frontier = config / "ltp-frontier"
            supported.write_text("uname01\n")
            frontier.write_text("getuid01\n")

            self.assertEqual(
                rootfs_builder.validate_ltp_selections(ltp, config),
                {"supported": ["uname01"], "frontier": ["getuid01"]},
            )

            supported.write_text("uname01\nuname01\n")
            with self.assertRaisesRegex(ValueError, "duplicate entry"):
                rootfs_builder.validate_ltp_selections(ltp, config)

            supported.write_text("unknown01\n")
            with self.assertRaisesRegex(ValueError, "found 0"):
                rootfs_builder.validate_ltp_selections(ltp, config)

            supported.write_text(" uname01\n")
            with self.assertRaisesRegex(ValueError, "one exact entry name"):
                rootfs_builder.validate_ltp_selections(ltp, config)

            supported.write_text("uname01\n")
            runtest.write_text("uname01 uname01\nuname01 uname01\ngetuid01 getuid01\n")
            with self.assertRaisesRegex(ValueError, "found 2"):
                rootfs_builder.validate_ltp_selections(ltp, config)

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
            for name in (
                "inittab",
                "passwd.entry",
                "shadow.entry",
                "rc-local.sh",
                "ltp-supported",
                "ltp-frontier",
                "ltp-select.sh",
                "ltp-init.sh",
                "ltp-proc-meminfo",
            ):
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
