"""Canonical JSON I/O and strict tools2 protocol validation."""

from __future__ import annotations

import hashlib
import json
import os
import tempfile
from pathlib import Path
from typing import Any

from .schemas import PRODUCER


_REPOSITORY_ROOT = Path(__file__).resolve().parents[4]


class ProtocolError(ValueError):
    """An intermediate file is not owned by the expected tools2 protocol."""


def normalize_signal_request(value: str, *, option: str = "Signal") -> str:
    """Return the canonical Target.SignalName used by all tools2 entry points."""
    if "." not in value or value.startswith(".") or value.endswith("."):
        raise ValueError(f"{option} must be Target.SignalName")
    target, name = value.rsplit(".", 1)
    if name == "Startup":
        name = "Preset"
    return f"{target}.{name}"


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode(
        "utf-8"
    )


def fingerprint(value: Any) -> str:
    return "sha256:" + hashlib.sha256(canonical_bytes(value)).hexdigest()


def stable_source_path(path: str | Path) -> str:
    """Return a checkout-independent display path for repository sources."""
    resolved = Path(path).resolve()
    try:
        return resolved.relative_to(_REPOSITORY_ROOT).as_posix()
    except ValueError:
        return str(resolved)


def read_json(path: str | Path) -> dict[str, Any]:
    with Path(path).open("r", encoding="utf-8") as stream:
        value = json.load(stream)
    if not isinstance(value, dict):
        raise ProtocolError(f"expected a JSON object: {path}")
    return value


def write_json(path: str | Path, value: Any) -> None:
    target = Path(path)
    target.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix=f".{target.name}.", dir=target.parent)
    try:
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(canonical_bytes(value))
        os.replace(temporary, target)
    except BaseException:
        try:
            os.unlink(temporary)
        except FileNotFoundError:
            pass
        raise


def require_protocol(
    value: dict[str, Any], *, schema: str, version: int, label: str
) -> dict[str, Any]:
    actual = (value.get("schema"), value.get("version"), value.get("producer"))
    expected = (schema, version, PRODUCER)
    if actual != expected:
        raise ProtocolError(
            f"{label} protocol mismatch: expected schema={schema!r} version={version} "
            f"producer={PRODUCER!r}, got schema={actual[0]!r} version={actual[1]!r} "
            f"producer={actual[2]!r}"
        )
    return value
