"""Compatibility exports for the parse stage implementation."""

from __future__ import annotations

import sys
from pathlib import Path

_TOOLS_ROOT = Path(__file__).resolve().parents[3]
for source in (_TOOLS_ROOT / "common" / "src", _TOOLS_ROOT / "parse" / "src"):
    if source.is_dir():
        source_text = str(source)
        if source_text not in sys.path:
            sys.path.insert(0, source_text)

from parse_tool.parser import (
    ParseError,
    parse_file,
    parse_text,
    strip_comments,
    summarize,
)
from common.spec_ast import statement_entries

__all__ = [
    "ParseError",
    "parse_file",
    "parse_text",
    "statement_entries",
    "strip_comments",
    "summarize",
]
