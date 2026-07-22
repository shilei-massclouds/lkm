"""Projection of derive records; this module deliberately performs no derivation."""

from __future__ import annotations

from copy import deepcopy
from typing import Any


def build_view(derivation: dict[str, Any]) -> dict[str, Any]:
    signals = deepcopy(derivation.get("signals", []))
    depths: dict[str, int] = {}
    by_id = {item["id"]: item for item in signals}
    for signal in signals:
        depth = 0
        cause = signal.get("cause_id")
        seen: set[str] = set()
        while cause is not None and cause not in seen:
            seen.add(cause)
            depth += 1
            cause = by_id.get(cause, {}).get("cause_id")
        depths[signal["id"]] = depth
        signal["cause_depth"] = depth
    return {
        "view": "signal-trace",
        "root_request": deepcopy(derivation.get("root_request")),
        "model_fingerprint": derivation.get("model_fingerprint"),
        "budget": deepcopy(derivation.get("budget")),
        "verdict": derivation.get("verdict"),
        "initial_snapshot": deepcopy(derivation.get("initial_snapshot")),
        "last_stable_snapshot": deepcopy(derivation.get("last_stable_snapshot")),
        "signals": signals,
        "events": deepcopy(derivation.get("events", [])),
        "truncated_frontier": deepcopy(derivation.get("truncated_frontier", [])),
        "failure": deepcopy(derivation.get("failure")),
        "summary": deepcopy(derivation.get("summary", {})),
        "metadata": {
            "semantic_source": "derive.json",
            "rederived": False,
            "signal_depths": depths,
        },
    }
