"""Human-readable rendering of structured tools2 Signal trace records."""

from __future__ import annotations

import json
import os
from typing import Any


def _snapshot_delta(before: dict[str, Any] | None, after: dict[str, Any] | None) -> list[str]:
    if before is None or after is None:
        return []
    lines: list[str] = []
    for name in sorted(set(before["states"]) | set(after["states"])):
        old, new = before["states"].get(name), after["states"].get(name)
        if old != new:
            lines.append(f"state {name}: {old} -> {new}")
    before_facts, after_facts = set(before["facts"]), set(after["facts"])
    lines.extend(f"fact + {item}" for item in sorted(after_facts - before_facts))
    lines.extend(f"fact - {item}" for item in sorted(before_facts - after_facts))
    for name in sorted(set(before["references"]) | set(after["references"])):
        old, new = before["references"].get(name), after["references"].get(name)
        if old != new:
            lines.append(f"reference {name}: {old} -> {new}")
    before_bindings = before.get("contextual_bindings", {})
    after_bindings = after.get("contextual_bindings", {})
    for kind in sorted(set(before_bindings) | set(after_bindings)):
        old_entries = before_bindings.get(kind, {})
        new_entries = after_bindings.get(kind, {})
        for key in sorted(set(old_entries) | set(new_entries)):
            old, new = old_entries.get(key), new_entries.get(key)
            if old != new:
                lines.append(
                    f"contextual binding {kind}[{key}]: "
                    f"{json.dumps(old, sort_keys=True)} -> {json.dumps(new, sort_keys=True)}"
                )
    return lines


def _compact_signal_name(name: str) -> str:
    return "Startup" if name == "Preset" else name


def _transition_states(signal: dict[str, Any]) -> tuple[Any, Any] | None:
    handler = signal.get("handler")
    if handler is None or handler.get("kind") != "Transition":
        return None
    before = signal.get("before_snapshot")
    after = signal.get("after_snapshot")
    if before is None or after is None:
        return None
    target = signal["target"]
    before_states = before.get("states", {})
    after_states = after.get("states", {})
    if target not in before_states or target not in after_states:
        return None
    return before_states[target], after_states[target]


def _render_compact(view: dict[str, Any]) -> str:
    lines = [f"verdict: {view['verdict']}"]
    boundary = view.get("boundary")
    if boundary is not None:
        lines.append(
            f"boundary: {boundary['source']} -- {_compact_signal_name(boundary['signal'])} "
            f"--> {boundary['target']} (before send)"
        )

    signals = view["signals"]
    minimum_depth = min(
        (signal["coordinate"]["depth"] for signal in signals),
        default=0,
    )
    depth_origin = min(0, minimum_depth)
    for signal in signals:
        indent = "  " * (signal["coordinate"]["depth"] - depth_origin)
        line = (
            f"{indent}{signal['source']} -- {_compact_signal_name(signal['name'])} "
            f"--> {signal['target']}"
        )
        states = _transition_states(signal)
        if states is not None:
            line += f"[{states[0]}:{states[1]}]"
        if signal["outcome"] != "completed":
            line += f" !! {signal['outcome']}: {signal.get('reason')}"
        lines.append(line)

    failure = view.get("failure")
    if failure is not None:
        lines.append(
            "failure chain: "
            + " -> ".join(failure["chain"])
            + "; reason: "
            + str(failure["reason"])
        )
    return "\n".join(lines) + "\n"


def _render_verbose(view: dict[str, Any]) -> str:
    root = view["root_request"]
    lines = [
        f"Signal derivation: {root['source']} -> {root['target']}.{root['signal']}",
        f"verdict: {view['verdict']}",
        f"budget: depth={view['budget']['max_depth']} breadth={view['budget']['max_breadth']}",
    ]
    until_request = view.get("until_request")
    if until_request is not None:
        lines.append(f"until: {until_request['normalized_signal']} (before send)")
    boundary = view.get("boundary")
    if boundary is not None:
        position = boundary["send_position"]
        coordinate = position["coordinate"]
        lines.append(
            f"reached boundary: {boundary['source']} -> {boundary['normalized_signal']} "
            f"[{position['delivery']}] @ depth={coordinate['depth']},breadth={coordinate['breadth']}"
        )
        span = boundary.get("call_span")
        if span is not None:
            lines.append(
                f"boundary call: {span['source_file']}:{span['start_line']}:{span['start_column']}"
            )
    lines.append("signals:")
    for signal in view["signals"]:
        indent = "  " * (signal.get("cause_depth", 0) + 1)
        coordinate = signal["coordinate"]
        delivery = signal["delivery"]
        lines.append(
            f"{indent}{signal['id']} [{delivery}] {signal['source']} -> "
            f"{signal['target']}.{signal['name']} @ depth={coordinate['depth']},breadth={coordinate['breadth']} "
            f"=> {signal['outcome']}"
        )
        if delivery == "drives":
            lines.append(f"{indent}  synchronous: sender waits for this response")
        elif delivery == "emits":
            lines.append(f"{indent}  asynchronous: delivered by global FIFO")
        if signal.get("handler"):
            lines.append(f"{indent}  handler: {signal['handler']['id']}")
        if signal.get("payload"):
            rendered = ", ".join(
                f"{item['name']}:{item['type']}={json.dumps(item['value'], ensure_ascii=False)}"
                for item in signal["payload"]
            )
            lines.append(f"{indent}  payload: {rendered}")
        if signal.get("reason"):
            lines.append(f"{indent}  reason: {signal['reason']}")
        for delta in _snapshot_delta(signal.get("before_snapshot"), signal.get("after_snapshot")):
            lines.append(f"{indent}  {delta}")
    fifo = [event for event in view["events"] if event["kind"] in {"emits_enqueued", "emits_dequeued"}]
    if fifo:
        lines.append("asynchronous FIFO:")
        for event in fifo:
            if event["kind"] == "emits_enqueued":
                lines.append(
                    f"  #{event['sequence']} enqueue {event['child_id']} from {event['signal_id']} "
                    f"at position {event['fifo_position']}"
                )
            else:
                lines.append(
                    f"  #{event['sequence']} dequeue {event['signal_id']} remaining={event['remaining']}"
                )
    if view["truncated_frontier"]:
        lines.append("truncated frontier:")
        for item in view["truncated_frontier"]:
            coordinate = item["coordinate"]
            lines.append(
                f"  {item['signal_id']} {item['target']}.{item['signal']} "
                f"at depth={coordinate['depth']},breadth={coordinate['breadth']}"
            )
    if view.get("failure"):
        failure = view["failure"]
        lines.append("failure chain: " + " -> ".join(failure["chain"]))
        lines.append("failure reason: " + str(failure["reason"]))
    return "\n".join(lines) + "\n"


def render_text(view: dict[str, Any]) -> str:
    if os.environ.get("VERBOSE") == "1":
        return _render_verbose(view)
    return _render_compact(view)
