"""Final tools2 verdict policy: only complete is successful."""

from __future__ import annotations

from typing import Any


def check_derivation(derivation: dict[str, Any]) -> dict[str, Any]:
    verdict = derivation.get("verdict")
    if verdict not in {"complete", "failed", "bounded"}:
        raise ValueError(f"unknown derivation verdict {verdict!r}")
    allowed = verdict == "complete"
    reasons: list[str] = []
    if verdict == "failed":
        failure = derivation.get("failure") or {}
        reasons.append(failure.get("reason", "derivation failed"))
    elif verdict == "bounded":
        reasons.append(
            f"derivation stopped at {len(derivation.get('truncated_frontier', []))} truncated frontier signal(s)"
        )
    return {
        "policy": "tools2-signal-complete-v1",
        "verdict": verdict,
        "allowed": allowed,
        "exit_code": 0 if allowed else 1,
        "root_request": derivation.get("root_request"),
        "model_fingerprint": derivation.get("model_fingerprint"),
        "reasons": reasons,
        "summary": derivation.get("summary", {}),
    }
