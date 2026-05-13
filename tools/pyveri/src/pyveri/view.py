"""Model views and renderers."""

from __future__ import annotations

import sys
from pathlib import Path

_TOOLS_ROOT = Path(__file__).resolve().parents[3]
for source in (
    _TOOLS_ROOT / "common" / "src",
    _TOOLS_ROOT / "view" / "src",
):
    if source.is_dir():
        source_text = str(source)
        if source_text not in sys.path:
            sys.path.insert(0, source_text)

from common.view_types import TimelineItem, TimelineRow, ViewEdge, ViewModel, ViewNode
from view_tool.builder import (
    build_drives_view,
    build_object_view,
    build_timeline_view,
)


def render_text(view: ViewModel) -> str:
    """Render a model view as plain text."""

    if view.name == "drives":
        return _render_drives_text(view)
    if view.name == "timeline":
        return _render_timeline_text(view)

    lines = [f"{view.name} view:"]
    for node in view.nodes.values():
        lines.append(f"{node.id}: {node.kind}")

    if view.edges:
        lines.append("")
        lines.append("edges:")
        for edge in view.edges:
            label = f" [{edge.label}]" if edge.label else ""
            lines.append(f"{edge.source} -> {edge.target}{label}")

    return "\n".join(lines)


def render_dot(view: ViewModel) -> str:
    """Render a model view as Graphviz DOT."""

    lines = [f"digraph {view.name.title()}View {{", f"  rankdir={view.rankdir};"]
    for node in view.nodes.values():
        lines.append(
            f'  "{_dot_escape(node.id)}" '
            f'[label="{_dot_escape(node.label)}\\n{_dot_escape(node.kind)}"];'
        )

    for edge in view.edges:
        attrs = []
        if edge.label:
            attrs.append(f'label="{_dot_escape(edge.label)}"')
        attr_text = f" [{', '.join(attrs)}]" if attrs else ""
        lines.append(
            f'  "{_dot_escape(edge.source)}" -> "{_dot_escape(edge.target)}"{attr_text};'
        )

    lines.append("}")
    return "\n".join(lines)


def render_svg(view: ViewModel) -> str:
    """Render a startup timeline SVG."""

    if view.name != "timeline":
        raise ValueError("SVG rendering is currently only supported for timeline views")

    rows = _timeline_rows(view)
    width = 1180
    top_margin = 44
    bottom_margin = 44
    left_margin = 24
    column_gap = 10
    phase_width = 108
    subphase_width = 134
    state_width = 132
    item_width = 170
    item_height = 42
    item_gap_x = 14
    item_gap_y = 12
    max_items_per_line = 4
    state_base_height = 74
    cell_padding = 12

    phase_x = left_margin
    subphase_x = phase_x + phase_width + column_gap
    state_x = subphase_x + subphase_width + column_gap
    item_x = state_x + state_width + column_gap

    row_heights = {
        row.id: _timeline_row_height(
            row,
            max_items_per_line,
            item_height,
            item_gap_y,
            state_base_height,
            cell_padding,
        )
        for row in rows
    }
    row_spans = _timeline_row_spans(rows, row_heights, top_margin, bottom_margin)
    height = max(320, top_margin + bottom_margin + sum(row_heights.values()))

    lines = [
        '<?xml version="1.0" encoding="UTF-8"?>',
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
        "<style>",
        "text { font-family: Arial, sans-serif; fill: #1f2937; }",
        ".cell { fill: #ffffff; stroke: #94a3b8; stroke-width: 1; }",
        ".phase { fill: #ffffff; stroke: #334155; stroke-width: 1.3; }",
        ".subphase { fill: #ffffff; stroke: #475569; stroke-width: 1.2; }",
        ".state { fill: #f8fafc; stroke: #64748b; stroke-width: 1.1; }",
        ".item { fill: #ffffff; stroke: #475569; stroke-width: 1.1; }",
        ".state-item { fill: #f0fdf4; stroke: #16a34a; }",
        ".muted { fill: #64748b; }",
        "</style>",
    ]

    for phase, phase_rows in _group_rows(rows, lambda row: row.phase).items():
        _append_cell(
            lines,
            phase,
            phase_rows,
            row_spans,
            x=phase_x,
            width=phase_width,
            css_class="phase",
        )

    for subphase, subphase_rows in _group_rows(
        tuple(row for row in rows if row.subphase is not None),
        lambda row: row.subphase or "",
    ).items():
        _append_cell(
            lines,
            subphase,
            subphase_rows,
            row_spans,
            x=subphase_x,
            width=subphase_width,
            css_class="subphase",
        )

    for row in rows:
        seg_top, seg_bottom = row_spans[row.id]
        seg_height = seg_bottom - seg_top
        seg_mid = seg_top + seg_height / 2
        item_lines = max(1, (len(row.items) + max_items_per_line - 1) // max_items_per_line)
        first_item_y = seg_mid + ((item_lines - 1) * (item_height + item_gap_y)) / 2
        lines.extend(
            [
                f'<rect class="cell" x="{item_x}" y="{seg_top:.1f}" width="{width - item_x - left_margin}" height="{seg_height:.1f}" rx="4" />',
                f'<rect class="state" x="{state_x}" y="{seg_top:.1f}" width="{state_width}" height="{seg_height:.1f}" rx="4" />',
                f'<text x="{state_x + state_width / 2}" y="{seg_mid - 4:.1f}" font-size="14" text-anchor="middle">{_xml_escape(row.label)}</text>',
                f'<text class="muted" x="{state_x + state_width / 2}" y="{seg_mid + 14:.1f}" font-size="11" text-anchor="middle">{_xml_escape(row.detail)}</text>',
            ]
        )

        for index, item in enumerate(row.items):
            col = index % max_items_per_line
            line = index // max_items_per_line
            x = item_x + col * (item_width + item_gap_x)
            item_y = first_item_y - line * (item_height + item_gap_y) - item_height / 2
            item_class = "state-item"
            lines.extend(
                [
                    f'<rect class="item {item_class}" x="{x}" y="{item_y:.1f}" width="{item_width}" height="{item_height}" rx="5" />',
                    f'<text x="{x + item_width / 2}" y="{item_y + 18:.1f}" font-size="13" text-anchor="middle">{_xml_escape(item.object_name)}</text>',
                    f'<text class="muted" x="{x + item_width / 2}" y="{item_y + 34:.1f}" font-size="11" text-anchor="middle">{_xml_escape(item.detail)}</text>',
                ]
            )

    lines.append("</svg>")
    return "\n".join(lines)


def _dot_escape(value: str) -> str:
    return value.replace("\\", "\\\\").replace('"', '\\"')


def _render_drives_text(view: ViewModel) -> str:
    lines = [f"{view.name} view:"]
    outgoing: dict[str, list[ViewEdge]] = {}
    for edge in view.edges:
        outgoing.setdefault(edge.source, []).append(edge)

    for source, edges in outgoing.items():
        lines.append(source)
        for edge in edges:
            lines.append(f"  -> {edge.target}")

    return "\n".join(lines)


def _render_timeline_text(view: ViewModel) -> str:
    lines = [f"{view.name} view:"]
    rows = _timeline_rows(view)
    if rows:
        for row in rows:
            row_name = row.subphase or row.phase
            lines.append(f"{row_name}: {row.label} ({row.detail})")
            for item in row.items:
                lines.append(f"  - {item.object_name}.{item.detail}")
        return "\n".join(lines)

    phase_nodes = [
        node
        for node in view.nodes.values()
        if node.kind in {"TimelineObject", "PhaseObject"}
    ]
    phase_nodes.sort(key=lambda node: _timeline_phase_order(view, node.id))

    for phase in phase_nodes:
        lines.append(f"{phase.id}: {phase.kind}")
        for event in _timeline_events_for_phase(view, phase.id):
            lines.append(f"  {event.id}")
            for edge in view.edges:
                if edge.kind == "drives" and edge.source == event.id:
                    lines.append(f"    -> {edge.target}")

    return "\n".join(lines)


def _timeline_phase_order(view: ViewModel, phase_id: str) -> int:
    return _timeline_phase_order_from_id(phase_id)


def _timeline_phase_order_from_id(phase_id: str) -> int:
    order = {
        "StartupTimeline": 0,
        "PreparePhase": 1,
        "BootPhase": 2,
        "EntryPreludePhase": 3,
    }
    return order.get(phase_id, 1000)


def _timeline_events_for_phase(view: ViewModel, phase_id: str) -> list[ViewNode]:
    prefix = f"{phase_id}."
    return [
        node
        for node in view.nodes.values()
        if node.kind == "Event" and node.id.startswith(prefix)
    ]


def _timeline_rows(view: ViewModel) -> tuple[TimelineRow, ...]:
    rows = view.metadata.get("timeline_rows", ())
    return rows if isinstance(rows, tuple) else ()


def _phase_parents(view: ViewModel) -> dict[str, str | None]:
    parents = view.metadata.get("phase_parents", {})
    return parents if isinstance(parents, dict) else {}


def _timeline_row_height(
    row: TimelineRow,
    max_items_per_line: int,
    item_height: int,
    item_gap_y: int,
    state_base_height: int,
    cell_padding: int,
) -> int:
    line_count = max(1, (len(row.items) + max_items_per_line - 1) // max_items_per_line)
    content_height = line_count * item_height + (line_count - 1) * item_gap_y
    return max(state_base_height, content_height + cell_padding * 2)


def _timeline_row_spans(
    rows: tuple[TimelineRow, ...],
    row_heights: dict[str, int],
    top_margin: int,
    bottom_margin: int,
) -> dict[str, tuple[float, float]]:
    total_height = top_margin + bottom_margin + sum(row_heights.values())
    bottom = total_height - bottom_margin
    spans: dict[str, tuple[float, float]] = {}
    for row in rows:
        row_height = row_heights[row.id]
        top = bottom - row_height
        spans[row.id] = (top, bottom)
        bottom = top
    return spans


def _group_rows(
    rows: tuple[TimelineRow, ...], key_fn
) -> dict[str, list[TimelineRow]]:
    groups: dict[str, list[TimelineRow]] = {}
    for row in rows:
        key = key_fn(row)
        if key:
            groups.setdefault(key, []).append(row)
    return groups


def _append_cell(
    lines: list[str],
    label: str,
    cell_rows: list[TimelineRow],
    row_spans: dict[str, tuple[float, float]],
    *,
    x: int,
    width: int,
    css_class: str,
) -> None:
    top = min(row_spans[row.id][0] for row in cell_rows)
    bottom = max(row_spans[row.id][1] for row in cell_rows)
    box_height = bottom - top
    center_y = top + box_height / 2
    label_x = x + width / 2
    lines.extend(
        [
            f'<rect class="{css_class}" x="{x}" y="{top:.1f}" width="{width}" height="{box_height:.1f}" rx="4" />',
            f'<text x="{label_x:.1f}" y="{center_y:.1f}" font-size="13" text-anchor="middle" transform="rotate(-90 {label_x:.1f} {center_y:.1f})">{_xml_escape(label)}</text>',
        ]
    )


def _xml_escape(value: str) -> str:
    return (
        value.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace('"', "&quot;")
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
]
