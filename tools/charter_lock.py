#!/usr/bin/env python3
"""Manage task-scoped AI protection locks for charter files."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import sys
import tempfile
from typing import Any


MANIFEST_NAME = "charter-locks.json"
MANIFEST_VERSION = 1
LOCK_NOTICE = (
    "> **AI 保护锁：已经锁定；未经用户明确解锁，AI 只能提出建议，不得直接修改；"
    "授权修改完成后必须重新锁定。**"
)
SHA256_RE = re.compile(r"[0-9a-f]{64}")


class LockError(RuntimeError):
    """A manifest or lock-state error safe to show to the caller."""


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def has_lock_notice(path: Path) -> bool:
    with path.open("rb") as source:
        first_line = source.readline().rstrip(b"\r\n")
    return first_line == LOCK_NOTICE.encode("utf-8")


def validate_relative_path(value: Any) -> str:
    if not isinstance(value, str) or not value:
        raise LockError("lock path must be a non-empty string")
    if "\\" in value:
        raise LockError(f"lock path must use POSIX separators: {value!r}")

    path = PurePosixPath(value)
    if path.is_absolute() or value != path.as_posix():
        raise LockError(f"lock path must be a canonical repository-relative path: {value!r}")
    if any(part in ("", ".", "..") for part in path.parts):
        raise LockError(f"lock path contains an invalid component: {value!r}")
    if len(path.parts) < 3 or path.parts[:2] != ("spec", "charter"):
        raise LockError(f"lock path must be under spec/charter: {value!r}")
    if path.suffix != ".md":
        raise LockError(f"lock target must be a Markdown charter: {value!r}")
    return value


def load_manifest(root: Path) -> dict[str, Any]:
    manifest_path = root / MANIFEST_NAME
    try:
        manifest_stat = manifest_path.lstat()
    except FileNotFoundError as error:
        raise LockError(f"missing root manifest: {MANIFEST_NAME}") from error
    if stat.S_ISLNK(manifest_stat.st_mode) or not stat.S_ISREG(manifest_stat.st_mode):
        raise LockError(f"root manifest must be a regular file: {MANIFEST_NAME}")

    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise LockError(f"cannot read valid JSON from {MANIFEST_NAME}: {error}") from error

    if not isinstance(manifest, dict) or set(manifest) != {"version", "locks"}:
        raise LockError("manifest must contain exactly 'version' and 'locks'")
    if manifest["version"] != MANIFEST_VERSION:
        raise LockError(
            f"unsupported manifest version {manifest['version']!r}; expected {MANIFEST_VERSION}"
        )
    if not isinstance(manifest["locks"], list) or not manifest["locks"]:
        raise LockError("manifest 'locks' must be a non-empty list")

    seen: set[str] = set()
    for index, entry in enumerate(manifest["locks"]):
        prefix = f"manifest lock entry {index}"
        if not isinstance(entry, dict) or set(entry) != {"path", "state", "sha256"}:
            raise LockError(f"{prefix} must contain exactly path, state, and sha256")
        path = validate_relative_path(entry["path"])
        if path in seen:
            raise LockError(f"duplicate lock path: {path}")
        seen.add(path)
        if entry["state"] not in ("locked", "unlocked"):
            raise LockError(f"{prefix} has invalid state: {entry['state']!r}")
        if not isinstance(entry["sha256"], str) or not SHA256_RE.fullmatch(entry["sha256"]):
            raise LockError(f"{prefix} has an invalid lowercase SHA-256 digest")

    return manifest


def target_path(root: Path, relative_path: str) -> Path:
    root = root.resolve(strict=True)
    target = root.joinpath(*PurePosixPath(relative_path).parts)
    try:
        target_stat = target.lstat()
    except FileNotFoundError as error:
        raise LockError(f"lock target does not exist: {relative_path}") from error
    if stat.S_ISLNK(target_stat.st_mode) or not stat.S_ISREG(target_stat.st_mode):
        raise LockError(f"lock target must be a regular file, not a symlink: {relative_path}")
    if target.resolve(strict=True) != target:
        raise LockError(f"lock target path must not traverse symlinks: {relative_path}")
    return target


def find_entry(manifest: dict[str, Any], requested_path: str) -> dict[str, str]:
    requested_path = validate_relative_path(requested_path)
    for entry in manifest["locks"]:
        if entry["path"] == requested_path:
            return entry
    raise LockError(f"path is not in {MANIFEST_NAME}: {requested_path}")


def write_manifest(root: Path, manifest: dict[str, Any]) -> None:
    manifest_path = root / MANIFEST_NAME
    old_mode = stat.S_IMODE(manifest_path.stat().st_mode)
    descriptor, temporary_name = tempfile.mkstemp(prefix=f".{MANIFEST_NAME}.", dir=root)
    temporary_path = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8", newline="\n") as output:
            json.dump(manifest, output, ensure_ascii=False, indent=2)
            output.write("\n")
            output.flush()
            os.fsync(output.fileno())
        temporary_path.chmod(old_mode)
        os.replace(temporary_path, manifest_path)
    finally:
        try:
            temporary_path.unlink()
        except FileNotFoundError:
            pass


def inspect_entry(root: Path, entry: dict[str, str]) -> dict[str, Any]:
    path = target_path(root, entry["path"])
    mode = stat.S_IMODE(path.stat().st_mode)
    return {
        "path": path,
        "mode": mode,
        "writable": bool(mode & 0o222),
        "notice": has_lock_notice(path),
        "actual_sha256": sha256_file(path),
    }


def integrity_errors(entry: dict[str, str], observed: dict[str, Any]) -> list[str]:
    errors = []
    if not observed["notice"]:
        errors.append(f"{entry['path']}: required first-line lock notice is missing")
    if observed["actual_sha256"] != entry["sha256"]:
        errors.append(
            f"{entry['path']}: SHA-256 drift "
            f"(expected {entry['sha256']}, actual {observed['actual_sha256']})"
        )
    return errors


def command_status(root: Path, manifest: dict[str, Any]) -> int:
    healthy = True
    for entry in manifest["locks"]:
        observed = inspect_entry(root, entry)
        entry_healthy = (
            entry["state"] == "locked"
            and not integrity_errors(entry, observed)
            and not observed["writable"]
        )
        healthy = healthy and entry_healthy
        print(
            f"{entry['path']}: state={entry['state']} "
            f"expected_sha256={entry['sha256']} actual_sha256={observed['actual_sha256']} "
            f"notice={'present' if observed['notice'] else 'missing'} "
            f"mode={observed['mode']:04o} writable={'yes' if observed['writable'] else 'no'}"
        )
    return 0 if healthy else 1


def command_check(root: Path, manifest: dict[str, Any]) -> int:
    errors = []
    for entry in manifest["locks"]:
        observed = inspect_entry(root, entry)
        if entry["state"] != "locked":
            errors.append(f"{entry['path']}: state is {entry['state']}, expected locked")
        errors.extend(integrity_errors(entry, observed))
        if observed["writable"]:
            errors.append(
                f"{entry['path']}: write permission remains in mode {observed['mode']:04o}"
            )
    if errors:
        for error in errors:
            print(f"charter-lock check: {error}", file=sys.stderr)
        return 1
    print(f"charter-lock check: {len(manifest['locks'])} locked charter(s) valid")
    return 0


def command_enforce(root: Path, manifest: dict[str, Any]) -> int:
    locked_entries = []
    errors = []
    for entry in manifest["locks"]:
        if entry["state"] != "locked":
            print(f"charter-lock enforce: skipped unlocked {entry['path']}")
            continue
        observed = inspect_entry(root, entry)
        errors.extend(integrity_errors(entry, observed))
        locked_entries.append((entry, observed))

    if errors:
        for error in errors:
            print(f"charter-lock enforce: {error}", file=sys.stderr)
        return 1

    changed = 0
    for entry, observed in locked_entries:
        if observed["writable"]:
            observed["path"].chmod(observed["mode"] & ~0o222)
            changed += 1
            print(f"charter-lock enforce: restored read-only mode for {entry['path']}")
    print(f"charter-lock enforce: checked={len(locked_entries)} restored={changed}")
    return 0


def command_unlock(
    root: Path, manifest: dict[str, Any], requested_path: str
) -> int:
    entry = find_entry(manifest, requested_path)
    if entry["state"] != "locked":
        raise LockError(f"cannot unlock {requested_path}: state is {entry['state']}")
    observed = inspect_entry(root, entry)
    errors = integrity_errors(entry, observed)
    if errors:
        raise LockError("; ".join(errors))

    entry["state"] = "unlocked"
    write_manifest(root, manifest)
    observed["path"].chmod(observed["mode"] | stat.S_IWUSR)
    mode = stat.S_IMODE(observed["path"].stat().st_mode)
    print(f"unlocked {requested_path} mode={mode:04o}; relock before completing the task")
    return 0


def command_lock(root: Path, manifest: dict[str, Any], requested_path: str) -> int:
    entry = find_entry(manifest, requested_path)
    if entry["state"] != "unlocked":
        raise LockError(f"cannot lock {requested_path}: state is {entry['state']}, expected unlocked")
    path = target_path(root, entry["path"])
    if not has_lock_notice(path):
        raise LockError(f"{requested_path}: required first-line lock notice is missing")

    mode = stat.S_IMODE(path.stat().st_mode)
    path.chmod(mode & ~0o222)
    digest = sha256_file(path)
    entry["sha256"] = digest
    entry["state"] = "locked"
    write_manifest(root, manifest)
    final_mode = stat.S_IMODE(path.stat().st_mode)
    print(f"locked {requested_path} sha256={digest} mode={final_mode:04o}")
    return 0


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parent.parent,
        help=argparse.SUPPRESS,
    )
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("status", help="report lock state, content, notice, and mode")
    subparsers.add_parser("enforce", help="restore read-only mode for valid locked files")
    subparsers.add_parser("check", help="fail unless every manifest entry is securely locked")
    for command in ("unlock", "lock"):
        command_parser = subparsers.add_parser(command)
        command_parser.add_argument("path")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    try:
        root = args.root.resolve(strict=True)
        manifest = load_manifest(root)
        if args.command == "status":
            return command_status(root, manifest)
        if args.command == "check":
            return command_check(root, manifest)
        if args.command == "enforce":
            return command_enforce(root, manifest)
        if args.command == "unlock":
            return command_unlock(root, manifest, args.path)
        if args.command == "lock":
            return command_lock(root, manifest, args.path)
        raise AssertionError(f"unhandled command: {args.command}")
    except (LockError, OSError) as error:
        print(f"charter-lock: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
