"""Compatibility exports for view and render stage implementations."""

from __future__ import annotations

import sys
from pathlib import Path

_TOOLS_ROOT = Path(__file__).resolve().parents[3]
for source in (
    _TOOLS_ROOT / "common" / "src",
    _TOOLS_ROOT / "view" / "src",
    _TOOLS_ROOT / "render" / "src",
):
    if source.is_dir():
        source_text = str(source)
        if source_text not in sys.path:
            sys.path.insert(0, source_text)

from common.view_types import TimelineItem, TimelineRow, ViewEdge, ViewModel, ViewNode
from render_tool.render import render_dot, render_svg, render_text, render_view
from view_tool.builder import (
    build_drives_view,
    build_object_view,
    build_timeline_view,
)

__all__ = [
    "TimelineItem",
    "TimelineRow",
    "ViewEdge",
    "ViewModel",
    "ViewNode",
    "build_drives_view",
    "build_object_view",
    "build_timeline_view",
    "render_dot",
    "render_svg",
    "render_text",
    "render_view",
]
