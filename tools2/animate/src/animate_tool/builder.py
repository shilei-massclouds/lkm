"""Validate tools2 v5 inputs and project causal animation v3 moments."""

from __future__ import annotations

from copy import deepcopy
from typing import Any

from tools2_common import ANIMATION_SCHEMA, ANIMATION_VERSION, PRODUCER, ProtocolError


_HANDLER_KINDS = {"Transition", "Action"}
_OUTCOMES = {"completed", "rejected", "failed", "truncated", "stopped"}
_DELIVERIES = {"root", "drives", "emits"}
_TERMINAL_EVENTS = {
    "response_completed": "completed",
    "signal_rejected": "rejected",
    "signal_failed": "failed",
    "signal_truncated": "truncated",
    "signal_stopped": "stopped",
    "response_stopped": "stopped",
}


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
        isinstance(name, str) and (isinstance(state, str) or state is None)
        for name, state in states.items()
    ):
        raise ProtocolError(f"{label}.states must map system names to state names or null")
    if not isinstance(facts, list) or not all(isinstance(item, str) for item in facts):
        raise ProtocolError(f"{label}.facts must be a string list")
    if not isinstance(references, dict):
        raise ProtocolError(f"{label}.references must be an object")
    return value


def _require_snapshot_equal(
    actual: dict[str, Any], expected: dict[str, Any], *, label: str
) -> None:
    if actual != expected:
        raise ProtocolError(f"{label} snapshot does not match the causal replay state")


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


def _project_signal(
    signal: dict[str, Any], *, index: int, systems: dict[str, dict[str, Any]], external: str
) -> dict[str, Any]:
    label = f"view.signals[{index}]"
    signal_id = _required_string(signal, "id", label=label)
    source = _required_string(signal, "source", label=label)
    target = _required_string(signal, "target", label=label)
    name = _required_string(signal, "name", label=label)
    delivery = _required_string(signal, "delivery", label=label)
    if delivery not in _DELIVERIES:
        raise ProtocolError(f"{label}.delivery is not an animation v3 delivery: {delivery!r}")
    if source not in systems and source != external:
        raise ProtocolError(f"{label}.source references unknown endpoint {source!r}")
    if target not in systems:
        raise ProtocolError(f"{label}.target references unknown endpoint {target!r}")
    cause_id = signal.get("cause_id")
    if cause_id is not None and (not isinstance(cause_id, str) or not cause_id):
        raise ProtocolError(f"{label}.cause_id must be a non-empty string or null")
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
        raise ProtocolError(f"{label}.outcome is not an animation v3 outcome: {outcome!r}")
    reason = signal.get("reason")
    if reason is not None and not isinstance(reason, str):
        raise ProtocolError(f"{label}.reason must be a string or null")
    before = _validate_snapshot(signal.get("before_snapshot"), label=f"{label}.before_snapshot")
    after = _validate_snapshot(signal.get("after_snapshot"), label=f"{label}.after_snapshot")
    if target not in before["states"] or target not in after["states"]:
        raise ProtocolError(f"{label} target {target!r} is missing from its snapshots")
    before_state = before["states"][target]
    after_state = after["states"][target]
    declared_states = systems[target]["states"]
    if declared_states:
        if before_state not in declared_states or after_state not in declared_states:
            raise ProtocolError(f"{label} target {target!r} has an undeclared snapshot state")
    elif before_state is not None or after_state is not None:
        raise ProtocolError(f"{label} stateless target {target!r} must use null snapshot state")
    return {
        "signal_index": index,
        "signal_id": signal_id,
        "cause_id": cause_id,
        "source": source,
        "target": target,
        "signal": name,
        "delivery": delivery,
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


def _moment(signal: dict[str, Any], *, kind: str, sequence: int) -> dict[str, Any]:
    if kind == "request":
        transfer = {"from": signal["source"], "to": signal["target"]}
    elif kind == "feedback":
        transfer = {"from": signal["target"], "to": signal["source"]}
    else:
        transfer = None
    return {
        "index": -1,
        "id": f"{signal['signal_id']}:{kind}",
        "kind": kind,
        "event_sequence": sequence,
        "signal_id": signal["signal_id"],
        "cause_id": signal["cause_id"],
        "source": signal["source"],
        "target": signal["target"],
        "signal": signal["signal"],
        "delivery": signal["delivery"],
        "coordinate": deepcopy(signal["coordinate"]),
        "handler": deepcopy(signal["handler"]),
        "outcome": signal["outcome"],
        "reason": signal["reason"],
        "transfer": transfer,
        "response": deepcopy(signal["response"]),
    }


def _validate_events(
    view: dict[str, Any], signals: list[dict[str, Any]]
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    raw_events = view.get("events")
    if not isinstance(raw_events, list):
        raise ProtocolError("view.events must be a list")
    by_id = {signal["signal_id"]: signal for signal in signals}
    replay = deepcopy(
        _validate_snapshot(view.get("initial_snapshot"), label="view.initial_snapshot")
    )
    moments: list[dict[str, Any]] = []
    moment_snapshots: list[dict[str, Any]] = []
    sent: set[str] = set()
    received: set[str] = set()
    terminal: dict[str, str] = {}
    next_signal_index = 0

    for event_index, event in enumerate(raw_events):
        label = f"view.events[{event_index}]"
        if not isinstance(event, dict):
            raise ProtocolError(f"{label} must be an object")
        sequence = event.get("sequence")
        if not isinstance(sequence, int) or isinstance(sequence, bool) or sequence != event_index + 1:
            raise ProtocolError(f"{label}.sequence must be the next contiguous event sequence")
        kind = _required_string(event, "kind", label=label)
        if kind == "signal_sent":
            signal_id = _required_string(event, "signal_id", label=label)
            signal = by_id.get(signal_id)
            if signal is None:
                raise ProtocolError(f"{label} references unknown Signal {signal_id!r}")
            if signal_id in sent:
                raise ProtocolError(f"Signal {signal_id!r} has duplicate signal_sent events")
            if next_signal_index >= len(signals) or signals[next_signal_index] is not signal:
                raise ProtocolError(f"{label} does not preserve view.signals creation order")
            for event_field, signal_field in (
                ("source", "source"),
                ("target", "target"),
                ("signal", "signal"),
                ("delivery", "delivery"),
                ("cause_id", "cause_id"),
                ("coordinate", "coordinate"),
            ):
                if event.get(event_field) != signal[signal_field]:
                    raise ProtocolError(
                        f"{label}.{event_field} does not match Signal {signal_id!r}"
                    )
            if signal["cause_id"] is not None and signal["cause_id"] not in sent:
                raise ProtocolError(f"{label}.cause_id must reference an earlier sent Signal")
            sent.add(signal_id)
            next_signal_index += 1
            continue

        if kind == "signal_received":
            signal_id = _required_string(event, "signal_id", label=label)
            signal = by_id.get(signal_id)
            if signal is None:
                raise ProtocolError(f"{label} references unknown Signal {signal_id!r}")
            if signal_id not in sent or signal_id in terminal:
                raise ProtocolError(f"{label} is outside the Signal send/terminal interval")
            if signal_id in received:
                raise ProtocolError(f"Signal {signal_id!r} has duplicate signal_received events")
            if event.get("target") != signal["target"]:
                raise ProtocolError(f"{label}.target does not match Signal {signal_id!r}")
            _require_snapshot_equal(
                replay, signal["_before_snapshot"], label=f"{label} before"
            )
            received.add(signal_id)
            moments.append(_moment(signal, kind="request", sequence=sequence))
            moment_snapshots.append(deepcopy(replay))
            continue

        terminal_spec = _TERMINAL_EVENTS.get(kind)
        if terminal_spec is None:
            continue
        signal_id = _required_string(event, "signal_id", label=label)
        signal = by_id.get(signal_id)
        if signal is None:
            raise ProtocolError(f"{label} references unknown Signal {signal_id!r}")
        if signal_id not in sent:
            raise ProtocolError(f"{label} occurs before signal_sent")
        if signal_id in terminal:
            raise ProtocolError(f"Signal {signal_id!r} has duplicate terminal events")
        expected_outcome = terminal_spec
        if signal["outcome"] != expected_outcome:
            raise ProtocolError(
                f"{label} does not match Signal {signal_id!r} outcome {signal['outcome']!r}"
            )
        if expected_outcome in {"completed", "rejected", "failed"} and signal_id not in received:
            raise ProtocolError(f"{label} occurs before signal_received")
        if kind == "response_stopped" and signal_id not in received:
            raise ProtocolError(f"{label} occurs before signal_received")
        if kind == "signal_stopped" and signal_id in received:
            raise ProtocolError(f"{label} cannot terminate an already received Signal")
        if event.get("reason") != signal["reason"] and expected_outcome != "truncated":
            raise ProtocolError(f"{label}.reason does not match Signal {signal_id!r}")

        if kind == "response_completed":
            event_before = _validate_snapshot(event.get("before"), label=f"{label}.before")
            event_after = _validate_snapshot(event.get("after"), label=f"{label}.after")
            if event_before != signal["_before_snapshot"]:
                raise ProtocolError(f"{label}.before does not match Signal {signal_id!r}")
            if event_after != signal["_after_snapshot"]:
                raise ProtocolError(f"{label}.after does not match Signal {signal_id!r}")
            replay = deepcopy(event_after)
        else:
            _require_snapshot_equal(
                replay, signal["_after_snapshot"], label=f"{label} terminal"
            )
        terminal[signal_id] = kind
        if expected_outcome in {"truncated", "stopped"}:
            moment_kind = "terminal"
        elif signal["delivery"] == "emits":
            moment_kind = "settle"
        else:
            moment_kind = "feedback"
        moments.append(_moment(signal, kind=moment_kind, sequence=sequence))
        moment_snapshots.append(deepcopy(replay))

    for signal in signals:
        signal_id = signal["signal_id"]
        if signal_id not in sent:
            raise ProtocolError(f"Signal {signal_id!r} has no signal_sent event")
        if signal_id not in terminal:
            raise ProtocolError(f"Signal {signal_id!r} has no terminal event")
    for index, moment in enumerate(moments):
        moment["index"] = index
    return moments, moment_snapshots


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
    moments: list[dict[str, Any]],
    snapshots: list[dict[str, Any]],
    systems: dict[str, dict[str, Any]],
    external: str,
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

    def frame(
        snapshot: dict[str, Any], *, index: int, moment_id: str | None
    ) -> dict[str, Any]:
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
                if not structural and name not in snapshot["states"]:
                    raise ProtocolError(
                        f"animation frame {index} has no snapshot state for revealed system {name!r}"
                    )
                state = None if structural else snapshot["states"][name]
                declared_states = systems[name]["states"]
                if not structural and declared_states and state not in declared_states:
                    raise ProtocolError(
                        f"animation frame {index} has undeclared state for revealed system {name!r}"
                    )
                if not structural and not declared_states and state is not None:
                    raise ProtocolError(
                        f"animation frame {index} has state for stateless system {name!r}"
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
            parent: sorted(names, key=first_seen.__getitem__)
            for parent, names in sorted(children.items())
        }
        return {
            "index": index,
            "moment_id": moment_id,
            "nodes": nodes,
            "sibling_order": sibling_order,
        }

    initial_snapshot = snapshots[0] if snapshots else {
        "states": {},
        "facts": [],
        "references": {},
    }
    initial = frame(initial_snapshot, index=-1, moment_id=None)
    frames: list[dict[str, Any]] = []
    for moment, snapshot in zip(moments, snapshots, strict=True):
        if moment["kind"] == "request":
            reveal(moment["source"])
            reveal(moment["target"])
        frames.append(
            frame(snapshot, index=moment["index"], moment_id=moment["id"])
        )
    return initial, frames


def build_animation(model: dict[str, Any], view: dict[str, Any]) -> dict[str, Any]:
    """Build animation v3 moments without deriving any new Signal behavior."""
    systems = _validate_systems(model)
    source, model_fingerprint = _validate_identity(model, view)
    root_request = view.get("root_request")
    if not isinstance(root_request, dict):
        raise ProtocolError("view.root_request must be an object")
    external = _required_string(root_request, "source", label="view.root_request")
    raw_signals = view.get("signals")
    if not isinstance(raw_signals, list):
        raise ProtocolError("view.signals must be a list")
    signals: list[dict[str, Any]] = []
    seen: set[str] = set()
    for index, raw_signal in enumerate(raw_signals):
        if not isinstance(raw_signal, dict):
            raise ProtocolError(f"view.signals[{index}] must be an object")
        signal = _project_signal(
            raw_signal, index=index, systems=systems, external=external
        )
        if signal["signal_id"] in seen:
            raise ProtocolError(f"view.signals has duplicate id {signal['signal_id']!r}")
        if signal["cause_id"] is not None and signal["cause_id"] not in seen:
            raise ProtocolError(
                f"view.signals[{index}].cause_id must reference an earlier Signal"
            )
        seen.add(signal["signal_id"])
        signals.append(signal)
    moments, snapshots = _validate_events(view, signals)
    initial_frame, frames = _build_frames(
        moments=moments,
        snapshots=snapshots,
        systems=systems,
        external=external,
    )
    boundary = view.get("boundary")
    if boundary is not None and not isinstance(boundary, dict):
        raise ProtocolError("view.boundary must be an object or null")
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
            "boundary": deepcopy(boundary),
            "total_signals": len(signals),
            "total_moments": len(moments),
        },
        "systems": {
            name: {
                "parent": system.get("parent"),
                "states": sorted(system["states"]),
            }
            for name, system in sorted(systems.items())
        },
        "moments": moments,
        "initial_frame": initial_frame,
        "frames": frames,
    }


__all__ = ["build_animation"]
