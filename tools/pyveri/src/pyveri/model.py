"""Compatibility exports for the model stage implementation."""

from __future__ import annotations

import sys
from pathlib import Path

_TOOLS_ROOT = Path(__file__).resolve().parents[3]
for source in (
    _TOOLS_ROOT / "common" / "src",
    _TOOLS_ROOT / "model" / "src",
):
    if source.is_dir():
        source_text = str(source)
        if source_text not in sys.path:
            sys.path.insert(0, source_text)

from common.model_types import (
    BuildResult,
    Diagnostic,
    EventDef,
    ObjectDef,
    ObjectModel,
    Severity,
    StateDef,
)
from model_tool.builder import build_model, summarize_model

__all__ = [
    "BuildResult",
    "Diagnostic",
    "EventDef",
    "ObjectDef",
    "ObjectModel",
    "Severity",
    "StateDef",
    "build_model",
    "summarize_model",
]
