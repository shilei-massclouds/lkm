#!/usr/bin/env python3
"""Run one versioned basic kernel/QEMU test configuration."""

from __future__ import annotations

import argparse
from contextlib import contextmanager
from dataclasses import dataclass
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import pty
import re
import selectors
import shlex
import shutil
import signal
import socket
import subprocess
import sys
import termios
import time
import tomllib
import tty
from typing import Any, Iterator

try:
    from .rootfs_builder import parse_size, validate_template
except ImportError:  # Direct script execution.
    from rootfs_builder import parse_size, validate_template


SCHEMA_VERSION = 2
RESULT_SCHEMA_VERSION = 2
TEST_NAME_RE = re.compile(r"^[a-z0-9][a-z0-9-]*$")
GUEST_EXIT_RE = re.compile(r"user exit status=(-?\d+)")
ANSI_RE = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")
STRESS_MEM_HEADER_RE = re.compile(
    r"stress_mem: v=1 encoding=hex bytes=(?P<bytes>\d+) total=(?P<total>\d+) "
    r"overflow=(?P<overflow>[01]) dropped=(?P<dropped>\d+) data="
)
STRESS_MEM_PROMPT_ECHO_RE = re.compile(r"(?:~|/) # [^\r\n]*(?:\r?\n)?")
STAGES = (
    "config",
    "kernel-build",
    "disk-prepare",
    "pre-script",
    "qemu",
    "post-script",
    "result-cleanup",
)
ALLOWED_PROVIDERS = {"native", "linux-object"}
ALLOWED_PROFILES = {"release", "trace"}
ALLOWED_APPS = {"hello", "smoke", "user-boot"}
ALLOWED_DISK_MODES = {"none", "template-readonly", "private-copy", "generated", "external"}
ALLOWED_KERNEL_TARGETS = {"arceos_ex", "linux"}
ALLOWED_STRESS_MEM_BYTES = {32768, 65536, 131072, 262144, 524288}
ALLOWED_EXIT_POLICIES = {"process-exit", "guest-shutdown", "marker", "stress-mem"}
ALLOWED_PURPOSES = {"acceptance", "diagnostic"}
ALLOWED_INTERACTIONS = {"none", "scripted", "terminal"}
ALLOWED_ROOTFS_PROFILES = {"canonical"}
NON_TERMINAL_PRESENTATION_QUERIES = (b"\x1b[6n",)
COMPATIBILITY_ALIASES = {
    "hello": "hello-native",
    "smoke": "kernel-smoke-native",
    "kunit-native": "checkpoint-kunit-native",
    "kunit-linux-object": "checkpoint-kunit-linux-object",
}
RETIRED_TEST_NAMES = {
    "user-boot": "test name 'user-boot' is retired; use 'shell' for an interactive shell",
}


class ConfigError(ValueError):
    pass


class PipelineTimeout(RuntimeError):
    pass


@dataclass
class QemuOutcome:
    exit_code: int | None = None
    timed_out: bool = False
    terminated_after_marker: bool = False
    terminated_after_stress_mem: bool = False
    stress_mem: dict[str, Any] | None = None
    process_group_reaped: bool = True
    stdin_steps: list[dict[str, Any]] | None = None
    terminal_restored: bool | None = None
    timeout_diagnostics: dict[str, Any] | None = None


class _NonTerminalConsolePresentation:
    def __init__(self) -> None:
        self._pending = b""

    def write(self, chunk: bytes) -> None:
        data = self._pending + chunk
        pending_length = 0
        for query in NON_TERMINAL_PRESENTATION_QUERIES:
            for length in range(1, len(query)):
                if data.endswith(query[:length]):
                    pending_length = max(pending_length, length)
        if pending_length:
            data, self._pending = data[:-pending_length], data[-pending_length:]
        else:
            self._pending = b""
        for query in NON_TERMINAL_PRESENTATION_QUERIES:
            data = data.replace(query, b"")
        if data:
            _console_write(data)

    def finish(self) -> None:
        if self._pending:
            _console_write(self._pending)
            self._pending = b""


def main(argv: list[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    repo_root = args.repo_root.resolve()
    requested_test = args.test
    canonical_test = COMPATIBILITY_ALIASES.get(requested_test, requested_test)
    compatibility_alias = requested_test if canonical_test != requested_test else None
    retired_error = RETIRED_TEST_NAMES.get(requested_test)
    if retired_error is not None:
        error = ConfigError(retired_error)
        print(f"basic test configuration error: {error}", file=sys.stderr)
        if args.command in {"build", "run"}:
            return _record_early_config_failure(
                args=args,
                repo_root=repo_root,
                requested_test=requested_test,
                canonical_test=canonical_test,
                compatibility_alias=compatibility_alias,
                error=error,
            )
        return 1
    try:
        config_path = resolve_case(canonical_test, args.cases_dir, repo_root)
    except ConfigError as error:
        print(f"basic test configuration error: {error}", file=sys.stderr)
        if args.command in {"build", "run"}:
            return _record_early_config_failure(
                args=args,
                repo_root=repo_root,
                requested_test=requested_test,
                canonical_test=canonical_test,
                compatibility_alias=compatibility_alias,
                error=error,
            )
        return 1

    if args.command == "manifest":
        try:
            config = load_config(config_path, repo_root)
            artifact_dir = _manifest_artifact_dir(args, repo_root, config["name"])
            manifest = freeze_manifest(config, config_path, repo_root, artifact_dir)
            manifest["request"] = {
                "test": requested_test,
                "canonical_test": canonical_test,
                "compatibility_alias": compatibility_alias,
            }
        except ConfigError as error:
            print(f"basic test configuration error: {error}", file=sys.stderr)
            return 1
        encoded = json.dumps(manifest, indent=2, sort_keys=True) + "\n"
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(encoded)
        else:
            print(encoded, end="")
        return 0

    return run_pipeline(
        command=args.command,
        config_path=config_path,
        repo_root=repo_root,
        out_root=args.out_root,
        output_dir=args.output_dir,
        requested_test=requested_test,
        compatibility_alias=compatibility_alias,
    )


def _parser() -> argparse.ArgumentParser:
    repo_root = Path(__file__).resolve().parents[4]
    cases_dir = repo_root / "impl" / "arceos_ex" / "tests" / "basic" / "cases"
    out_root = repo_root / "impl" / "arceos_ex" / "tests" / "basic" / "out"
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("run", "build", "manifest"))
    parser.add_argument("test", nargs="?", default="hello-native")
    parser.add_argument("--repo-root", type=Path, default=repo_root)
    parser.add_argument("--cases-dir", type=Path, default=cases_dir)
    parser.add_argument("--out-root", type=Path, default=out_root)
    parser.add_argument("--output-dir", type=Path)
    parser.add_argument("--output", type=Path, help="manifest output path")
    return parser


def resolve_case(test: str, cases_dir: Path, repo_root: Path) -> Path:
    if not TEST_NAME_RE.fullmatch(test):
        raise ConfigError(f"invalid test name: {test!r}")
    directory = cases_dir if cases_dir.is_absolute() else repo_root / cases_dir
    path = (directory / f"{test}.toml").resolve()
    return path


def load_config(path: Path, repo_root: Path) -> dict[str, Any]:
    try:
        raw = tomllib.loads(path.read_text())
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise ConfigError(f"cannot parse {path}: {error}") from error
    _table(raw, "top level")
    _keys(raw, {"schema_version", "name", "purpose", "timeout_seconds", "pre_script", "post_script", "kernel", "disk", "qemu", "expect"}, "top level")
    version = _integer(raw, "schema_version", "top level")
    if version not in {1, SCHEMA_VERSION}:
        raise ConfigError(f"unsupported schema_version: {version}")
    compatibility_mappings: list[str] = []
    if version == 1:
        purpose = "acceptance"
        if "purpose" in raw:
            raise ConfigError("purpose is not valid in schema v1")
    else:
        purpose = _string(raw, "purpose", "top level")
        if purpose not in ALLOWED_PURPOSES:
            raise ConfigError(f"purpose must be one of {sorted(ALLOWED_PURPOSES)}")
    name = _string(raw, "name", "top level")
    if not TEST_NAME_RE.fullmatch(name):
        raise ConfigError(f"invalid configured test name: {name!r}")
    if path.stem != name:
        raise ConfigError(f"configured name {name!r} does not match file {path.name!r}")
    timeout = _positive_integer(raw, "timeout_seconds", "top level")

    kernel_raw = _required_table(raw, "kernel", "top level")
    if version == 1 and "target" in kernel_raw:
        raise ConfigError("kernel.target is not valid in schema v1")
    target = kernel_raw.get("target", "arceos_ex")
    if not isinstance(target, str) or target not in ALLOWED_KERNEL_TARGETS:
        raise ConfigError(f"kernel.target must be one of {sorted(ALLOWED_KERNEL_TARGETS)}")
    if target == "linux":
        _keys(kernel_raw, {"target"}, "kernel target linux")
        kernel = {"target": target}
    else:
        _keys(kernel_raw, {"target", "app", "provider", "probe", "probe_file", "profile", "stress_mem_bytes", "extra_rustflags"}, "kernel target arceos_ex")
        app = _string(kernel_raw, "app", "kernel")
        if app not in ALLOWED_APPS:
            raise ConfigError(f"kernel.app must be one of {sorted(ALLOWED_APPS)}")
        provider = _string(kernel_raw, "provider", "kernel")
        if provider not in ALLOWED_PROVIDERS:
            raise ConfigError(f"kernel.provider must be one of {sorted(ALLOWED_PROVIDERS)}")
        profile = _string(kernel_raw, "profile", "kernel")
        if profile not in ALLOWED_PROFILES:
            raise ConfigError(f"kernel.profile must be one of {sorted(ALLOWED_PROFILES)}")
        probe = _string_list(kernel_raw.get("probe", []), "kernel.probe")
        probe_file = _optional_repo_path(kernel_raw, "probe_file", repo_root, "kernel")
        stress_mem_bytes = kernel_raw.get("stress_mem_bytes")
        if stress_mem_bytes is not None and (
            not _is_integer(stress_mem_bytes) or stress_mem_bytes not in ALLOWED_STRESS_MEM_BYTES
        ):
            raise ConfigError(
                f"kernel.stress_mem_bytes must be one of {sorted(ALLOWED_STRESS_MEM_BYTES)}"
            )
        extra_rustflags = _string_list(kernel_raw.get("extra_rustflags", []), "kernel.extra_rustflags")
        kernel = {
            "target": target,
            "app": app,
            "provider": provider,
            "probe": probe,
            "probe_file": str(probe_file) if probe_file else None,
            "profile": profile,
            "stress_mem_bytes": stress_mem_bytes,
            "extra_rustflags": extra_rustflags,
        }

    disk_raw = _required_table(raw, "disk", "top level")
    _keys(disk_raw, {"mode", "profile", "path", "readonly", "generator", "size"}, "disk")
    disk = _parse_disk(disk_raw, repo_root, version, compatibility_mappings)

    qemu_raw = _required_table(raw, "qemu", "top level")
    _keys(qemu_raw, {"memory_mb", "smp", "kernel_cmdline", "rng", "user_network", "host_forwards", "exit_policy", "exit_marker", "interaction", "stdin_steps"}, "qemu")
    memory_mb = _positive_integer(qemu_raw, "memory_mb", "qemu")
    smp = _positive_integer(qemu_raw, "smp", "qemu")
    kernel_cmdline = _string(qemu_raw, "kernel_cmdline", "qemu")
    rng = _boolean(qemu_raw, "rng", "qemu")
    user_network = qemu_raw.get("user_network", False)
    if not isinstance(user_network, bool):
        raise ConfigError("qemu.user_network must be a boolean")
    host_forwards = _host_forwards(qemu_raw.get("host_forwards", []))
    if host_forwards and not user_network:
        raise ConfigError("qemu.host_forwards require qemu.user_network = true")
    exit_policy = _string(qemu_raw, "exit_policy", "qemu")
    if exit_policy not in ALLOWED_EXIT_POLICIES:
        raise ConfigError(f"qemu.exit_policy must be one of {sorted(ALLOWED_EXIT_POLICIES)}")
    exit_marker = qemu_raw.get("exit_marker")
    if exit_policy == "marker":
        if not isinstance(exit_marker, str) or not exit_marker:
            raise ConfigError("qemu.exit_marker is required for marker exit_policy")
    elif exit_marker is not None:
        raise ConfigError("qemu.exit_marker is only valid for marker exit_policy")
    stdin_steps = _stdin_steps(qemu_raw.get("stdin_steps", []))
    if version == 1:
        if "interaction" in qemu_raw:
            raise ConfigError("qemu.interaction is not valid in schema v1")
        interaction = "scripted" if stdin_steps else "none"
        compatibility_mappings.append(f"qemu.interaction={interaction}")
    else:
        interaction = _string(qemu_raw, "interaction", "qemu")
        if interaction not in ALLOWED_INTERACTIONS:
            raise ConfigError(f"qemu.interaction must be one of {sorted(ALLOWED_INTERACTIONS)}")
    if interaction == "none" and stdin_steps:
        raise ConfigError("qemu.interaction none forbids stdin_steps")
    if interaction == "scripted" and not stdin_steps:
        raise ConfigError("qemu.interaction scripted requires stdin_steps")
    if interaction == "terminal" and stdin_steps:
        raise ConfigError("qemu.interaction terminal forbids stdin_steps")

    expect_raw = _required_table(raw, "expect", "top level")
    _keys(expect_raw, {"process_exit", "guest_exit_status", "markers", "forbidden_markers", "marker_counts"}, "expect")
    process_exit = _optional_integer(expect_raw, "process_exit", "expect")
    guest_exit_status = _optional_integer(expect_raw, "guest_exit_status", "expect")
    markers = _string_list(expect_raw.get("markers", []), "expect.markers")
    forbidden_markers = _string_list(expect_raw.get("forbidden_markers", []), "expect.forbidden_markers")
    marker_counts = _marker_counts(expect_raw.get("marker_counts", []))
    if purpose == "acceptance" and not markers and not forbidden_markers and not marker_counts and process_exit is None and guest_exit_status is None:
        raise ConfigError("expect must declare at least one observable fact")

    scripts = {
        "pre": _script_path(raw, "pre_script", path),
        "post": _script_path(raw, "post_script", path),
    }
    return {
        "schema_version": SCHEMA_VERSION,
        "source_schema_version": version,
        "compatibility_mappings": compatibility_mappings,
        "name": name,
        "purpose": purpose,
        "timeout_seconds": timeout,
        "scripts": scripts,
        "kernel": kernel,
        "disk": disk,
        "qemu": {
            "memory_mb": memory_mb,
            "smp": smp,
            "kernel_cmdline": kernel_cmdline,
            "rng": rng,
            "user_network": user_network,
            "host_forwards": host_forwards,
            "exit_policy": exit_policy,
            "exit_marker": exit_marker,
            "interaction": interaction,
            "stdin_steps": stdin_steps,
        },
        "expect": {
            "process_exit": process_exit,
            "guest_exit_status": guest_exit_status,
            "markers": markers,
            "forbidden_markers": forbidden_markers,
            "marker_counts": marker_counts,
        },
    }


def freeze_manifest(config: dict[str, Any], config_path: Path, repo_root: Path, artifact_dir: Path) -> dict[str, Any]:
    kernel_dir = repo_root / "impl" / "arceos_ex"
    probes = set(config["kernel"].get("probe", []))
    probe_file = config["kernel"].get("probe_file")
    if probe_file:
        for line in Path(probe_file).read_text().splitlines():
            content = line.partition("#")[0].strip()
            if content:
                probes.update(content.replace(",", " ").split())
    sorted_probes = sorted(probes)
    kernel_image = kernel_image_path(repo_root, kernel_dir, config["kernel"], sorted_probes)
    canonical = _canonical_image(repo_root)
    disk = dict(config["disk"])
    disk["canonical_path"] = str(canonical)
    if disk["mode"] in {"private-copy", "generated"}:
        disk["runtime_path"] = str((artifact_dir / "private-disk.raw").resolve())
    elif disk["mode"] == "template-readonly":
        disk["runtime_path"] = str(canonical)
    elif disk["mode"] == "external":
        disk["runtime_path"] = disk["path"]
    else:
        disk["runtime_path"] = None
    build_command = kernel_build_command(repo_root, kernel_dir, config["kernel"])
    artifact_identity = hashlib.sha256(str(artifact_dir.resolve()).encode()).hexdigest()[:12]
    qmp_socket = Path("/tmp") / f"lkm-qmp-{os.getpid()}-{artifact_identity}.sock"
    timeout_diagnostics = (artifact_dir / "qemu-timeout-diagnostics.json").resolve()
    qemu_command = qemu_command_for(config, kernel_image, disk, qmp_socket)
    return {
        "schema_version": SCHEMA_VERSION,
        "source_schema_version": config["source_schema_version"],
        "compatibility_mappings": config["compatibility_mappings"],
        "test": config["name"],
        "purpose": config["purpose"],
        "config_path": str(config_path.resolve()),
        "artifact_dir": str(artifact_dir.resolve()),
        "timeout_seconds": config["timeout_seconds"],
        "scripts": config["scripts"],
        "kernel": {**config["kernel"], "resolved_probes": sorted_probes, "image": str(kernel_image)},
        "disk": disk,
        "qemu": {
            **config["qemu"],
            "command": qemu_command,
            "qmp_socket": str(qmp_socket),
            "timeout_diagnostics": str(timeout_diagnostics),
        },
        "expect": config["expect"],
        "build_command": build_command,
    }


def kernel_image_path(repo_root: Path, kernel_dir: Path, kernel: dict[str, Any], probes: list[str]) -> Path:
    if kernel["target"] == "linux":
        return (_linux_provider_dir(repo_root) / "arch" / "riscv" / "boot" / "Image").resolve()
    suffix = ""
    if kernel["provider"] != "native":
        suffix += f"/plic-{kernel['provider']}"
    if probes:
        label = "+".join(probes).replace("/", "_").replace(".", "_")
        suffix += f"/probe-{label}"
        if "stress-mem" in probes:
            suffix += f"-bytes{kernel['stress_mem_bytes'] or 65536}"
    relative = f"build/riscv64imac-unknown-none-elf/{kernel['profile']}/{kernel['app']}{suffix}/arceos_ex.bin"
    return (kernel_dir / relative).resolve()


def kernel_build_command(repo_root: Path, kernel_dir: Path, kernel: dict[str, Any]) -> list[str]:
    if kernel["target"] == "linux":
        return [
            *_tool_command("MAKE", "make"),
            "-C",
            str(_linux_provider_dir(repo_root)),
            "ARCH=riscv",
            f"CROSS_COMPILE={os.environ.get('LINUX_CROSS_COMPILE', 'riscv64-linux-gnu-')}",
            "KCPPFLAGS=-DCONFIG_LKM_CHECKPOINTS",
            "-j",
            str(max(1, os.cpu_count() or 1)),
            "Image",
        ]
    command = [
        *_tool_command("MAKE", "make"),
        "-C",
        str(kernel_dir),
        "build",
        f"APP={kernel['app']}",
        f"PLIC_PROVIDER={kernel['provider']}",
        f"PROFILE={kernel['profile']}",
        f"PROBE={','.join(kernel['probe'])}",
        f"EXTRA_RUSTFLAGS={' '.join(kernel['extra_rustflags'])}",
    ]
    if kernel["probe_file"]:
        command.append(f"PROBE_FILE={kernel['probe_file']}")
    if kernel["stress_mem_bytes"] is not None:
        command.append(f"STRESS_MEM_BYTES={kernel['stress_mem_bytes']}")
    return command


def qemu_command_for(
    config: dict[str, Any], kernel_image: Path, disk: dict[str, Any], qmp_socket: Path
) -> list[str]:
    qemu = config["qemu"]
    command = [
        *_tool_command("QEMU", "qemu-system-riscv64"),
        "-machine",
        "virt",
        "-m",
        f"{qemu['memory_mb']}M",
        "-smp",
        str(qemu["smp"]),
        "-nographic",
        "-serial",
        "mon:stdio",
        "-qmp",
        f"unix:{qmp_socket},server=on,wait=off",
    ]
    if config["kernel"]["target"] == "arceos_ex":
        command[3:3] = ["-cpu", "rv64"]
    else:
        command.extend(["-bios", "default"])
    if qemu["rng"]:
        command.extend(["-object", "rng-random,id=rng0,filename=/dev/urandom", "-device", "virtio-rng-device,rng=rng0"])
    if qemu["user_network"]:
        netdev = "user,id=net0"
        for forward in qemu["host_forwards"]:
            netdev += (
                f",hostfwd={forward['protocol']}:{forward['host_address']}:"
                f"{forward['host_port']}-:{forward['guest_port']}"
            )
        command.extend(["-device", "virtio-net-device,netdev=net0", "-netdev", netdev])
    runtime_path = disk.get("runtime_path")
    if runtime_path:
        readonly = disk["mode"] == "template-readonly" or (disk["mode"] == "external" and disk["readonly"])
        drive = f"file={runtime_path},if=none,format=raw,id=blk0"
        if readonly:
            drive += ",readonly=on"
        command.extend(["-drive", drive, "-device", "virtio-blk-device,drive=blk0"])
    command.extend(["-append", qemu["kernel_cmdline"], "-kernel", str(kernel_image)])
    return command


def run_pipeline(
    *,
    command: str,
    config_path: Path,
    repo_root: Path,
    out_root: Path,
    output_dir: Path | None,
    requested_test: str | None = None,
    compatibility_alias: str | None = None,
) -> int:
    test_hint = config_path.stem
    requested_test = requested_test or test_hint
    artifact_dir = _new_artifact_dir(repo_root, out_root, output_dir, test_hint)
    qemu_log = artifact_dir / "qemu.log"
    qemu_log.touch()
    result_path = artifact_dir / "result.json"
    manifest_path = artifact_dir / "manifest.json"
    result = _initial_result(
        requested_test,
        test_hint,
        compatibility_alias,
        command,
        config_path,
        artifact_dir,
        manifest_path,
        qemu_log,
        result_path,
    )
    manifest: dict[str, Any] | None = None
    config: dict[str, Any] | None = None
    qemu_outcome = QemuOutcome(stdin_steps=[])
    private_disk: Path | None = None
    qemu_started = False
    failure = False
    expectation_failed = False

    try:
        with _stage(result, "config"):
            config = load_config(config_path, repo_root)
            result["test"] = config["name"]
            result["purpose"] = config["purpose"]
            manifest = freeze_manifest(config, config_path, repo_root, artifact_dir)
            manifest["request"] = {
                "test": requested_test,
                "canonical_test": config["name"],
                "compatibility_alias": compatibility_alias,
            }
            _write_json(manifest_path, manifest)
            result["manifest"] = str(manifest_path)
            if command == "run" and config["qemu"]["interaction"] == "terminal" and not _terminal_available():
                raise ConfigError("qemu.interaction terminal requires a real stdin/stdout TTY")

        with _stage(result, "kernel-build"):
            _run_logged(manifest["build_command"], repo_root, artifact_dir / "kernel-build.log")
            kernel_image = Path(manifest["kernel"]["image"])
            if not kernel_image.is_file():
                raise RuntimeError(f"kernel build did not create {kernel_image}")

        if command == "build":
            _skip(result, "disk-prepare", "build-only")
            _skip(result, "pre-script", "build-only")
            _skip(result, "qemu", "build-only")
            _skip(result, "post-script", "build-only")
        else:
            with _stage(result, "disk-prepare"):
                if manifest["disk"]["mode"] in {"private-copy", "generated"}:
                    private_disk = Path(manifest["disk"]["runtime_path"])
                private_disk = _prepare_disk(manifest, repo_root, artifact_dir / "disk-prepare.log")

            pre_script = manifest["scripts"]["pre"]
            if pre_script:
                with _stage(result, "pre-script"):
                    _run_script(Path(pre_script), manifest, qemu_log, result_path, artifact_dir / "pre-script.log")
            else:
                _skip(result, "pre-script", "not configured")

            try:
                qemu_started = True
                with _stage(result, "qemu"):
                    _run_qemu(manifest, repo_root, qemu_log, qemu_outcome)
                    if qemu_outcome.timed_out:
                        raise PipelineTimeout(f"QEMU timed out after {manifest['timeout_seconds']} seconds")
            finally:
                post_script = manifest["scripts"]["post"]
                if qemu_started and post_script:
                    try:
                        with _stage(result, "post-script"):
                            _run_script(Path(post_script), manifest, qemu_log, result_path, artifact_dir / "post-script.log")
                    except Exception:
                        failure = True
                elif result["stages"]["post-script"]["status"] == "pending":
                    _skip(result, "post-script", "not configured" if qemu_started else "QEMU not started")
    except Exception as error:
        failure = True
        result["errors"].append(str(error))
        print(f"basic test failed: {error}", file=sys.stderr)
        _skip_pending_before_cleanup(result, "earlier stage failed")
    finally:
        cleanup_errors: list[str] = []
        cleanup = result["cleanup"]
        cleanup["process_group_reaped"] = qemu_outcome.process_group_reaped
        if qemu_started and manifest is not None:
            qmp_socket = Path(manifest["qemu"]["qmp_socket"])
            try:
                qmp_socket.unlink(missing_ok=True)
                cleanup["qmp_socket_removed"] = not qmp_socket.exists()
            except OSError as error:
                cleanup_errors.append(f"QMP socket cleanup failed: {error}")
                cleanup["qmp_socket_removed"] = False
        if private_disk is not None:
            try:
                private_disk.unlink(missing_ok=True)
                cleanup["private_disk_removed"] = not private_disk.exists()
            except OSError as error:
                cleanup_errors.append(f"private disk cleanup failed: {error}")
                cleanup["private_disk_removed"] = False
        else:
            cleanup["private_disk_removed"] = None
        if not qemu_outcome.process_group_reaped:
            cleanup_errors.append("QEMU process group was not reaped")
        if cleanup_errors:
            failure = True
            result["errors"].extend(cleanup_errors)

        if command == "run" and manifest is not None and result["stages"]["qemu"]["status"] in {"success", "failed"}:
            expectation_result = evaluate_expectations(manifest["expect"], qemu_log, qemu_outcome)
            result["expectations"] = expectation_result
            if not expectation_result["passed"]:
                expectation_failed = True
                result["errors"].append("one or more expectations failed")

        result["qemu"] = {
            "exit_code": qemu_outcome.exit_code,
            "timed_out": qemu_outcome.timed_out,
            "terminated_after_marker": qemu_outcome.terminated_after_marker,
            "terminated_after_stress_mem": qemu_outcome.terminated_after_stress_mem,
            "stress_mem": qemu_outcome.stress_mem,
            "timeout_diagnostics": qemu_outcome.timeout_diagnostics,
            "interaction": manifest["qemu"]["interaction"] if manifest else None,
            "stdin_steps": qemu_outcome.stdin_steps or [],
        }
        stage_failed = any(stage["status"] in {"failed", "timed_out"} for stage in result["stages"].values())
        failure = failure or stage_failed
        result["execution_status"] = "failed" if failure else "completed"
        if failure or command == "build" or result["purpose"] == "diagnostic":
            result["verdict"] = "inconclusive"
        elif expectation_failed:
            result["verdict"] = "failed"
        else:
            result["verdict"] = "passed"
        result["exit_code"] = 1 if failure or result["verdict"] == "failed" else 0
        result["ended_at"] = _now()
        result["duration_seconds"] = time.monotonic() - result.pop("_started_monotonic")
        cleanup_stage = result["stages"]["result-cleanup"]
        cleanup_stage["status"] = "failed" if cleanup_errors else "success"
        cleanup_stage["duration_seconds"] = 0.0
        cleanup["terminal_restored"] = qemu_outcome.terminal_restored
        _write_json(result_path, result)

    print(f"basic test result: {result_path}")
    return result["exit_code"]


def evaluate_expectations(expect: dict[str, Any], qemu_log: Path, outcome: QemuOutcome) -> dict[str, Any]:
    text = qemu_log.read_text(errors="replace")
    observed_text, _ = _observed_text(text)
    checks: list[dict[str, Any]] = []
    configured_process_exit = expect["process_exit"]
    if configured_process_exit is not None:
        checks.append({
            "kind": "process_exit",
            "expected": configured_process_exit,
            "actual": outcome.exit_code,
            "passed": outcome.exit_code == configured_process_exit,
        })
    configured_guest_exit = expect["guest_exit_status"]
    matches = GUEST_EXIT_RE.findall(observed_text)
    actual_guest_exit = int(matches[-1]) if matches else None
    if configured_guest_exit is not None:
        checks.append({
            "kind": "guest_exit_status",
            "expected": configured_guest_exit,
            "actual": actual_guest_exit,
            "passed": actual_guest_exit == configured_guest_exit,
        })
    for marker in expect["markers"]:
        count = observed_text.count(marker)
        checks.append({"kind": "marker", "marker": marker, "actual": count, "expected": ">=1", "passed": count >= 1})
    for marker in expect["forbidden_markers"]:
        count = observed_text.count(marker)
        checks.append({"kind": "forbidden_marker", "marker": marker, "actual": count, "expected": 0, "passed": count == 0})
    for marker_count in expect["marker_counts"]:
        count = observed_text.count(marker_count["marker"])
        if marker_count["exactly"] is not None:
            passed = count == marker_count["exactly"]
            expected: Any = marker_count["exactly"]
        else:
            passed = count >= marker_count["at_least"]
            expected = f">={marker_count['at_least']}"
        checks.append({"kind": "marker_count", "marker": marker_count["marker"], "actual": count, "expected": expected, "passed": passed})
    return {"passed": all(check["passed"] for check in checks), "guest_exit_status": actual_guest_exit, "checks": checks}


def _prepare_disk(manifest: dict[str, Any], repo_root: Path, log_path: Path) -> Path | None:
    disk = manifest["disk"]
    mode = disk["mode"]
    if mode == "none":
        log_path.write_text("disk mode: none\n")
        return None
    if mode in {"template-readonly", "private-copy"}:
        canonical = Path(disk["canonical_path"])
        metadata = Path(f"{canonical}.inputs.json")
        valid, detail = validate_template(canonical, metadata, repo_root)
        log_path.write_text(f"rootfs profile: {disk['profile']}\n{detail}\n")
        if not valid:
            raise RuntimeError(
                f"canonical rootfs template is missing or stale: {detail}; "
                "run 'make disk ROOTFS=canonical' first"
            )
        if mode == "private-copy":
            private = Path(disk["runtime_path"])
            shutil.copy2(canonical, private)
            return private
        return None
    if mode == "generated":
        private = Path(disk["runtime_path"])
        if disk["generator"] == "blank":
            with private.open("wb") as output:
                output.truncate(parse_size(disk["size"]))
            log_path.write_text(f"generated blank disk: {private}\n")
            return private
        raise RuntimeError(f"unimplemented disk generator: {disk['generator']}")
    if mode == "external":
        external = Path(disk["runtime_path"])
        if not external.is_file():
            raise RuntimeError(f"external disk not found: {external}")
        log_path.write_text(f"external disk: {external}\n")
        return None
    raise RuntimeError(f"unhandled disk mode: {mode}")


def _run_qemu(
    manifest: dict[str, Any],
    cwd: Path,
    log_path: Path,
    outcome: QemuOutcome,
) -> None:
    if manifest["qemu"]["interaction"] == "terminal":
        _run_qemu_terminal(manifest, cwd, log_path, outcome)
        return
    steps = [dict(step, sent=False) for step in manifest["qemu"]["stdin_steps"]]
    outcome.stdin_steps = []
    _prepare_qmp_socket(manifest)
    process = subprocess.Popen(
        manifest["qemu"]["command"],
        cwd=cwd,
        stdin=subprocess.PIPE if steps else subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        start_new_session=True,
        bufsize=0,
    )
    assert process.stdout is not None
    presentation = _NonTerminalConsolePresentation()
    selector = selectors.DefaultSelector()
    selector.register(process.stdout, selectors.EVENT_READ)
    deadline = time.monotonic() + manifest["timeout_seconds"]
    captured = bytearray()
    timed_out = False
    terminated_after_marker = False
    terminated_after_stress_mem = False
    stress_mem: dict[str, Any] | None = None
    reaped = True
    pipe_open = True
    try:
        with log_path.open("wb") as log:
            while pipe_open or process.poll() is None:
                remaining = deadline - time.monotonic()
                if remaining <= 0 and process.poll() is None:
                    if not timed_out:
                        outcome.timeout_diagnostics = _capture_timeout_diagnostics(manifest)
                    timed_out = True
                    reaped = _terminate_process_group(process)
                events = selector.select(max(0.0, min(0.1, remaining))) if pipe_open else []
                for key, _ in events:
                    chunk = os.read(key.fd, 65536)
                    if not chunk:
                        selector.unregister(process.stdout)
                        pipe_open = False
                        continue
                    captured.extend(chunk)
                    log.write(chunk)
                    log.flush()
                    presentation.write(chunk)

                visible = captured.decode(errors="replace")
                for step in steps:
                    if step["sent"]:
                        continue
                    if step["ready_marker"] in visible:
                        if process.stdin is None:
                            raise RuntimeError("stdin step configured without QEMU stdin")
                        process.stdin.write(step["payload"].encode())
                        process.stdin.flush()
                        step["sent"] = True
                    break
                if manifest["qemu"]["exit_policy"] == "marker" and not terminated_after_marker:
                    marker = manifest["qemu"]["exit_marker"]
                    if marker in visible:
                        terminated_after_marker = True
                        reaped = _terminate_process_group(process)
                if manifest["qemu"]["exit_policy"] == "stress-mem" and not terminated_after_stress_mem:
                    parsed = _parse_stress_mem(visible)
                    if parsed is not None:
                        _, stress_mem = parsed
                        terminated_after_stress_mem = True
                        reaped = _terminate_process_group(process)
                if process.poll() is not None and not pipe_open:
                    break
            if process.poll() is None:
                reaped = _terminate_process_group(process) and reaped
        try:
            exit_code = process.wait(timeout=1)
        except subprocess.TimeoutExpired:
            reaped = _terminate_process_group(process) and reaped
            exit_code = process.poll()
    finally:
        presentation.finish()
        selector.close()
        if process.poll() is None:
            reaped = _terminate_process_group(process) and reaped
        if process.stdin is not None:
            try:
                process.stdin.close()
            except BrokenPipeError:
                pass
        process.stdout.close()
    public_steps = [{"ready_marker": step["ready_marker"], "payload_length": len(step["payload"].encode()), "sent": step["sent"]} for step in steps]
    outcome.exit_code = exit_code
    outcome.timed_out = timed_out
    outcome.terminated_after_marker = terminated_after_marker
    outcome.terminated_after_stress_mem = terminated_after_stress_mem
    outcome.stress_mem = stress_mem
    outcome.process_group_reaped = reaped
    outcome.stdin_steps = public_steps
    for step in public_steps:
        if not step["sent"]:
            raise RuntimeError(f"QEMU exited before stdin marker: {step['ready_marker']!r}")
    if manifest["qemu"]["exit_policy"] == "stress-mem" and stress_mem is None:
        raise RuntimeError("QEMU exited before a complete stress_mem record")


def _run_qemu_terminal(
    manifest: dict[str, Any],
    cwd: Path,
    log_path: Path,
    outcome: QemuOutcome,
) -> None:
    input_fd = sys.stdin.fileno()
    saved_attributes = termios.tcgetattr(input_fd)
    master_fd, slave_fd = pty.openpty()
    process: subprocess.Popen[bytes] | None = None
    selector = selectors.DefaultSelector()
    deadline = time.monotonic() + manifest["timeout_seconds"]
    timed_out = False
    terminated_after_marker = False
    reaped = True
    captured = bytearray()
    output_open = True
    try:
        tty.setraw(input_fd)
        _prepare_qmp_socket(manifest)
        process = subprocess.Popen(
            manifest["qemu"]["command"],
            cwd=cwd,
            stdin=slave_fd,
            stdout=slave_fd,
            stderr=slave_fd,
            start_new_session=True,
            close_fds=True,
        )
        os.close(slave_fd)
        slave_fd = -1
        selector.register(master_fd, selectors.EVENT_READ, "qemu")
        selector.register(input_fd, selectors.EVENT_READ, "operator")
        with log_path.open("wb") as log:
            while output_open or process.poll() is None:
                remaining = deadline - time.monotonic()
                if remaining <= 0 and process.poll() is None:
                    if not timed_out:
                        outcome.timeout_diagnostics = _capture_timeout_diagnostics(manifest)
                    timed_out = True
                    reaped = _terminate_process_group(process)
                for key, _ in selector.select(max(0.0, min(0.1, remaining))):
                    if key.data == "operator":
                        data = os.read(input_fd, 65536)
                        if data:
                            try:
                                os.write(master_fd, data)
                            except OSError:
                                pass
                        continue
                    try:
                        chunk = os.read(master_fd, 65536)
                    except OSError:
                        chunk = b""
                    if not chunk:
                        selector.unregister(master_fd)
                        output_open = False
                        continue
                    captured.extend(chunk)
                    log.write(chunk)
                    log.flush()
                    _console_write(chunk)
                if manifest["qemu"]["exit_policy"] == "marker" and not terminated_after_marker:
                    marker = manifest["qemu"]["exit_marker"]
                    if marker in captured.decode(errors="replace"):
                        terminated_after_marker = True
                        reaped = _terminate_process_group(process)
                if manifest["qemu"]["exit_policy"] == "stress-mem" and not outcome.terminated_after_stress_mem:
                    parsed = _parse_stress_mem(captured.decode(errors="replace"))
                    if parsed is not None:
                        _, outcome.stress_mem = parsed
                        outcome.terminated_after_stress_mem = True
                        reaped = _terminate_process_group(process)
                if process.poll() is not None and not output_open:
                    break
            if process.poll() is None:
                reaped = _terminate_process_group(process) and reaped
        try:
            exit_code = process.wait(timeout=1)
        except subprocess.TimeoutExpired:
            reaped = _terminate_process_group(process) and reaped
            exit_code = process.poll()
        outcome.exit_code = exit_code
        outcome.timed_out = timed_out
        outcome.terminated_after_marker = terminated_after_marker
        outcome.process_group_reaped = reaped
        outcome.stdin_steps = []
        if manifest["qemu"]["exit_policy"] == "stress-mem" and outcome.stress_mem is None:
            raise RuntimeError("QEMU exited before a complete stress_mem record")
    finally:
        if process is not None and process.poll() is None:
            outcome.process_group_reaped = _terminate_process_group(process)
        selector.close()
        if slave_fd >= 0:
            os.close(slave_fd)
        os.close(master_fd)
        termios.tcsetattr(input_fd, termios.TCSADRAIN, saved_attributes)
        outcome.terminal_restored = True


def _terminate_process_group(process: subprocess.Popen[bytes]) -> bool:
    if process.poll() is not None:
        return True
    try:
        os.killpg(process.pid, signal.SIGTERM)
    except ProcessLookupError:
        return True
    try:
        process.wait(timeout=2)
        return True
    except subprocess.TimeoutExpired:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        try:
            process.wait(timeout=2)
            return True
        except subprocess.TimeoutExpired:
            return False


def _prepare_qmp_socket(manifest: dict[str, Any]) -> None:
    path = Path(manifest["qemu"]["qmp_socket"])
    try:
        path.unlink(missing_ok=True)
    except OSError as error:
        raise RuntimeError(f"cannot prepare QMP socket {path}: {error}") from error


def _capture_timeout_diagnostics(manifest: dict[str, Any]) -> dict[str, Any]:
    path = Path(manifest["qemu"]["timeout_diagnostics"])
    qmp_socket = Path(manifest["qemu"]["qmp_socket"])
    started = time.monotonic()
    diagnostic: dict[str, Any] = {
        "schema_version": 1,
        "test": manifest["test"],
        "captured_at": _now(),
        "kernel_image": manifest["kernel"]["image"],
        "qmp_socket": str(qmp_socket),
        "status": "failed",
        "queries": {},
        "errors": [],
    }
    try:
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
            client.settimeout(2.0)
            client.connect(str(qmp_socket))
            with client.makefile("rb") as stream:
                greeting = _qmp_read(stream)
                if "QMP" not in greeting:
                    raise RuntimeError("QMP greeting is missing the QMP object")
                diagnostic["greeting"] = greeting
                _qmp_execute(client, stream, "qmp_capabilities", command_id="capabilities")
                fixed_queries = (
                    ("status_before_stop", "query-status", None),
                    ("stop", "stop", None),
                    ("status_after_stop", "query-status", None),
                    ("cpus", "query-cpus-fast", None),
                    (
                        "registers",
                        "human-monitor-command",
                        {"command-line": "info registers -a"},
                    ),
                    (
                        "interrupts",
                        "human-monitor-command",
                        {"command-line": "info irq"},
                    ),
                    (
                        "interrupt_controllers",
                        "human-monitor-command",
                        {"command-line": "info pic"},
                    ),
                )
                for name, command, arguments in fixed_queries:
                    try:
                        diagnostic["queries"][name] = _qmp_execute(
                            client,
                            stream,
                            command,
                            arguments=arguments,
                            command_id=name,
                        )
                    except Exception as error:
                        diagnostic["errors"].append(f"{name}: {error}")
                diagnostic["status"] = "partial" if diagnostic["errors"] else "captured"
    except Exception as error:
        diagnostic["errors"].append(str(error))
    diagnostic["duration_seconds"] = round(time.monotonic() - started, 6)
    try:
        _write_json(path, diagnostic)
    except OSError as error:
        diagnostic["status"] = "failed"
        diagnostic["errors"].append(f"cannot write timeout diagnostics: {error}")
    return {
        "path": str(path),
        "status": diagnostic["status"],
        "errors": list(diagnostic["errors"]),
    }


def _qmp_read(stream: Any) -> dict[str, Any]:
    line = stream.readline()
    if not line:
        raise RuntimeError("QMP connection closed before a response")
    try:
        message = json.loads(line)
    except json.JSONDecodeError as error:
        raise RuntimeError(f"invalid QMP response: {error}") from error
    if not isinstance(message, dict):
        raise RuntimeError("QMP response is not an object")
    return message


def _qmp_execute(
    client: socket.socket,
    stream: Any,
    command: str,
    *,
    arguments: dict[str, Any] | None = None,
    command_id: str,
) -> Any:
    request: dict[str, Any] = {"execute": command, "id": command_id}
    if arguments is not None:
        request["arguments"] = arguments
    client.sendall((json.dumps(request, separators=(",", ":")) + "\r\n").encode())
    while True:
        response = _qmp_read(stream)
        if response.get("id") != command_id:
            continue
        if "error" in response:
            raise RuntimeError(f"QMP {command} failed: {response['error']}")
        if "return" not in response:
            raise RuntimeError(f"QMP {command} response has no return value")
        return response["return"]


def _run_script(script: Path, manifest: dict[str, Any], qemu_log: Path, result_path: Path, log_path: Path) -> None:
    disk_path = manifest["disk"].get("runtime_path") or ""
    env = {
        "PATH": os.environ.get("PATH", os.defpath),
        "TMPDIR": os.environ.get("TMPDIR", "/tmp"),
        "LC_ALL": "C",
        "LKM_TEST_NAME": manifest["test"],
        "LKM_TEST_CONFIG": manifest["config_path"],
        "LKM_TEST_KERNEL": manifest["kernel"]["image"],
        "LKM_TEST_DISK": disk_path,
        "LKM_TEST_QEMU_LOG": str(qemu_log),
        "LKM_TEST_RESULT": str(result_path),
        "LKM_TEST_ARTIFACTS": manifest["artifact_dir"],
    }
    _run_logged([str(script)], Path(manifest["artifact_dir"]), log_path, env=env)


def _run_logged(command: list[str], cwd: Path, log_path: Path, env: dict[str, str] | None = None) -> None:
    print("+ " + shlex.join(command), flush=True)
    with log_path.open("wb") as log:
        with subprocess.Popen(command, cwd=cwd, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT) as process:
            assert process.stdout is not None
            while chunk := process.stdout.read(65536):
                log.write(chunk)
                log.flush()
                _console_write(chunk)
            exit_code = process.wait()
    if exit_code != 0:
        raise RuntimeError(f"command exited with status {exit_code}: {shlex.join(command)}")


def _console_write(chunk: bytes) -> None:
    buffer = getattr(sys.stdout, "buffer", None)
    if buffer is not None:
        buffer.write(chunk)
        buffer.flush()
    else:
        sys.stdout.write(chunk.decode(errors="replace"))
        sys.stdout.flush()


def _parse_disk(
    raw: dict[str, Any],
    repo_root: Path,
    version: int,
    compatibility_mappings: list[str],
) -> dict[str, Any]:
    source_mode = _string(raw, "mode", "disk")
    mode = source_mode
    if version == 1 and mode == "canonical-readonly":
        mode = "template-readonly"
        compatibility_mappings.append("disk.canonical-readonly=template-readonly/canonical")
    if mode not in ALLOWED_DISK_MODES:
        raise ConfigError(f"disk.mode must be one of {sorted(ALLOWED_DISK_MODES)}")
    allowed_by_mode = {
        "none": {"mode"},
        "template-readonly": {"mode", "profile"},
        "private-copy": {"mode", "profile"},
        "generated": {"mode", "generator", "size"},
        "external": {"mode", "path", "readonly"},
    }
    allowed = allowed_by_mode[mode]
    if version == 1 and mode in {"template-readonly", "private-copy"}:
        allowed = allowed - {"profile"}
    _keys(raw, allowed, f"disk mode {source_mode}")
    result: dict[str, Any] = {"mode": mode}
    if mode in {"template-readonly", "private-copy"}:
        if version == 1:
            profile = "canonical"
            compatibility_mappings.append(f"disk.{source_mode}.profile=canonical")
        else:
            profile = _string(raw, "profile", "disk")
        if profile not in ALLOWED_ROOTFS_PROFILES:
            raise ConfigError(f"disk.profile must be one of {sorted(ALLOWED_ROOTFS_PROFILES)}")
        result["profile"] = profile
    elif mode == "generated":
        generator = _string(raw, "generator", "disk")
        if generator != "blank":
            raise ConfigError("disk.generator must be 'blank'")
        size = _string(raw, "size", "disk")
        try:
            parse_size(size)
        except ValueError as error:
            raise ConfigError(str(error)) from error
        result.update(generator=generator, size=size)
    elif mode == "external":
        path_text = _string(raw, "path", "disk")
        path = Path(path_text)
        path = path.resolve() if path.is_absolute() else (repo_root / path).resolve()
        if not path.is_file():
            raise ConfigError(f"disk.path does not exist: {path}")
        readonly = raw.get("readonly", True)
        if not isinstance(readonly, bool):
            raise ConfigError("disk.readonly must be a boolean")
        result.update(path=str(path), readonly=readonly)
    return result


def _stdin_steps(raw: Any) -> list[dict[str, str]]:
    if not isinstance(raw, list):
        raise ConfigError("qemu.stdin_steps must be an array of tables")
    result = []
    for index, step in enumerate(raw):
        _table(step, f"qemu.stdin_steps[{index}]")
        _keys(step, {"ready_marker", "payload"}, f"qemu.stdin_steps[{index}]")
        marker = _string(step, "ready_marker", f"qemu.stdin_steps[{index}]")
        payload = _string(step, "payload", f"qemu.stdin_steps[{index}]", allow_empty=True)
        if not marker:
            raise ConfigError(f"qemu.stdin_steps[{index}].ready_marker must not be empty")
        result.append({"ready_marker": marker, "payload": payload})
    return result


def _host_forwards(raw: Any) -> list[dict[str, Any]]:
    if not isinstance(raw, list):
        raise ConfigError("qemu.host_forwards must be an array of tables")
    result: list[dict[str, Any]] = []
    for index, forward in enumerate(raw):
        context = f"qemu.host_forwards[{index}]"
        _table(forward, context)
        _keys(forward, {"protocol", "host_address", "host_port", "guest_port"}, context)
        protocol = _string(forward, "protocol", context)
        if protocol not in {"tcp", "udp"}:
            raise ConfigError(f"{context}.protocol must be 'tcp' or 'udp'")
        host_address = forward.get("host_address", "")
        if not isinstance(host_address, str):
            raise ConfigError(f"{context}.host_address must be a string")
        host_port = _positive_integer(forward, "host_port", context)
        guest_port = _positive_integer(forward, "guest_port", context)
        if host_port > 65535 or guest_port > 65535:
            raise ConfigError(f"{context} ports must be <= 65535")
        result.append({
            "protocol": protocol,
            "host_address": host_address,
            "host_port": host_port,
            "guest_port": guest_port,
        })
    return result


def _observed_text(stdout: str) -> tuple[str, dict[str, Any] | None]:
    parsed = _parse_stress_mem(stdout)
    if parsed is None:
        return stdout, None
    decoded, metadata = parsed
    return stdout + "\n" + decoded, metadata


def _parse_stress_mem(stdout: str) -> tuple[str, dict[str, Any]] | None:
    normalized = ANSI_RE.sub("", stdout)
    for match in reversed(list(STRESS_MEM_HEADER_RE.finditer(normalized))):
        expected_bytes = int(match.group("bytes"))
        data_segment = STRESS_MEM_PROMPT_ECHO_RE.sub("", normalized[match.end() :])
        hex_data = _take_hex_payload(data_segment, expected_bytes * 2)
        if hex_data is None:
            continue
        try:
            decoded = bytes.fromhex(hex_data).decode("utf-8", errors="replace")
        except ValueError:
            continue
        return decoded, {
            "bytes": expected_bytes,
            "total": int(match.group("total")),
            "overflow": match.group("overflow") == "1",
            "dropped": int(match.group("dropped")),
        }
    return None


def _take_hex_payload(data_segment: str, expected_hex_len: int) -> str | None:
    if expected_hex_len == 0:
        return ""
    chars: list[str] = []
    for char in data_segment:
        if char in "0123456789abcdef":
            chars.append(char)
            if len(chars) == expected_hex_len:
                return "".join(chars)
            continue
        if char.isspace():
            continue
        return None
    return None


def _marker_counts(raw: Any) -> list[dict[str, Any]]:
    if not isinstance(raw, list):
        raise ConfigError("expect.marker_counts must be an array of tables")
    result = []
    for index, item in enumerate(raw):
        context = f"expect.marker_counts[{index}]"
        _table(item, context)
        _keys(item, {"marker", "at_least", "exactly"}, context)
        marker = _string(item, "marker", context)
        at_least = _optional_integer(item, "at_least", context)
        exactly = _optional_integer(item, "exactly", context)
        if (at_least is None) == (exactly is None):
            raise ConfigError(f"{context} must set exactly one of at_least or exactly")
        if at_least is not None and at_least <= 0:
            raise ConfigError(f"{context}.at_least must be positive")
        if exactly is not None and exactly < 0:
            raise ConfigError(f"{context}.exactly must be non-negative")
        result.append({"marker": marker, "at_least": at_least, "exactly": exactly})
    return result


def _script_path(raw: dict[str, Any], key: str, config_path: Path) -> str | None:
    value = raw.get(key)
    if value is None:
        return None
    if not isinstance(value, str) or not value:
        raise ConfigError(f"{key} must be a non-empty relative path")
    path = Path(value)
    if path.is_absolute():
        raise ConfigError(f"{key} must be relative to the configuration")
    resolved = (config_path.parent / path).resolve()
    if not resolved.is_file():
        raise ConfigError(f"{key} not found: {resolved}")
    if not os.access(resolved, os.X_OK):
        raise ConfigError(f"{key} is not executable: {resolved}")
    return str(resolved)


def _optional_repo_path(raw: dict[str, Any], key: str, repo_root: Path, context: str) -> Path | None:
    value = raw.get(key)
    if value is None:
        return None
    if not isinstance(value, str) or not value:
        raise ConfigError(f"{context}.{key} must be a non-empty path")
    path = Path(value)
    resolved = path.resolve() if path.is_absolute() else (repo_root / path).resolve()
    if not resolved.is_file():
        raise ConfigError(f"{context}.{key} not found: {resolved}")
    return resolved


def _canonical_image(repo_root: Path) -> Path:
    configured = os.environ.get("CANONICAL_ROOTFS_IMAGE")
    if configured:
        path = Path(configured)
        return path.resolve() if path.is_absolute() else (repo_root / path).resolve()
    return (repo_root / "impl" / "arceos_ex" / "build" / "rootfs" / "canonical.raw").resolve()


def _linux_provider_dir(repo_root: Path) -> Path:
    configured = os.environ.get("LINUX_PROVIDER_DIR")
    if configured:
        path = Path(configured)
        return path.resolve() if path.is_absolute() else (repo_root / path).resolve()
    return (repo_root.parent / "linux-6.12").resolve()


def _tool_command(variable: str, default: str) -> list[str]:
    command = shlex.split(os.environ.get(variable, default))
    if not command:
        raise ConfigError(f"{variable} tool command must not be empty")
    return command


def _new_artifact_dir(repo_root: Path, out_root: Path, output_dir: Path | None, test: str) -> Path:
    if output_dir is not None:
        path = output_dir.resolve() if output_dir.is_absolute() else (repo_root / output_dir).resolve()
        path.mkdir(parents=True, exist_ok=False)
        return path
    root = out_root.resolve() if out_root.is_absolute() else (repo_root / out_root).resolve()
    root.mkdir(parents=True, exist_ok=True)
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S.%fZ")
    path = root / f"{stamp}-{test}-{os.getpid()}"
    path.mkdir()
    return path


def _manifest_artifact_dir(args: argparse.Namespace, repo_root: Path, test: str) -> Path:
    if args.output_dir:
        return args.output_dir.resolve() if args.output_dir.is_absolute() else (repo_root / args.output_dir).resolve()
    return (repo_root / "impl" / "arceos_ex" / "tests" / "basic" / "out" / f"manifest-{test}").resolve()


def _record_early_config_failure(
    *,
    args: argparse.Namespace,
    repo_root: Path,
    requested_test: str,
    canonical_test: str,
    compatibility_alias: str | None,
    error: ConfigError,
) -> int:
    artifact_dir = _new_artifact_dir(
        repo_root,
        args.out_root,
        args.output_dir,
        "config-error",
    )
    qemu_log = artifact_dir / "qemu.log"
    qemu_log.touch()
    result_path = artifact_dir / "result.json"
    manifest_path = artifact_dir / "manifest.json"
    config_path = artifact_dir / "unresolved-config.toml"
    result = _initial_result(
        requested_test,
        canonical_test,
        compatibility_alias,
        args.command,
        config_path,
        artifact_dir,
        manifest_path,
        qemu_log,
        result_path,
    )
    result["stages"]["config"].update(
        status="failed",
        duration_seconds=0.0,
        error=str(error),
    )
    _skip_pending_before_cleanup(result, "configuration could not be resolved")
    result["stages"]["result-cleanup"].update(status="success", duration_seconds=0.0)
    result["errors"].append(str(error))
    result["execution_status"] = "failed"
    result["verdict"] = "inconclusive"
    result["exit_code"] = 1
    result["ended_at"] = _now()
    result["duration_seconds"] = time.monotonic() - result.pop("_started_monotonic")
    _write_json(result_path, result)
    print(f"basic test result: {result_path}")
    return 1


def _initial_result(
    requested_test: str,
    test: str,
    compatibility_alias: str | None,
    command: str,
    config: Path,
    artifact: Path,
    manifest: Path,
    qemu_log: Path,
    result: Path,
) -> dict[str, Any]:
    return {
        "schema_version": RESULT_SCHEMA_VERSION,
        "request": {
            "test": requested_test,
            "canonical_test": test,
            "compatibility_alias": compatibility_alias,
        },
        "command": command,
        "test": test,
        "purpose": None,
        "config": str(config.resolve()),
        "manifest": None,
        "artifacts": {
            "directory": str(artifact),
            "manifest": str(manifest),
            "qemu_log": str(qemu_log),
            "timeout_diagnostics": str(artifact / "qemu-timeout-diagnostics.json"),
            "result": str(result),
        },
        "started_at": _now(),
        "ended_at": None,
        "duration_seconds": None,
        "_started_monotonic": time.monotonic(),
        "execution_status": None,
        "verdict": None,
        "exit_code": None,
        "stages": {name: {"status": "pending", "duration_seconds": None} for name in STAGES},
        "qemu": {},
        "expectations": {"passed": False, "checks": []},
        "cleanup": {
            "process_group_reaped": True,
            "private_disk_removed": None,
            "qmp_socket_removed": None,
            "terminal_restored": None,
        },
        "errors": [],
    }


@contextmanager
def _stage(result: dict[str, Any], name: str) -> Iterator[None]:
    stage = result["stages"][name]
    stage["status"] = "running"
    started = time.monotonic()
    try:
        yield
    except Exception as error:
        stage["status"] = "timed_out" if isinstance(error, PipelineTimeout) else "failed"
        stage["error"] = str(error)
        raise
    else:
        stage["status"] = "success"
    finally:
        stage["duration_seconds"] = time.monotonic() - started


def _skip(result: dict[str, Any], name: str, reason: str) -> None:
    stage = result["stages"][name]
    if stage["status"] == "pending":
        stage.update(status="skipped", duration_seconds=0.0, reason=reason)


def _skip_pending_before_cleanup(result: dict[str, Any], reason: str) -> None:
    for name in STAGES:
        if name != "result-cleanup":
            _skip(result, name, reason)


def _write_json(path: Path, data: dict[str, Any]) -> None:
    temporary = path.with_name(f".{path.name}.tmp-{os.getpid()}")
    temporary.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")
    temporary.replace(path)


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


def _terminal_available() -> bool:
    return sys.stdin.isatty() and sys.stdout.isatty()


def _keys(table: dict[str, Any], allowed: set[str], context: str) -> None:
    unknown = sorted(set(table) - allowed)
    if unknown:
        raise ConfigError(f"unknown {context} field(s): {', '.join(unknown)}")


def _table(value: Any, context: str) -> None:
    if not isinstance(value, dict):
        raise ConfigError(f"{context} must be a table")


def _required_table(table: dict[str, Any], key: str, context: str) -> dict[str, Any]:
    value = table.get(key)
    if not isinstance(value, dict):
        raise ConfigError(f"{context}.{key} must be a table")
    return value


def _string(table: dict[str, Any], key: str, context: str, *, allow_empty: bool = False) -> str:
    value = table.get(key)
    if not isinstance(value, str) or (not allow_empty and not value):
        raise ConfigError(f"{context}.{key} must be a {'string' if allow_empty else 'non-empty string'}")
    return value


def _string_list(value: Any, context: str) -> list[str]:
    if not isinstance(value, list) or any(not isinstance(item, str) or not item for item in value):
        raise ConfigError(f"{context} must be an array of non-empty strings")
    return list(value)


def _is_integer(value: Any) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def _integer(table: dict[str, Any], key: str, context: str) -> int:
    value = table.get(key)
    if not _is_integer(value):
        raise ConfigError(f"{context}.{key} must be an integer")
    return value


def _positive_integer(table: dict[str, Any], key: str, context: str) -> int:
    value = _integer(table, key, context)
    if value <= 0:
        raise ConfigError(f"{context}.{key} must be positive")
    return value


def _optional_integer(table: dict[str, Any], key: str, context: str) -> int | None:
    value = table.get(key)
    if value is None:
        return None
    if not _is_integer(value):
        raise ConfigError(f"{context}.{key} must be an integer")
    return value


def _boolean(table: dict[str, Any], key: str, context: str) -> bool:
    value = table.get(key)
    if not isinstance(value, bool):
        raise ConfigError(f"{context}.{key} must be a boolean")
    return value


if __name__ == "__main__":
    raise SystemExit(main())
