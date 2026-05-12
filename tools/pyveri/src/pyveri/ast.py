"""Compatibility exports for syntax-level AST nodes."""

from __future__ import annotations

import sys
from pathlib import Path

_COMMON_SRC = Path(__file__).resolve().parents[3] / "common" / "src"
if _COMMON_SRC.is_dir():
    common_src = str(_COMMON_SRC)
    if common_src not in sys.path:
        sys.path.insert(0, common_src)

from common.spec_ast import (
    Block,
    EnumDecl,
    EventDecl,
    FunctionDecl,
    ObjectDecl,
    PredicateDecl,
    SourceSpan,
    SpecDocument,
    StateDecl,
    TypeDecl,
    statement_entries,
    statement_entry_spans,
)

__all__ = [
    "Block",
    "EnumDecl",
    "EventDecl",
    "FunctionDecl",
    "ObjectDecl",
    "PredicateDecl",
    "SourceSpan",
    "SpecDocument",
    "StateDecl",
    "TypeDecl",
    "statement_entries",
    "statement_entry_spans",
]
