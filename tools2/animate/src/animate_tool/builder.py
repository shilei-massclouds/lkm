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
    before_state = before["states"].get(target)
    after_state = after["states"].get(target)
    if before_state is None or after_state is None:
        raise ProtocolError(f"{label} target {target!r} is missing from its snapshots")
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
        "response": {
            "before_state": before_state if handler_kind == "Transition" else None,
            "after_state": after_state if handler_kind == "Transition" else None,
        },
        "_before_snapshot": before,
        "_after_snapshot": after,
    }


def _ancestors(name: str, systems: dict[str, dict[str, Any]]) -> list[str]:
    result: list[str] = []
    current = systems[name].get("parent")
    while current is not None:
        result.append(current)
        current = systems[current].get("parent")
    result.reverse()
    return result


def _build_frames(
    *,
    steps: list[dict[str, Any]],
    systems: dict[str, dict[str, Any]],
    external: str,
    initial_snapshot: dict[str, Any],
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    visible: set[str] = set()
    revealed: set[str] = set()
    first_seen: dict[str, int] = {}
    ordinal = 0

    def add_visible(name: str) -> None:
        nonlocal ordinal
        if name not in visible:
            visible.add(name)
            first_seen[name] = ordinal
            ordinal += 1

    def reveal(name: str) -> None:
        if name not in systems:
            if name != external:
                raise ProtocolError(f"animation frame references unknown external endpoint {name!r}")
            add_visible(name)
            revealed.add(name)
            return
        for ancestor in _ancestors(name, systems):
            add_visible(ancestor)
        add_visible(name)
        revealed.add(name)

    def frame(snapshot: dict[str, Any], *, index: int, step_id: str | None) -> dict[str, Any]:
        nodes: list[dict[str, Any]] = []
        children: dict[str, list[str]] = {}
        for name in sorted(visible, key=first_seen.__getitem__):
            if name not in systems:
                parent = None
                kind = "external"
                structural = False
                state = None
            else:
                parent = systems[name].get("parent")
                kind = "system"
                structural = name not in revealed
                state = None if structural else snapshot["states"].get(name)
                if not structural and state is None:
                    raise ProtocolError(
                        f"animation frame {index} has no snapshot state for revealed system {name!r}"
                    )
            nodes.append(
                {
                    "id": name,
                    "parent": parent,
                    "kind": kind,
                    "state": state,
                    "structural": structural,
                    "first_seen": first_seen[name],
                }
            )
            children.setdefault(parent or "$root", []).append(name)
        sibling_order = {
            parent: sorted(names, key=first_seen.__getitem__, reverse=True)
            for parent, names in sorted(children.items())
        }
        return {
            "index": index,
            "step_id": step_id,
            "nodes": nodes,
            "sibling_order": sibling_order,
        }

    if steps:
        reveal(steps[0]["source"])
    initial = frame(initial_snapshot, index=-1, step_id=None)
    frames: list[dict[str, Any]] = []
    for step in steps:
        reveal(step["source"])
        reveal(step["target"])
        frames.append(
            frame(step["_after_snapshot"], index=step["index"], step_id=step["id"])
        )
    return initial, frames


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
    initial_snapshot = _validate_snapshot(
        view.get("initial_snapshot"), label="view.initial_snapshot"
    )
    initial_frame, frames = _build_frames(
        steps=steps,
        systems=systems,
        external=external,
        initial_snapshot=initial_snapshot,
    )
    for step in steps:
        del step["_before_snapshot"]
        del step["_after_snapshot"]
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
        "initial_frame": initial_frame,
        "frames": frames,
    }


__all__ = ["build_animation"]
