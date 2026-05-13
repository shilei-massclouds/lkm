"""Compatibility exports for the derive stage implementation."""

from __future__ import annotations

import sys
from pathlib import Path

_TOOLS_ROOT = Path(__file__).resolve().parents[3]
for source in (
    _TOOLS_ROOT / "common" / "src",
    _TOOLS_ROOT / "derive" / "src",
):
    if source.is_dir():
        source_text = str(source)
        if source_text not in sys.path:
            sys.path.insert(0, source_text)

from common.defaults import DEFAULT_TARGET
from common.derive_types import (
    DerivationRecord,
    DerivationResult,
    DerivationStatus,
    DerivationTraceNode,
    EventTransition,
)
from derive_tool.engine import derive, render_derivation_text, summarize_derivation

__all__ = [
    "DEFAULT_TARGET",
    "DerivationRecord",
    "DerivationResult",
    "DerivationStatus",
    "DerivationTraceNode",
    "EventTransition",
    "derive",
    "render_derivation_text",
    "summarize_derivation",
]
