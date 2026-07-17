#!/usr/bin/env python3
"""Build the input-sensitive canonical Alpine rootfs image."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import shlex
import shutil
import stat
import subprocess
import sys
import tempfile
from typing import Any, Iterable


SCHEMA_VERSION = 1
FIXTURES = (
    ("user-smoke", "user_smoke", "musl", "dynamic"),
    ("init-hello", "init_hello", "gnu", "static"),
    ("fileio-nolibc", "fileio_nolibc", "gnu", "static"),
    ("unsupported-syscall-nolibc", "unsupported_syscall_nolibc", "gnu", "static"),
)


def main(argv: list[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        build_canonical_rootfs(args)
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"canonical rootfs failed: {error}", file=sys.stderr)
        return 1
    return 0


def _parser() -> argparse.ArgumentParser:
    repo_root = Path(__file__).resolve().parents[4]
    kernel_dir = repo_root / "impl" / "arceos_ex"
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=repo_root)
    parser.add_argument("--image", type=Path, required=True)
    parser.add_argument("--size", default="320M")
    parser.add_argument("--tarball", type=Path, required=True)
    parser.add_argument("--url", required=True)
    parser.add_argument("--ltp-dir", type=Path, required=True)
    parser.add_argument("--wget", default="wget")
    parser.add_argument("--tar", default="tar")
    parser.add_argument("--mkfs-ext2", default="mkfs.ext2")
    parser.add_argument("--mkfs-feature-args", default="-O ^dir_index")
    parser.add_argument("--ext2-block-size", default="")
    parser.add_argument("--gnu-cc", default="riscv64-linux-gnu-gcc")
    parser.add_argument("--musl-cc", default="riscv64-linux-musl-gcc")
    parser.add_argument("--make", dest="make_command", default="make")
    parser.add_argument("--force", action="store_true")
    parser.set_defaults(kernel_dir=kernel_dir)
    return parser


def build_canonical_rootfs(args: argparse.Namespace) -> None:
    repo_root = args.repo_root.resolve()
    kernel_dir = repo_root / "impl" / "arceos_ex"
    user_dir = kernel_dir / "tests" / "user"
    image = _resolve(repo_root, args.image)
    tarball = _resolve(repo_root, args.tarball)
    ltp_dir = _resolve(repo_root, args.ltp_dir)
    metadata_path = Path(f"{image}.inputs.json")

    _ensure_tarball(tarball, args.url, args.wget)
    _validate_ltp(ltp_dir)
    manifest = input_manifest(
        repo_root=repo_root,
        tarball=tarball,
        ltp_dir=ltp_dir,
        user_dir=user_dir,
        size=args.size,
        mkfs_feature_args=args.mkfs_feature_args,
        ext2_block_size=args.ext2_block_size,
        gnu_cc=args.gnu_cc,
        musl_cc=args.musl_cc,
    )
    manifest["fingerprint"] = manifest_fingerprint(manifest)

    if not should_rebuild(image, metadata_path, manifest["fingerprint"], args.force):
        print(f"canonical rootfs reused: {image}")
        return

    image.parent.mkdir(parents=True, exist_ok=True)
    fixtures_dir = kernel_dir / "build" / "rootfs" / "fixtures"
    _build_fixtures(
        user_dir=user_dir,
        fixtures_dir=fixtures_dir,
        make_command=args.make_command,
        gnu_cc=args.gnu_cc,
        musl_cc=args.musl_cc,
    )

    staging = Path(tempfile.mkdtemp(prefix="canonical-staging-", dir=image.parent))
    image_tmp = image.with_name(f".{image.name}.tmp-{os.getpid()}")
    metadata_tmp = metadata_path.with_name(f".{metadata_path.name}.tmp-{os.getpid()}")
    try:
        _run([*shlex.split(args.tar), "--no-same-owner", "-xzf", str(tarball), "-C", str(staging)])
        _validate_distribution_root(staging)
        destination = staging / "opt" / "lkm" / "tests"
        destination.mkdir(parents=True, exist_ok=True)
        for installed_name, _, _, _ in FIXTURES:
            shutil.copy2(fixtures_dir / installed_name, destination / installed_name)
            (destination / installed_name).chmod(0o755)

        ltp_source = ltp_dir / "opt" / "ltp"
        ltp_destination = staging / "opt" / "ltp"
        if ltp_destination.exists() or ltp_destination.is_symlink():
            if ltp_destination.is_dir() and not ltp_destination.is_symlink():
                shutil.rmtree(ltp_destination)
            else:
                ltp_destination.unlink()
        shutil.copytree(ltp_source, ltp_destination, symlinks=True)
        _validate_distribution_root(staging)

        with image_tmp.open("wb") as image_file:
            image_file.truncate(parse_size(args.size))
        mkfs_command = [*shlex.split(args.mkfs_ext2), "-q", "-F"]
        mkfs_command.extend(shlex.split(args.mkfs_feature_args))
        if args.ext2_block_size:
            mkfs_command.extend(["-b", args.ext2_block_size])
        mkfs_command.extend(["-d", str(staging), str(image_tmp)])
        _run(mkfs_command)

        metadata_tmp.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
        image_tmp.replace(image)
        metadata_tmp.replace(metadata_path)
    finally:
        shutil.rmtree(staging, ignore_errors=True)
        image_tmp.unlink(missing_ok=True)
        metadata_tmp.unlink(missing_ok=True)
    print(f"canonical rootfs rebuilt: {image}")


def input_manifest(
    *,
    repo_root: Path,
    tarball: Path,
    ltp_dir: Path,
    user_dir: Path,
    size: str,
    mkfs_feature_args: str,
    ext2_block_size: str,
    gnu_cc: str,
    musl_cc: str,
) -> dict[str, Any]:
    return {
        "schema_version": SCHEMA_VERSION,
        "tarball": _file_signature(tarball, content=True),
        "ltp": tree_signature(ltp_dir / "opt" / "ltp", content=True),
        "fixtures": tree_signature(user_dir, content=True),
        "builder": _file_signature(Path(__file__).resolve(), content=True),
        "settings": {
            "repo_root": str(repo_root),
            "size": size,
            "mkfs_feature_args": mkfs_feature_args,
            "ext2_block_size": ext2_block_size,
            "gnu_cc": command_signature(gnu_cc),
            "musl_cc": command_signature(musl_cc),
            "fixtures": [list(fixture) for fixture in FIXTURES],
        },
    }


def manifest_fingerprint(manifest: dict[str, Any]) -> str:
    encoded = json.dumps(manifest, sort_keys=True, separators=(",", ":")).encode()
    return hashlib.sha256(encoded).hexdigest()


def should_rebuild(image: Path, metadata: Path, fingerprint: str, force: bool) -> bool:
    if force or not image.is_file() or not metadata.is_file():
        return True
    try:
        recorded = json.loads(metadata.read_text())
    except (OSError, json.JSONDecodeError):
        return True
    return recorded.get("fingerprint") != fingerprint


def tree_signature(root: Path, *, content: bool) -> dict[str, Any]:
    if not root.is_dir():
        raise ValueError(f"input tree not found: {root}")
    digest = hashlib.sha256()
    entries = 0
    for path in _walk_tree(root):
        relative = path.relative_to(root).as_posix()
        info = path.lstat()
        mode = stat.S_IFMT(info.st_mode) | stat.S_IMODE(info.st_mode)
        record = f"{relative}\0{mode:o}\0{info.st_size}\0{info.st_mtime_ns}\0".encode()
        digest.update(record)
        if path.is_symlink():
            digest.update(os.readlink(path).encode())
        elif content and path.is_file():
            _hash_file_into(digest, path)
        entries += 1
    return {"path": str(root), "entries": entries, "sha256": digest.hexdigest()}


def command_signature(command: str) -> dict[str, Any]:
    words = shlex.split(command)
    if not words:
        raise ValueError("tool command must not be empty")
    executable = shutil.which(words[0])
    signature: dict[str, Any] = {"command": words, "executable": executable}
    if executable:
        signature["file"] = _file_signature(Path(executable), content=False)
    return signature


def parse_size(value: str) -> int:
    if not value:
        raise ValueError("image size must not be empty")
    suffixes = {"K": 1024, "M": 1024**2, "G": 1024**3}
    suffix = value[-1].upper()
    if suffix in suffixes:
        number = value[:-1]
        multiplier = suffixes[suffix]
    else:
        number = value
        multiplier = 1
    if not number.isdigit() or int(number) <= 0:
        raise ValueError(f"invalid image size: {value}")
    return int(number) * multiplier


def _build_fixtures(
    *,
    user_dir: Path,
    fixtures_dir: Path,
    make_command: str,
    gnu_cc: str,
    musl_cc: str,
) -> None:
    fixtures_dir.mkdir(parents=True, exist_ok=True)
    for installed_name, test_name, toolchain, link in FIXTURES:
        output = fixtures_dir / installed_name
        command = [
            *shlex.split(make_command),
            "-B",
            "-C",
            str(user_dir),
            "build",
            f"USER_TEST={test_name}",
            f"USER_TEST_OUT={output}",
            f"USER_TEST_TOOLCHAIN={toolchain}",
            f"USER_TEST_LINK={link}",
            f"GNU_CC={gnu_cc}",
            f"MUSL_CC={musl_cc}",
        ]
        _run(command)
        if not output.is_file():
            raise ValueError(f"fixture build did not create {output}")


def _ensure_tarball(tarball: Path, url: str, wget: str) -> None:
    if tarball.is_file():
        return
    tarball.parent.mkdir(parents=True, exist_ok=True)
    temporary = tarball.with_name(f".{tarball.name}.tmp-{os.getpid()}")
    try:
        _run([*shlex.split(wget), "-O", str(temporary), url])
        temporary.replace(tarball)
    finally:
        temporary.unlink(missing_ok=True)


def _validate_ltp(ltp_dir: Path) -> None:
    runner = ltp_dir / "opt" / "ltp" / "run-syscalls.sh"
    if not ltp_dir.is_dir():
        raise ValueError(f"LTP rootfs staging directory not found: {ltp_dir}")
    if not runner.is_file():
        raise ValueError(f"LTP rootfs staging is invalid: missing {runner}")
    if not os.access(runner, os.X_OK):
        raise ValueError(f"LTP rootfs staging is invalid: not executable: {runner}")


def _validate_distribution_root(staging: Path) -> None:
    init = staging / "sbin" / "init"
    if not init.is_file() and not init.is_symlink():
        raise ValueError("Alpine rootfs is invalid: missing /sbin/init")
    if init.is_symlink() and not (staging / os.readlink(init).lstrip("/")).is_file():
        raise ValueError(f"Alpine rootfs is invalid: broken /sbin/init link: {os.readlink(init)}")
    if not (staging / "etc").is_dir():
        raise ValueError("Alpine rootfs is invalid: missing /etc")


def _file_signature(path: Path, *, content: bool) -> dict[str, Any]:
    info = path.stat()
    result: dict[str, Any] = {
        "path": str(path),
        "size": info.st_size,
        "mtime_ns": info.st_mtime_ns,
        "mode": stat.S_IMODE(info.st_mode),
    }
    if content:
        digest = hashlib.sha256()
        _hash_file_into(digest, path)
        result["sha256"] = digest.hexdigest()
    return result


def _hash_file_into(digest: Any, path: Path) -> None:
    with path.open("rb") as source:
        while chunk := source.read(1024 * 1024):
            digest.update(chunk)


def _walk_tree(root: Path) -> Iterable[Path]:
    for directory, names, filenames in os.walk(root, topdown=True, followlinks=False):
        names.sort()
        filenames.sort()
        directory_path = Path(directory)
        symlink_dirs = [name for name in names if (directory_path / name).is_symlink()]
        for name in names:
            yield directory_path / name
        names[:] = [name for name in names if name not in symlink_dirs]
        for name in filenames:
            yield directory_path / name


def _resolve(base: Path, path: Path) -> Path:
    return path.resolve() if path.is_absolute() else (base / path).resolve()


def _run(command: list[str]) -> None:
    print("+ " + shlex.join(command), flush=True)
    subprocess.run(command, check=True)


if __name__ == "__main__":
    raise SystemExit(main())
