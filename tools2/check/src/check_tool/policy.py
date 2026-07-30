"""Final tools2 verdict policy for complete and pre-send boundary success."""

from __future__ import annotations

from typing import Any


def check_derivation(derivation: dict[str, Any]) -> dict[str, Any]:
    verdict = derivation.get("verdict")
    if verdict not in {
        "complete",
        "reached",
        "until_signal_not_reached",
        "failed",
        "bounded",
    }:
        raise ValueError(f"unknown derivation verdict {verdict!r}")
    obligations = derivation.get("obligations")
    if not isinstance(obligations, list):
        raise ValueError("derivation obligations must be a list")
    unresolved = [item for item in obligations if item.get("unresolved") is True]
    summary = derivation.get("summary")
    if not isinstance(summary, dict) or summary.get("unresolved_obligations") != len(unresolved):
        raise ValueError("derivation unresolved obligation summary is inconsistent")
    allowed = verdict in {"complete", "reached"} and not unresolved
    reasons: list[str] = []
    if verdict == "failed":
        failure = derivation.get("failure") or {}
        reasons.append(failure.get("reason", "derivation failed"))
    elif verdict == "bounded":
        reasons.append(
            f"derivation stopped at {len(derivation.get('truncated_frontier', []))} truncated frontier signal(s)"
        )
    elif verdict == "until_signal_not_reached":
        request = derivation.get("until_request") or {}
        reasons.append(
            f"until_signal_not_reached: {request.get('normalized_signal', '<unknown>')}"
        )
    if unresolved:
        reasons.append(f"unresolved_obligations: {len(unresolved)}")
    return {
        "policy": "tools2-signal-v9-boundary-obligations",
        "verdict": verdict,
        "allowed": allowed,
        "exit_code": 0 if allowed else 1,
        "root_request": derivation.get("root_request"),
        "until_request": derivation.get("until_request"),
        "boundary": derivation.get("boundary"),
        "model_fingerprint": derivation.get("model_fingerprint"),
        "reasons": reasons,
        "summary": summary,
        "unresolved_obligations": len(unresolved),
    }
