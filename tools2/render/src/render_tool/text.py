"""Human-readable rendering of structured tools2 Signal trace records."""

from __future__ import annotations

import json
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
    return lines


def render_text(view: dict[str, Any]) -> str:
    root = view["root_request"]
    lines = [
        f"Signal derivation: {root['source']} -> {root['target']}.{root['signal']}",
        f"verdict: {view['verdict']}",
        f"budget: depth={view['budget']['max_depth']} breadth={view['budget']['max_breadth']}",
        "signals:",
    ]
    for signal in view["signals"]:
        indent = "  " * (signal.get("cause_depth", 0) + 1)
        coordinate = signal["coordinate"]
        delivery = signal["delivery"]
        mode = "lossy" if signal["lossy"] else "strict"
        lines.append(
            f"{indent}{signal['id']} [{delivery} {mode}] {signal['source']} -> "
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
