"""Shared parsing for asynchronous Signal expressions."""

from __future__ import annotations

from dataclasses import dataclass
import re


_TRANSITION_SUFFIX_RE = re.compile(
    r"\A([A-Za-z_][A-Za-z0-9_]*)(?:\s*\((.*)\))?\Z",
    re.S,
)


@dataclass(frozen=True)
class EmitExpression:
    """One strict asynchronous Signal and its receiver expression."""

    receiver: str | None
    kind: str
    name: str
    args: str | None = None

    @property
    def transition(self) -> str:
        """Compatibility alias for transition-only consumers."""

        return self.name

def parse_emit_expression(expression: str) -> EmitExpression | None:
    """Parse local, static, or association-path strict Signal syntax."""

    text = expression.strip()
    if text.startswith("lossy "):
        return None

    for kind in ("Transition", "Action"):
        local_marker = f"{kind}::"
        path_marker = f".{kind}::"
        if text.startswith(local_marker):
            receiver = None
            suffix = text[len(local_marker) :]
            break
        receiver, marker, suffix = text.rpartition(path_marker)
        if marker and receiver.strip():
            receiver = receiver.strip()
            break
    else:
        return None

    match = _TRANSITION_SUFFIX_RE.match(suffix.strip())
    if match is None:
        return None
    return EmitExpression(
        receiver=receiver,
        kind=kind,
        name=match.group(1),
        args=match.group(2),
    )
