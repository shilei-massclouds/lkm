"""Validate tools2 v4 inputs and project stable animation metadata."""

from __future__ import annotations

from copy import deepcopy
from typing import Any

from tools2_common import ANIMATION_SCHEMA, ANIMATION_VERSION, PRODUCER, ProtocolError


_HANDLER_KINDS = {"Transition", "Action"}
_OUTCOMES = {"completed", "rejected", "failed", "truncated", "stopped"}


def _required_string(value: dict[str, Any], field: str, *, label: str) -> str:
    result = value.get(field)
    if not isinstance(result, str) or not result:
        raise ProtocolError(f"{label}.{field} must be a non-empty string")
    return result


def _validate_snapshot(value: Any, *, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ProtocolError(f"{label} must be an object")
    states = value.get("states")
    facts = value.get("facts")
    references = value.get("references")
    if not isinstance(states, dict) or not all(
        isinstance(name, str) and isinstance(state, str) for name, state in states.items()
    ):
        raise ProtocolError(f"{label}.states must map system names to state names")
    if not isinstance(facts, list) or not all(isinstance(item, str) for item in facts):
        raise ProtocolError(f"{label}.facts must be a string list")
    if not isinstance(references, dict):
        raise ProtocolError(f"{label}.references must be an object")
    return value


def _validate_systems(model: dict[str, Any]) -> dict[str, dict[str, Any]]:
    payload = model.get("model")
    systems = payload.get("systems") if isinstance(payload, dict) else None
    if not isinstance(systems, dict):
        raise ProtocolError("model.model.systems must be an object")
    for name, system in systems.items():
        if not isinstance(name, str) or not name or not isinstance(system, dict):
            raise ProtocolError("model systems must use non-empty names and object values")
        parent = system.get("parent")
        if parent is not None and (not isinstance(parent, str) or parent not in systems):
            raise ProtocolError(f"model system {name!r} has unknown parent {parent!r}")
        states = system.get("states")
        if not isinstance(states, dict):
            raise ProtocolError(f"model system {name!r} has invalid states")
    for name in systems:
        seen: set[str] = set()
        current: str | None = name
        while current is not None:
            if current in seen:
                raise ProtocolError(f"model parent cycle contains {current!r}")
            seen.add(current)
            current = systems[current].get("parent")
    return systems


def _validate_identity(model: dict[str, Any], view: dict[str, Any]) -> tuple[str, str]:
    source = _required_string(model, "source", label="model")
    if view.get("source") != source:
        raise ProtocolError(
            f"model/view source mismatch: model={source!r}, view={view.get('source')!r}"
        )
    model_fingerprint = _required_string(model, "model_fingerprint", label="model")
    if view.get("model_fingerprint") != model_fingerprint:
        raise ProtocolError("model/view model_fingerprint mismatch")
    return source, model_fingerprint


def _project_step(
    signal: dict[str, Any], *, index: int, systems: dict[str, dict[str, Any]], external: str
) -> dict[str, Any]:
    label = f"view.signals[{index}]"
    signal_id = _required_string(signal, "id", label=label)
    source = _required_string(signal, "source", label=label)
    target = _required_string(signal, "target", label=label)
    name = _required_string(signal, "name", label=label)
    if source not in systems and source != external:
        raise ProtocolError(f"{label}.source references unknown endpoint {source!r}")
    if target not in systems:
        raise ProtocolError(f"{label}.target references unknown endpoint {target!r}")
    handler = signal.get("handler")
    if handler is not None:
        if not isinstance(handler, dict) or handler.get("kind") not in _HANDLER_KINDS:
            raise ProtocolError(f"{label}.handler has an unknown structure")
        handler_id = _required_string(handler, "id", label=f"{label}.handler")
        handler_kind = handler["kind"]
    else:
        handler_id = None
        handler_kind = None
    outcome = signal.get("outcome")
    if outcome not in _OUTCOMES:
        raise ProtocolError(f"{label}.outcome is not an animation v1 outcome: {outcome!r}")
    reason = signal.get("reason")
    if reason is not None and not isinstance(reason, str):
        raise ProtocolError(f"{label}.reason must be a string or null")
    before = _validate_snapshot(signal.get("before_snapshot"), label=f"{label}.before_snapshot")
    after = _validate_snapshot(signal.get("after_snapshot"), label=f"{label}.after_snapshot")
    return {
        "index": index,
        "id": signal_id,
        "cause_id": deepcopy(signal.get("cause_id")),
        "source": source,
        "target": target,
        "signal": name,
        "delivery": deepcopy(signal.get("delivery")),
        "coordinate": deepcopy(signal.get("coordinate")),
        "handler": {"id": handler_id, "kind": handler_kind},
        "outcome": outcome,
        "reason": reason,
        "before_snapshot": deepcopy(before),
        "after_snapshot": deepcopy(after),
    }


def build_animation(model: dict[str, Any], view: dict[str, Any]) -> dict[str, Any]:
    """Build animation v1 metadata without deriving any new Signal behavior."""
    systems = _validate_systems(model)
    source, model_fingerprint = _validate_identity(model, view)
    root_request = view.get("root_request")
    if not isinstance(root_request, dict):
        raise ProtocolError("view.root_request must be an object")
    external = _required_string(root_request, "source", label="view.root_request")
    raw_signals = view.get("signals")
    if not isinstance(raw_signals, list):
        raise ProtocolError("view.signals must be a list")
    steps: list[dict[str, Any]] = []
    seen: set[str] = set()
    for index, signal in enumerate(raw_signals):
        if not isinstance(signal, dict):
            raise ProtocolError(f"view.signals[{index}] must be an object")
        step = _project_step(signal, index=index, systems=systems, external=external)
        if step["id"] in seen:
            raise ProtocolError(f"view.signals has duplicate id {step['id']!r}")
        if step["cause_id"] is not None and step["cause_id"] not in seen:
            raise ProtocolError(
                f"view.signals[{index}].cause_id must reference an earlier Signal"
            )
        seen.add(step["id"])
        steps.append(step)
    return {
        "schema": ANIMATION_SCHEMA,
        "version": ANIMATION_VERSION,
        "producer": PRODUCER,
        "source": source,
        "inputs": {
            "model": {"schema": model["schema"], "version": model["version"]},
            "view": {"schema": view["schema"], "version": view["version"]},
            "model_fingerprint": model_fingerprint,
        },
        "trace": {
            "verdict": deepcopy(view.get("verdict")),
            "root_request": deepcopy(root_request),
            "budget": deepcopy(view.get("budget")),
            "total_steps": len(steps),
        },
        "systems": {
            name: {
                "parent": system.get("parent"),
                "states": sorted(system["states"]),
            }
            for name, system in sorted(systems.items())
        },
        "steps": steps,
        "initial_frame": {"nodes": [], "sibling_order": {}},
        "frames": [],
    }


__all__ = ["build_animation"]
