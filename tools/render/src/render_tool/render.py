"""Renderer dispatch for view models."""

from __future__ import annotations

import unicodedata

from common.view_types import (
    TimelineItem,
    TimelineRow,
    TraceArrow,
    TraceCell,
    ViewEdge,
    ViewModel,
    ViewNode,
)


_TRACE_DRIVE_ARROW_MAX_LENGTH = 81
_TRACE_STATE_BOX_HEIGHT = 34
_TRACE_LABEL_LINE_HEIGHT = 12
_TRACE_EVENT_LABEL_PAD_X = 8
_TRACE_EVENT_LABEL_HEIGHT = 34
_TRACE_ANNOTATION_WIDTH = 160
_TRACE_ANNOTATION_WRAP_UNITS = 26
_TRACE_ANNOTATION_LINE_HEIGHT = 13
_TRACE_ANNOTATION_PADDING_X = 8
_TRACE_ANNOTATION_PADDING_Y = 6
_TRACE_ANNOTATION_GAP = 10
_TRACE_ANNOTATION_MARGIN = 18
_TRACE_ANNOTATION_CLEARANCE = 6
_TRACE_ANNOTATION_FALLBACK_SCAN_STEPS = 80


def render_view(
    view: ViewModel, fmt: str, annotations: dict[str, object] | None = None
) -> str:
    """Render a view model with the requested output format."""

    if fmt == "text":
        return render_text(view)
    if fmt == "dot":
        return render_dot(view)
    if fmt == "svg":
        return render_svg(view, annotations)
    raise ValueError(f"unknown render format: {fmt}")


def render_text(view: ViewModel) -> str:
    """Render a model view as plain text."""

    if view.name == "drives":
        return _render_drives_text(view)
    if view.name == "timeline":
        return _render_timeline_text(view)
    if view.name == "trace":
        return _render_trace_text(view)

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


def render_svg(view: ViewModel, annotations: dict[str, object] | None = None) -> str:
    """Render a startup timeline SVG."""

    if view.name == "trace":
        return _render_trace_svg(view, annotations)
    if view.name != "timeline":
        raise ValueError("SVG rendering is currently only supported for timeline and trace views")

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


def _render_trace_svg(view: ViewModel, annotations: dict[str, object] | None) -> str:
    columns = _trace_columns(view)
    rows = _trace_rows(view)
    cells = _trace_cells(view)
    arrows = _trace_arrows(view)
    if not columns or not rows:
        raise ValueError("trace view has no layout metadata")

    column_metrics = _trace_column_metrics(columns)
    row_metrics = _trace_row_metrics(rows)
    left_margin = 28
    right_margin = 28
    top_margin = 28
    bottom_margin = 28
    annotation_items = _trace_annotation_items(annotations)
    annotation_margin = (
        _TRACE_ANNOTATION_WIDTH + _TRACE_ANNOTATION_MARGIN
        if annotation_items
        else 0
    )
    width = int(
        left_margin
        + right_margin
        + annotation_margin
        + sum(metric[1] for metric in column_metrics.values())
    )
    height = int(top_margin + bottom_margin + sum(metric[1] for metric in row_metrics.values()))
    cell_by_id = {cell.id: cell for cell in cells}
    context_cells = [cell for cell in cells if cell.kind == "context_span"]
    nested_context_ids = {
        inner.id
        for inner in context_cells
        for outer in context_cells
        if inner.id != outer.id and _trace_cell_contains(outer, inner)
    }
    phase_span_ids = {
        cell.id
        for cell in cells
        if cell.kind == "event_span" and _is_trace_phase_event(cell.label)
    }
    phase_state_ids = {
        arrow.source
        for arrow in arrows
        if arrow.kind == "state" and _trace_event_id(arrow.source) in phase_span_ids
    } | {
        arrow.target
        for arrow in arrows
        if arrow.kind == "state" and _trace_event_id(arrow.source) in phase_span_ids
    }
    def cell_box(cell: TraceCell) -> tuple[float, float, float, float]:
        x = left_margin + sum(
            column_metrics[index][1]
            for index in range(cell.column)
            if index in column_metrics
        )
        w = sum(
            column_metrics[index][1]
            for index in range(cell.column, cell.column + cell.column_span)
            if index in column_metrics
        )
        h = sum(
            row_metrics[index][1]
            for index in range(cell.row, cell.row + cell.row_span)
            if index in row_metrics
        )
        y = height - bottom_margin - sum(
            row_metrics[index][1]
            for index in range(cell.row + cell.row_span)
            if index in row_metrics
        )
        return x, y, w, h

    lines = [
        '<?xml version="1.0" encoding="UTF-8"?>',
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
        "<defs>",
        '<marker id="arrow" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto" markerUnits="strokeWidth">',
        '<path d="M0,0 L8,4 L0,8 Z" fill="#334155" />',
        "</marker>",
        '<marker id="dot" markerWidth="6" markerHeight="6" refX="3" refY="3" orient="auto" markerUnits="strokeWidth">',
        '<circle cx="3" cy="3" r="2.2" fill="#334155" />',
        "</marker>",
        "</defs>",
        "<style>",
        "text { font-family: Arial, sans-serif; fill: #1f2937; }",
        ".state { fill: #ffffff; stroke: #334155; stroke-width: 1.1; }",
        ".event-label { fill: #f8fafc; stroke: #94a3b8; stroke-width: 1; }",
        ".action { fill: #f8fafc; stroke: #94a3b8; stroke-width: 1; }",
        ".action-arrow { stroke: #64748b; stroke-width: 1.1; fill: none; marker-start: url(#dot); marker-end: url(#arrow); }",
        ".verified-state { fill: #f8fafc; stroke: #64748b; stroke-width: 1.1; }",
        ".phase-arrow { stroke: #0f172a; stroke-width: 4; fill: none; marker-start: url(#dot); marker-end: url(#arrow); }",
        ".phase-label { fill: #0f172a; font-weight: 600; }",
        ".state-arrow { stroke: #334155; stroke-width: 1.2; fill: none; marker-start: url(#dot); marker-end: url(#arrow); }",
        ".drive-arrow { stroke: #64748b; stroke-width: 1.1; fill: none; marker-start: url(#dot); marker-end: url(#arrow); }",
        ".depends-arrow { stroke: #64748b; stroke-width: 1; stroke-dasharray: 4 4; fill: none; marker-start: url(#dot); marker-end: url(#arrow); }",
        ".within-arrow { stroke: #0f766e; stroke-width: 1.1; stroke-dasharray: 5 4; fill: none; marker-start: url(#dot); marker-end: url(#arrow); }",
        ".context-box { fill: #f0fdfa; stroke: #0f766e; stroke-width: 1.2; stroke-dasharray: 6 4; }",
        ".context-action { fill: #f8fafc; stroke: #94a3b8; stroke-width: 1; }",
        ".context-order { stroke: #0f766e; stroke-width: 1; fill: none; marker-end: url(#arrow); }",
        ".context-title { fill: #0f766e; font-weight: 600; }",
        ".context-guard { fill: #115e59; }",
        ".muted { fill: #64748b; }",
    ]
    if annotation_items:
        lines.extend(
            [
                ".annotation-box { fill: #ffffff; stroke: #0f766e; stroke-width: 1; }",
                ".annotation-leader { stroke: #0f766e; stroke-width: 1; fill: none; }",
                ".annotation-text { fill: #0f172a; }",
            ]
        )
    lines.extend(["</style>"])

    for cell in cells:
        if cell.id in phase_state_ids:
            continue
        if cell.kind in {"state", "verified_state"}:
            _append_trace_state_cell(lines, cell, cell_box(cell))

    for cell in cells:
        if cell.kind == "context_span":
            _append_trace_context_box(
                lines,
                cell,
                cell_box(cell),
                nested=cell.id in nested_context_ids,
            )

    for cell in cells:
        if cell.kind == "event_span" and cell.id in phase_span_ids:
            _append_trace_phase_event(lines, cell, cell_box(cell))
        elif cell.kind == "event_span":
            _append_trace_event_label(lines, cell, cell_box(cell))
        elif cell.kind == "action":
            _append_trace_action(lines, cell, cell_box(cell))
        elif cell.kind == "context_action":
            _append_trace_context_action(lines, cell, cell_box(cell))

    for arrow in arrows:
        source = cell_by_id.get(arrow.source)
        target = cell_by_id.get(arrow.target)
        if source is None or target is None:
            continue
        if arrow.kind == "state":
            if _trace_event_id(arrow.source) in phase_span_ids:
                continue
            _append_trace_state_arrow(lines, cell_box(source), cell_box(target))
        elif arrow.kind == "drives":
            if _is_phase_to_phase_arrow(source, target):
                continue
            if source.kind == "context_action" or target.kind == "context_action":
                _append_trace_directed_arrow(
                    lines,
                    _trace_semantic_anchor_box(source, cell_box(source)),
                    _trace_semantic_anchor_box(target, cell_box(target)),
                    css_class="drive-arrow",
                )
            else:
                _append_trace_horizontal_arrow(
                    lines,
                    _trace_event_anchor_box(source, cell_box(source)),
                    _trace_event_anchor_box(target, cell_box(target)),
                    css_class="drive-arrow",
                    max_length=_TRACE_DRIVE_ARROW_MAX_LENGTH,
                )
        elif arrow.kind == "depends_on":
            _append_trace_horizontal_arrow(
                lines, cell_box(source), cell_box(target), css_class="depends-arrow"
            )
        elif arrow.kind == "within":
            _append_trace_directed_horizontal_arrow(
                lines,
                _trace_event_anchor_box(source, cell_box(source)),
                cell_box(target),
                css_class="within-arrow",
            )
        elif arrow.kind == "context_order":
            _append_trace_context_order_arrow(lines, cell_box(source), cell_box(target))
        elif arrow.kind == "action":
            _append_trace_action_arrow(
                lines,
                _trace_event_anchor_box(source, cell_box(source)),
                cell_box(target),
            )

    if annotation_items:
        _append_trace_annotations(
            lines,
            cells,
            cell_box,
            phase_span_ids,
            phase_state_ids,
            annotation_items,
            width - right_margin - _TRACE_ANNOTATION_WIDTH,
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


def _render_trace_text(view: ViewModel) -> str:
    lines = [f"{view.name} view:"]
    columns = _trace_columns(view)
    rows = _trace_rows(view)
    cells = sorted(_trace_cells(view), key=lambda cell: (cell.row, cell.column, cell.id))
    arrows = _trace_arrows(view)

    lines.append(f"columns: {len(columns)}")
    for column in columns:
        lines.append(
            f"  col {column.get('index')}: {column.get('kind')} depth={column.get('depth')}"
        )

    lines.append(f"rows: {len(rows)}")
    for row in rows:
        lines.append(
            f"  row {row.get('index')}: {row.get('kind')} {row.get('label')}"
        )

    lines.append("cells:")
    for cell in cells:
        span = ""
        if cell.row_span != 1 or cell.column_span != 1:
            span = f" span={cell.row_span}x{cell.column_span}"
        label = f" {cell.label}" if cell.label else ""
        lines.append(
            f"  r{cell.row} c{cell.column} {cell.kind} {cell.id}{span}{label}"
        )

    if arrows:
        lines.append("arrows:")
        for arrow in arrows:
            lines.append(f"  {arrow.source} -> {arrow.target} [{arrow.kind}]")
    return "\n".join(lines)


def _timeline_phase_order(view: ViewModel, phase_id: str) -> int:
    return _timeline_phase_order_from_id(phase_id)


def _timeline_phase_order_from_id(phase_id: str) -> int:
    order = {
        "StartupTimeline": 0,
        "PreparePhase": 1,
        "BootPhase": 2,
        "EntryPreludePhase": 3,
        "EntrySuccessorPhase": 4,
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


def _trace_columns(view: ViewModel) -> list[dict[str, object]]:
    columns = view.metadata.get("trace_columns", [])
    return columns if isinstance(columns, list) else []


def _trace_rows(view: ViewModel) -> list[dict[str, object]]:
    rows = view.metadata.get("trace_rows", [])
    return rows if isinstance(rows, list) else []


def _trace_cells(view: ViewModel) -> tuple[TraceCell, ...]:
    cells = view.metadata.get("trace_cells", ())
    return cells if isinstance(cells, tuple) else ()


def _trace_arrows(view: ViewModel) -> tuple[TraceArrow, ...]:
    arrows = view.metadata.get("trace_arrows", ())
    return arrows if isinstance(arrows, tuple) else ()


def _trace_column_metrics(columns: list[dict[str, object]]) -> dict[int, tuple[str, int]]:
    metrics: dict[int, tuple[str, int]] = {}
    for column in columns:
        index = column.get("index")
        kind = column.get("kind")
        if not isinstance(index, int) or not isinstance(kind, str):
            continue
        if kind == "phase":
            width = 84
        elif kind == "phase_object_gap":
            width = 56
        elif kind == "context":
            width = 260
        elif kind == "object":
            width = 178
        else:
            width = _TRACE_DRIVE_ARROW_MAX_LENGTH
        metrics[index] = (kind, width)
    return metrics


def _trace_row_metrics(rows: list[dict[str, object]]) -> dict[int, tuple[str, int]]:
    metrics: dict[int, tuple[str, int]] = {}
    for row in rows:
        index = row.get("index")
        kind = row.get("kind")
        label = row.get("label")
        if not isinstance(index, int) or not isinstance(kind, str):
            continue
        group_role = row.get("group_role")
        if group_role in {"source", "target"}:
            height = 48
        elif group_role == "context_padding":
            height = 22
        elif group_role == "context_action" or kind == "context_action":
            height = 56
        elif group_role == "context_guard":
            height = 28
        elif group_role in {"body_start", "body_end"}:
            height = 12
        elif kind == "action":
            height = 46
        elif isinstance(label, str) and _is_trace_phase_row(label):
            height = 20
        elif kind == "state":
            height = 48
        else:
            height = 46
        metrics[index] = (kind, height)
    return metrics


def _append_trace_state_cell(
    lines: list[str], cell: TraceCell, box: tuple[float, float, float, float]
) -> None:
    x, y, width, height = box
    pad_x = 8
    box_width = max(10, width - pad_x * 2)
    box_height = _TRACE_STATE_BOX_HEIGHT
    box_x = x + pad_x
    box_y = y + (height - box_height) / 2
    center_x = box_x + box_width / 2
    center_y = box_y + box_height / 2
    css_class = "verified-state" if cell.kind == "verified_state" else "state"
    lines.extend(
        [
            f'<title>{_xml_escape(cell.label)}</title>',
            f'<rect class="{css_class}" x="{box_x:.1f}" y="{box_y:.1f}" width="{box_width:.1f}" height="{box_height:.1f}" rx="4" />',
        ]
    )
    _append_trace_centered_text(
        lines,
        _trace_label_lines(_shorten_trace_label(cell.label)),
        center_x,
        center_y,
        font_size=11,
        baseline_offset=4,
    )


def _append_trace_event_label(
    lines: list[str], cell: TraceCell, box: tuple[float, float, float, float]
) -> None:
    x, y, width, height = box
    label_width = max(10, width - _TRACE_EVENT_LABEL_PAD_X * 2)
    label_x = x + _TRACE_EVENT_LABEL_PAD_X
    label_y = y + (height - _TRACE_EVENT_LABEL_HEIGHT) / 2
    lines.extend(
        [
            f'<rect class="event-label" x="{label_x:.1f}" y="{label_y:.1f}" width="{label_width:.1f}" height="{_TRACE_EVENT_LABEL_HEIGHT:.1f}" rx="4" />',
        ]
    )
    _append_trace_centered_text(
        lines,
        _trace_label_lines(_shorten_trace_event(cell.label)),
        label_x + label_width / 2,
        label_y + _TRACE_EVENT_LABEL_HEIGHT / 2,
        css_class="muted",
        font_size=10,
        baseline_offset=3.5,
    )


def _append_trace_phase_event(
    lines: list[str],
    cell: TraceCell,
    box: tuple[float, float, float, float],
) -> None:
    x, y, width, height = box
    arrow_x = x + width / 2
    y1 = y + height - 8
    y2 = y + 8
    if y1 <= y2:
        y1 = y + height
        y2 = y
    lines.append(
        f'<line class="phase-arrow" x1="{arrow_x:.1f}" y1="{y1:.1f}" x2="{arrow_x:.1f}" y2="{y2:.1f}" />'
    )
    label_x = arrow_x - 16
    label_y = y + height / 2
    lines.append(
        f'<text class="phase-label" x="{label_x:.1f}" y="{label_y:.1f}" font-size="11" text-anchor="middle" transform="rotate(-90 {label_x:.1f} {label_y:.1f})">{_xml_escape(_shorten_trace_event(cell.label))}</text>'
    )


def _append_trace_context_box(
    lines: list[str],
    cell: TraceCell,
    box: tuple[float, float, float, float],
    *,
    nested: bool = False,
) -> None:
    box_x, box_y, box_width, box_height = _trace_context_box_rect(box, nested=nested)
    title, details = _trace_context_label_lines(cell.label)
    lines.extend(
        [
            f'<title>{_xml_escape(cell.label)}</title>',
            f'<rect class="context-box" x="{box_x:.1f}" y="{box_y:.1f}" width="{box_width:.1f}" height="{box_height:.1f}" rx="6" />',
            f'<text class="context-title" x="{box_x + box_width / 2:.1f}" y="{box_y - 4:.1f}" font-size="10" text-anchor="middle">{_xml_escape(title)}</text>',
        ]
    )
    detail_y = box_y + 14
    for index, detail in enumerate(details[:1]):
        lines.append(
            f'<text class="context-guard" x="{box_x + 8:.1f}" y="{detail_y + index * 12:.1f}" font-size="8">{_xml_escape(detail)}</text>'
        )


def _append_trace_context_action(
    lines: list[str], cell: TraceCell, box: tuple[float, float, float, float]
) -> None:
    box_x, box_y, box_width, box_height = _trace_step_rect(box)
    center_x = box_x + box_width / 2
    center_y = box_y + box_height / 2
    lines.extend(
        [
            f'<title>{_xml_escape(cell.label)}</title>',
            f'<rect class="context-action" x="{box_x:.1f}" y="{box_y:.1f}" width="{box_width:.1f}" height="{box_height:.1f}" rx="4" />',
        ]
    )
    _append_trace_centered_text(
        lines,
        _trace_action_label_lines(cell.label),
        center_x,
        center_y,
        font_size=9,
        baseline_offset=3,
    )


def _append_trace_action(
    lines: list[str], cell: TraceCell, box: tuple[float, float, float, float]
) -> None:
    box_x, box_y, box_width, box_height = _trace_step_rect(box)
    center_x = box_x + box_width / 2
    center_y = box_y + box_height / 2
    lines.extend(
        [
            f'<title>{_xml_escape(cell.label)}</title>',
            f'<rect class="action" x="{box_x:.1f}" y="{box_y:.1f}" width="{box_width:.1f}" height="{box_height:.1f}" rx="4" />',
        ]
    )
    _append_trace_centered_text(
        lines,
        _trace_action_label_lines(cell.label),
        center_x,
        center_y,
        font_size=9,
        baseline_offset=3,
    )


def _append_trace_context_order_arrow(
    lines: list[str],
    source_box: tuple[float, float, float, float],
    target_box: tuple[float, float, float, float],
) -> None:
    source_x, source_y, source_w, source_h = _trace_step_rect(source_box)
    target_x, target_y, target_w, target_h = _trace_step_rect(target_box)
    source_center_y = source_y + source_h / 2
    target_center_y = target_y + target_h / 2
    x = (source_x + source_w / 2 + target_x + target_w / 2) / 2
    if target_center_y < source_center_y:
        y1 = source_y
        y2 = target_y + target_h
    else:
        y1 = source_y + source_h
        y2 = target_y
    lines.append(
        f'<line class="context-order" x1="{x:.1f}" y1="{y1:.1f}" x2="{x:.1f}" y2="{y2:.1f}" />'
    )


def _append_trace_action_arrow(
    lines: list[str],
    source_box: tuple[float, float, float, float],
    target_box: tuple[float, float, float, float],
) -> None:
    source_x, _source_y, source_w, _source_h = source_box
    target_x, target_y, target_w, target_h = _trace_step_rect(target_box)
    target_center_x = target_x + target_w / 2
    source_center_x = source_x + source_w / 2
    y = target_y + target_h / 2
    if target_center_x < source_center_x:
        x1 = source_x
        x2 = target_x + target_w
    else:
        x1 = source_x + source_w
        x2 = target_x
    lines.append(
        f'<line class="action-arrow" x1="{x1:.1f}" y1="{y:.1f}" x2="{x2:.1f}" y2="{y:.1f}" />'
    )


def _trace_context_action_rect(
    box: tuple[float, float, float, float]
) -> tuple[float, float, float, float]:
    return _trace_step_rect(box)


def _trace_action_rect(
    box: tuple[float, float, float, float]
) -> tuple[float, float, float, float]:
    return _trace_step_rect(box)


def _trace_step_rect(
    box: tuple[float, float, float, float]
) -> tuple[float, float, float, float]:
    x, y, width, height = box
    pad_x = _TRACE_EVENT_LABEL_PAD_X
    box_width = max(20, width - pad_x * 2)
    box_height = _TRACE_EVENT_LABEL_HEIGHT
    box_x = x + pad_x
    box_y = y + (height - box_height) / 2
    return box_x, box_y, box_width, box_height


def _trace_cell_contains(outer: TraceCell, inner: TraceCell) -> bool:
    return (
        outer.row <= inner.row
        and inner.row + inner.row_span <= outer.row + outer.row_span
        and outer.column <= inner.column
        and inner.column + inner.column_span <= outer.column + outer.column_span
    )


def _append_trace_state_arrow(
    lines: list[str],
    source_box: tuple[float, float, float, float],
    target_box: tuple[float, float, float, float],
) -> None:
    source_x, source_y, source_w, _source_h = source_box
    target_x, target_y, target_w, target_h = target_box
    x = source_x + source_w / 2
    y1 = source_y + 8
    y2 = target_y + target_h - 8
    if y2 <= y1:
        y1 = source_y
        y2 = target_y + target_h
    lines.append(
        f'<line class="state-arrow" x1="{x:.1f}" y1="{y1:.1f}" x2="{target_x + target_w / 2:.1f}" y2="{y2:.1f}" />'
    )


def _append_trace_horizontal_arrow(
    lines: list[str],
    source_box: tuple[float, float, float, float],
    target_box: tuple[float, float, float, float],
    *,
    css_class: str,
    max_length: float | None = None,
) -> None:
    source_x, source_y, source_w, source_h = source_box
    target_x, target_y, _target_w, target_h = target_box
    y = target_y + min(max(target_h / 2, 18), 34)
    x1 = source_x + source_w
    x2 = target_x + 6
    if x2 <= x1:
        y = source_y + source_h / 2
        x2 = target_x
    elif max_length is not None and x2 - x1 > max_length:
        x1 = x2 - max_length
    lines.append(
        f'<line class="{css_class}" x1="{x1:.1f}" y1="{y:.1f}" x2="{x2:.1f}" y2="{y:.1f}" />'
    )


def _append_trace_directed_horizontal_arrow(
    lines: list[str],
    source_box: tuple[float, float, float, float],
    target_box: tuple[float, float, float, float],
    *,
    css_class: str,
) -> None:
    source_x, source_y, source_w, source_h = source_box
    target_x, target_y, target_w, target_h = target_box
    source_center_x = source_x + source_w / 2
    target_center_x = target_x + target_w / 2
    y = source_y + source_h / 2
    if target_center_x < source_center_x:
        x1 = source_x
        x2 = target_x + target_w
    else:
        x1 = source_x + source_w
        x2 = target_x
    target_center_y = target_y + target_h / 2
    if abs(target_center_y - y) <= 12:
        y = target_center_y
    lines.append(
        f'<line class="{css_class}" x1="{x1:.1f}" y1="{y:.1f}" x2="{x2:.1f}" y2="{y:.1f}" />'
    )


def _append_trace_directed_arrow(
    lines: list[str],
    source_box: tuple[float, float, float, float],
    target_box: tuple[float, float, float, float],
    *,
    css_class: str,
) -> None:
    source_center = _rect_center(source_box)
    target_center = _rect_center(target_box)
    x1, y1 = _rect_edge_point_towards(source_box, target_center)
    x2, y2 = _rect_edge_point_towards(target_box, source_center)
    lines.append(
        f'<line class="{css_class}" x1="{x1:.1f}" y1="{y1:.1f}" x2="{x2:.1f}" y2="{y2:.1f}" />'
    )


def _trace_event_anchor_box(
    cell: TraceCell, box: tuple[float, float, float, float]
) -> tuple[float, float, float, float]:
    if cell.kind != "event_span" or _is_trace_phase_event(cell.label):
        return box
    x, y, width, height = box
    label_width = max(10, width - _TRACE_EVENT_LABEL_PAD_X * 2)
    label_x = x + _TRACE_EVENT_LABEL_PAD_X
    label_y = y + (height - _TRACE_EVENT_LABEL_HEIGHT) / 2
    return label_x, label_y, label_width, _TRACE_EVENT_LABEL_HEIGHT


def _trace_semantic_anchor_box(
    cell: TraceCell, box: tuple[float, float, float, float]
) -> tuple[float, float, float, float]:
    if cell.kind in {"action", "context_action"}:
        return _trace_step_rect(box)
    return _trace_event_anchor_box(cell, box)


def _trace_annotation_items(
    annotations: dict[str, object] | None,
) -> list[tuple[str, str, str]]:
    if annotations is None:
        return []
    items: list[tuple[str, str, str]] = []
    for section, kind in (("states", "state"), ("events", "event")):
        values = annotations.get(section)
        if not isinstance(values, dict):
            continue
        for label, note in values.items():
            if not isinstance(label, str):
                continue
            note_text = _annotation_text(note)
            if note_text:
                normalized_label = (
                    _shorten_trace_label(label)
                    if kind == "state"
                    else _shorten_trace_event(label)
                )
                items.append((kind, normalized_label, note_text))
    return items


def _annotation_text(value: object) -> str:
    if isinstance(value, str):
        return value.strip()
    if isinstance(value, dict):
        title = value.get("title")
        text = value.get("text")
        parts = [part.strip() for part in (title, text) if isinstance(part, str)]
        return " ".join(part for part in parts if part)
    return ""


def _append_trace_annotations(
    lines: list[str],
    cells: tuple[TraceCell, ...],
    cell_box,
    phase_span_ids: set[str],
    phase_state_ids: set[str],
    annotations: list[tuple[str, str, str]],
    fallback_x: float,
) -> None:
    targets = _trace_annotation_targets(cells, cell_box, phase_span_ids, phase_state_ids)
    occupied = [
        _expand_rect(rect, _TRACE_ANNOTATION_CLEARANCE)
        for rect in _trace_annotation_occupied_boxes(
            cells, cell_box, phase_span_ids, phase_state_ids
        )
    ]
    placed: list[tuple[float, float, float, float]] = []
    placed_annotations: list[
        tuple[
            tuple[float, float, float, float],
            tuple[float, float, float, float],
            list[str],
        ]
    ] = []
    fallback_y = _TRACE_ANNOTATION_MARGIN

    lines.append('<g class="annotations">')
    for kind, label, note in annotations:
        target = targets.get((kind, label))
        if target is None:
            continue
        note_lines = _wrap_annotation(note, _TRACE_ANNOTATION_WRAP_UNITS, 4)
        box_height = (
            _TRACE_ANNOTATION_PADDING_Y * 2
            + len(note_lines) * _TRACE_ANNOTATION_LINE_HEIGHT
        )
        box = _choose_annotation_box(
            target,
            _TRACE_ANNOTATION_WIDTH,
            box_height,
            occupied + [_expand_rect(rect, _TRACE_ANNOTATION_CLEARANCE) for rect in placed],
            fallback_x,
            fallback_y,
            _TRACE_ANNOTATION_MARGIN,
        )
        if box[0] == fallback_x:
            fallback_y = box[1] + box[3] + _TRACE_ANNOTATION_GAP
        placed.append(box)
        placed_annotations.append((box, target, note_lines))
    for box, target, _note_lines in placed_annotations:
        _append_annotation_leader(lines, box, target)
    for box, _target, note_lines in placed_annotations:
        _append_annotation_body(lines, box, note_lines)
    lines.append("</g>")


def _trace_annotation_targets(
    cells: tuple[TraceCell, ...],
    cell_box,
    phase_span_ids: set[str],
    phase_state_ids: set[str],
) -> dict[tuple[str, str], tuple[float, float, float, float]]:
    targets: dict[tuple[str, str], tuple[float, float, float, float]] = {}
    for cell in cells:
        if cell.id in phase_state_ids:
            continue
        if cell.kind in {"state", "verified_state"} and not _is_trace_phase_label(cell.label):
            targets[("state", _shorten_trace_label(cell.label))] = _trace_state_anchor_box(
                cell_box(cell)
            )
        elif cell.kind == "event_span" and cell.id not in phase_span_ids:
            targets[("event", _shorten_trace_event(cell.label))] = _trace_event_anchor_box(
                cell, cell_box(cell)
            )
    return targets


def _trace_annotation_occupied_boxes(
    cells: tuple[TraceCell, ...],
    cell_box,
    phase_span_ids: set[str],
    phase_state_ids: set[str],
) -> list[tuple[float, float, float, float]]:
    boxes: list[tuple[float, float, float, float]] = []
    for cell in cells:
        if cell.id in phase_state_ids:
            continue
        if cell.kind in {"state", "verified_state"} and not _is_trace_phase_label(cell.label):
            boxes.append(_trace_state_anchor_box(cell_box(cell)))
        elif cell.kind == "event_span" and cell.id not in phase_span_ids:
            boxes.append(_trace_event_anchor_box(cell, cell_box(cell)))
        elif cell.kind == "context_span":
            boxes.append(_trace_context_box_rect(cell_box(cell)))
        elif cell.kind == "action":
            boxes.append(_trace_action_rect(cell_box(cell)))
        elif cell.kind == "context_action":
            boxes.append(_trace_context_action_rect(cell_box(cell)))
    return boxes


def _trace_context_box_rect(
    box: tuple[float, float, float, float], *, nested: bool = False
) -> tuple[float, float, float, float]:
    x, y, width, height = box
    pad_x = 16 if nested else 10
    pad_y = 6
    return x + pad_x, y + pad_y, max(20, width - pad_x * 2), max(20, height - pad_y * 2)


def _trace_state_anchor_box(
    box: tuple[float, float, float, float]
) -> tuple[float, float, float, float]:
    x, y, width, height = box
    pad_x = 8
    box_width = max(10, width - pad_x * 2)
    box_height = _TRACE_STATE_BOX_HEIGHT
    box_x = x + pad_x
    box_y = y + (height - box_height) / 2
    return box_x, box_y, box_width, box_height


def _choose_annotation_box(
    target: tuple[float, float, float, float],
    width: float,
    height: float,
    occupied: list[tuple[float, float, float, float]],
    fallback_x: float,
    fallback_y: float,
    min_y: float,
) -> tuple[float, float, float, float]:
    x, y, w, h = target
    candidates = [
        (x + w + _TRACE_ANNOTATION_GAP, y + h / 2 - height / 2, width, height),
        (x + w / 2 - width / 2, y + h + _TRACE_ANNOTATION_GAP, width, height),
        (x - width - _TRACE_ANNOTATION_GAP, y + h / 2 - height / 2, width, height),
        (x + w / 2 - width / 2, y - height - _TRACE_ANNOTATION_GAP, width, height),
    ]
    for candidate in candidates:
        if candidate[0] < _TRACE_ANNOTATION_MARGIN or candidate[1] < _TRACE_ANNOTATION_MARGIN:
            continue
        if not any(_rects_overlap(candidate, rect) for rect in occupied):
            return candidate
    target_center_y = y + h / 2
    preferred_y = max(min_y, target_center_y - height / 2)
    fallback_candidates = [(fallback_x, preferred_y, width, height)]
    for step in range(1, _TRACE_ANNOTATION_FALLBACK_SCAN_STEPS + 1):
        offset = step * _TRACE_ANNOTATION_GAP
        fallback_candidates.append((fallback_x, preferred_y - offset, width, height))
        fallback_candidates.append((fallback_x, preferred_y + offset, width, height))
    for candidate in fallback_candidates:
        if candidate[1] < min_y:
            continue
        if not any(_rects_overlap(candidate, rect) for rect in occupied):
            return candidate
    return fallback_x, fallback_y, width, height


def _append_annotation_leader(
    lines: list[str],
    box: tuple[float, float, float, float],
    target: tuple[float, float, float, float],
) -> None:
    leader_x, leader_y, target_x, target_y = _annotation_leader_points(box, target)
    lines.append(
        f'<line class="annotation-leader" x1="{leader_x:.1f}" y1="{leader_y:.1f}" x2="{target_x:.1f}" y2="{target_y:.1f}" />'
    )


def _append_annotation_body(
    lines: list[str],
    box: tuple[float, float, float, float],
    note_lines: list[str],
) -> None:
    x, y, width, height = box
    lines.append(
        f'<rect class="annotation-box" x="{x:.1f}" y="{y:.1f}" width="{width:.1f}" height="{height:.1f}" rx="4" />'
    )
    for index, line in enumerate(note_lines):
        text_y = y + _TRACE_ANNOTATION_PADDING_Y + (index + 1) * _TRACE_ANNOTATION_LINE_HEIGHT - 2
        lines.append(
            f'<text class="annotation-text" x="{x + _TRACE_ANNOTATION_PADDING_X:.1f}" y="{text_y:.1f}" font-size="10">{_xml_escape(line)}</text>'
        )


def _annotation_leader_points(
    source: tuple[float, float, float, float],
    target: tuple[float, float, float, float],
) -> tuple[float, float, float, float]:
    source_center = _rect_center(source)
    target_center = _rect_center(target)
    source_x, source_y = _rect_edge_point_towards(source, target_center)
    target_x, target_y = _rect_edge_point_towards(target, source_center)
    return source_x, source_y, target_x, target_y


def _rect_center(rect: tuple[float, float, float, float]) -> tuple[float, float]:
    x, y, width, height = rect
    return x + width / 2, y + height / 2


def _rect_edge_point_towards(
    rect: tuple[float, float, float, float],
    point: tuple[float, float],
) -> tuple[float, float]:
    x, y, width, height = rect
    center_x, center_y = _rect_center(rect)
    dx = point[0] - center_x
    dy = point[1] - center_y
    if dx == 0 and dy == 0:
        return center_x, center_y
    x_scale = (width / 2) / abs(dx) if dx else float("inf")
    y_scale = (height / 2) / abs(dy) if dy else float("inf")
    scale = min(x_scale, y_scale)
    return center_x + dx * scale, center_y + dy * scale


def _append_trace_centered_text(
    lines: list[str],
    label_lines: tuple[str, ...],
    x: float,
    center_y: float,
    *,
    css_class: str | None = None,
    font_size: int,
    baseline_offset: float,
) -> None:
    class_attr = f' class="{css_class}"' if css_class else ""
    y = center_y - (len(label_lines) - 1) * _TRACE_LABEL_LINE_HEIGHT / 2 + baseline_offset
    if len(label_lines) == 1:
        lines.append(
            f'<text{class_attr} x="{x:.1f}" y="{y:.1f}" font-size="{font_size}" text-anchor="middle">{_xml_escape(label_lines[0])}</text>'
        )
        return

    parts = [
        f'<text{class_attr} x="{x:.1f}" y="{y:.1f}" font-size="{font_size}" text-anchor="middle">',
        f'<tspan x="{x:.1f}">{_xml_escape(label_lines[0])}</tspan>',
    ]
    for line in label_lines[1:]:
        parts.append(
            f'<tspan x="{x:.1f}" dy="{_TRACE_LABEL_LINE_HEIGHT}">{_xml_escape(line)}</tspan>'
        )
    parts.append("</text>")
    lines.append("".join(parts))


def _wrap_annotation(text: str, max_units: int, max_lines: int) -> list[str]:
    words = _annotation_words(text, max_units)
    if not words:
        return [""]
    lines: list[str] = []
    current = ""
    truncated = False
    for index, word in enumerate(words):
        candidate = word if not current else f"{current} {word}"
        if _annotation_text_units(candidate) <= max_units:
            current = candidate
            continue
        if len(lines) == max_lines - 1:
            truncated = True
            break
        if current:
            lines.append(current)
        current = word
    if current and len(lines) < max_lines:
        lines.append(current)
    elif current:
        truncated = True
    if not truncated and len(lines) == max_lines and index < len(words) - 1:
        truncated = True
    if truncated and lines:
        lines[-1] = _ellipsis_line(lines[-1], max_units)
    return lines or [_trim_annotation_line(text, max_units)]


def _annotation_words(text: str, max_units: int) -> list[str]:
    words: list[str] = []
    for word in text.split():
        current = ""
        current_units = 0
        for char in word:
            char_units = _annotation_char_units(char)
            if current and current_units + char_units > max_units:
                words.append(current)
                current = char
                current_units = char_units
            else:
                current += char
                current_units += char_units
        if current:
            words.append(current)
    return words


def _annotation_text_units(text: str) -> int:
    return sum(_annotation_char_units(char) for char in text)


def _annotation_char_units(char: str) -> int:
    return 2 if unicodedata.east_asian_width(char) in {"F", "W"} else 1


def _trim_annotation_line(line: str, max_units: int) -> str:
    result = ""
    used_units = 0
    for char in line:
        char_units = _annotation_char_units(char)
        if result and used_units + char_units > max_units:
            break
        if not result and char_units > max_units:
            break
        result += char
        used_units += char_units
    return result


def _ellipsis_line(line: str, max_units: int) -> str:
    ellipsis = "..."
    ellipsis_units = _annotation_text_units(ellipsis)
    if max_units <= ellipsis_units:
        return "." * max_units
    trimmed = _trim_annotation_line(line.rstrip("."), max_units - ellipsis_units).rstrip()
    return f"{trimmed}{ellipsis}" if trimmed else ellipsis


def _rects_overlap(
    a: tuple[float, float, float, float], b: tuple[float, float, float, float]
) -> bool:
    ax, ay, aw, ah = a
    bx, by, bw, bh = b
    return ax < bx + bw and ax + aw > bx and ay < by + bh and ay + ah > by


def _expand_rect(
    rect: tuple[float, float, float, float], amount: float
) -> tuple[float, float, float, float]:
    x, y, width, height = rect
    return x - amount, y - amount, width + amount * 2, height + amount * 2


def _shorten_trace_label(label: str) -> str:
    return label.replace(".State::", ".")


def _shorten_trace_event(label: str) -> str:
    return label.replace(".Event::", ".")


def _trace_label_lines(label: str) -> tuple[str, ...]:
    if "." not in label:
        return (label,)
    object_name, state_or_event = label.split(".", 1)
    return (object_name, state_or_event)


def _trace_action_label_lines(label: str) -> tuple[str, ...]:
    if "<-" in label:
        binding, action = label.split("<-", 1)
        binding = binding.strip()
        if binding.startswith("let "):
            binding = binding[4:]
        action_lines = _trace_action_label_lines(action.strip())
        return (f"{binding} <-", ".".join(action_lines))
    compact = label.replace(".Action::", ".")
    compact = compact.replace(".Event::", ".")
    compact = compact.split("(", 1)[0]
    if "." not in compact:
        return (compact,)
    object_name, action_name = compact.split(".", 1)
    return (object_name, action_name)


def _trace_context_label_lines(label: str) -> tuple[str, tuple[str, ...]]:
    parts = tuple(part.strip() for part in label.split("|") if part.strip())
    if not parts:
        return "", ()
    title = parts[0]
    details = tuple(
        _shorten_context_detail(part) for part in parts[1:] if part.startswith("lock=")
    )
    return title, details


def _shorten_context_detail(label: str) -> str:
    if "=" not in label:
        return label
    key, value = label.split("=", 1)
    compact = value.replace(".Event::", ".")
    return f"{key}: {compact}"


def _is_trace_phase_event(label: str) -> bool:
    object_name = label.split(".Event::", 1)[0]
    return object_name == "StartupTimeline" or object_name.endswith("Phase")


def _is_trace_phase_label(label: str) -> bool:
    object_name = label.split(".", 1)[0]
    return object_name == "StartupTimeline" or object_name.endswith("Phase")


def _is_trace_phase_row(label: str) -> bool:
    if ".verified." in label:
        return False
    object_name = label.split(".", 1)[0]
    return object_name == "StartupTimeline" or object_name.endswith("Phase")


def _is_phase_to_phase_arrow(source: TraceCell, target: TraceCell) -> bool:
    if source.kind != "event_span" or target.kind != "event_span":
        return False
    return _is_trace_phase_event(source.label) and _is_trace_phase_event(target.label)


def _trace_event_id(cell_id: str) -> str:
    if "-" not in cell_id:
        return cell_id
    parts = cell_id.split("-")
    if len(parts) >= 2 and parts[0] == "event":
        return f"{parts[0]}-{parts[1]}-span"
    return cell_id


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
    "render_dot",
    "render_svg",
    "render_text",
    "render_view",
]
