"""Shared parsing for transition completion-event expressions."""

from __future__ import annotations

from dataclasses import dataclass
import re


_TRANSITION_SUFFIX_RE = re.compile(
    r"\A([A-Za-z_][A-Za-z0-9_]*)(?:\s*\((.*)\))?\Z",
    re.S,
)


@dataclass(frozen=True)
class EmitExpression:
    """One strict or lossy completion event and its receiver expression."""

    lossy: bool
    receiver: str | None
    transition: str
    args: str | None = None


def parse_emit_expression(expression: str) -> EmitExpression | None:
    """Parse local, static, or association-path transition emission syntax."""

    text = expression.strip()
    lossy = False
    if text.startswith("lossy "):
        lossy = True
        text = text[len("lossy ") :].strip()

    if text.startswith("Transition::"):
        receiver = None
        suffix = text[len("Transition::") :]
    else:
        receiver, marker, suffix = text.rpartition(".Transition::")
        if not marker or not receiver.strip():
            return None
        receiver = receiver.strip()

    match = _TRANSITION_SUFFIX_RE.match(suffix.strip())
    if match is None:
        return None
    return EmitExpression(
        lossy=lossy,
        receiver=receiver,
        transition=match.group(1),
        args=match.group(2),
    )
