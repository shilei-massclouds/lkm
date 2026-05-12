"""Check JSON generation and default verification policy."""

from __future__ import annotations

from typing import Any

from common import CHECK_SCHEMA, CHECK_VERSION, DERIVE_SCHEMA, DERIVE_VERSION


def check_derivation(data: dict[str, Any], policy: str = "default") -> dict[str, Any]:
    """Evaluate derivation facts with a verification policy."""

    _require_schema(data, DERIVE_SCHEMA, DERIVE_VERSION)
    if policy != "default":
        raise ValueError(f"unknown policy: {policy}")

    summary = _object(data, "summary")
    target = _object(data, "target")
    target_reached = _boolean(summary, "target_reached")
    blocked = _integer(summary, "blocked")
    contradiction = _integer(summary, "contradiction")
    obligation = _integer(summary, "obligation")
    deferred = _integer(summary, "deferred")

    reasons = []
    if not target_reached:
        reasons.append(
            {
                "kind": "target_not_reached",
                "message": f"target not reached: {target.get('event')}",
            }
        )
    if blocked:
        reasons.extend(_record_reasons(data, "blocked"))
    if contradiction:
        reasons.extend(_record_reasons(data, "contradiction"))

    verdict = "passed" if not reasons else "failed"
    exit_code = 0 if verdict == "passed" else 1
    return {
        "schema": CHECK_SCHEMA,
        "version": CHECK_VERSION,
        "source": data.get("source"),
        "input": {
            "schema": data.get("schema"),
            "version": data.get("version"),
        },
        "policy": policy,
        "target": target.get("event"),
        "verdict": verdict,
        "exit_code": exit_code,
        "summary": {
            "target_reached": target_reached,
            "blocked": blocked,
            "contradiction": contradiction,
            "obligation": obligation,
            "deferred": deferred,
        },
        "allowed": {
            "obligation": True,
            "deferred": True,
        },
        "reasons": reasons,
    }


def _record_reasons(data: dict[str, Any], status: str) -> list[dict[str, Any]]:
    reasons = []
    for record in _list(data, "records"):
        record_data = _as_object(record, "record")
        if record_data.get("status") != status:
            continue
        reasons.append(
            {
                "kind": status,
                "message": record_data.get("message"),
                "span": record_data.get("span"),
                "object": record_data.get("object"),
                "event": record_data.get("event"),
                "state": record_data.get("state"),
                "expression": record_data.get("expression"),
            }
        )
    return reasons


def _require_schema(data: dict[str, Any], schema: str, version: int) -> None:
    if data.get("schema") != schema:
        raise ValueError(f"expected schema {schema!r}")
    if data.get("version") != version:
        raise ValueError(f"expected version {version}")


def _object(data: dict[str, Any], key: str) -> dict[str, Any]:
    return _as_object(data.get(key), key)


def _list(data: dict[str, Any], key: str) -> list[Any]:
    value = data.get(key)
    if not isinstance(value, list):
        raise ValueError(f"{key} must be a list")
    return value


def _boolean(data: dict[str, Any], key: str) -> bool:
    value = data.get(key)
    if not isinstance(value, bool):
        raise ValueError(f"{key} must be a boolean")
    return value


def _integer(data: dict[str, Any], key: str) -> int:
    value = data.get(key)
    if not isinstance(value, int):
        raise ValueError(f"{key} must be an integer")
    return value


def _as_object(value: Any, name: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError(f"{name} must be an object")
    return value
