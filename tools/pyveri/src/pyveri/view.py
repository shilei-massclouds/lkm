"""Model views and renderers."""

from __future__ import annotations
import re
from dataclasses import dataclass, field

from .model import ObjectModel


@dataclass(frozen=True)
class ViewNode:
    """A node in a rendered model view."""

    id: str
    label: str
    kind: str


@dataclass(frozen=True)
class ViewEdge:
    """A directed edge in a rendered model view."""

    source: str
    target: str
    kind: str
    label: str = ""


@dataclass(frozen=True)
class ViewModel:
    """A renderable model view."""

    name: str
    nodes: dict[str, ViewNode] = field(default_factory=dict)
    edges: list[ViewEdge] = field(default_factory=list)
    rankdir: str = "TB"
    graph_format: str = "dot"


_OBJECT_EVENT_RE = re.compile(r"\b([A-Z][A-Za-z0-9_]*)\.Event::([A-Za-z_][A-Za-z0-9_]*)\b")


def build_object_view(model: ObjectModel) -> ViewModel:
    """Build an object-level view with static parent edges."""

    nodes = {
        name: ViewNode(id=name, label=name, kind=obj.kind)
        for name, obj in model.objects.items()
    }
    edges: list[ViewEdge] = []

    for obj in model.objects.values():
        if obj.parent is not None:
            edges.append(
                ViewEdge(
                    source=obj.parent,
                    target=obj.name,
                    kind="parent",
                    label="parent",
                )
            )

    return ViewModel(name="object", nodes=nodes, edges=edges)


def build_drives_view(model: ObjectModel) -> ViewModel:
    """Build an event-level view from drives blocks."""

    nodes: dict[str, ViewNode] = {}
    edges: list[ViewEdge] = []

    for obj in model.objects.values():
        for state in obj.states.values():
            for event in state.events.values():
                source = _event_node_id(obj.name, event.name)
                _add_event_node(nodes, source, obj.name, event.name)

                for block in event.decl.drives:
                    for target_obj, target_event in _OBJECT_EVENT_RE.findall(block.body):
                        if target_obj not in model.objects:
                            continue
                        target = _event_node_id(target_obj, target_event)
                        _add_event_node(nodes, target, target_obj, target_event)
                        edges.append(ViewEdge(source=source, target=target, kind="drives"))

    return ViewModel(name="drives", nodes=nodes, edges=edges, rankdir="LR")


def build_timeline_view(model: ObjectModel) -> ViewModel:
    """Build a timeline view from timeline and phase object events."""

    nodes: dict[str, ViewNode] = {}
    edges: list[ViewEdge] = []
    phase_objects = {
        name
        for name, obj in model.objects.items()
        if obj.kind in {"TimelineObject", "PhaseObject"}
    }

    for name in phase_objects:
        obj = model.objects[name]
        nodes[name] = ViewNode(id=name, label=name, kind=obj.kind)

    for name in phase_objects:
        obj = model.objects[name]
        for state in obj.states.values():
            for event in state.events.values():
                event_id = _event_node_id(name, event.name)
                _add_event_node(nodes, event_id, name, event.name)
                edges.append(
                    ViewEdge(
                        source=name,
                        target=event_id,
                        kind="has_event",
                    )
                )

                for block in event.decl.drives:
                    for target_obj, target_event in _OBJECT_EVENT_RE.findall(block.body):
                        if target_obj not in phase_objects:
                            continue
                        target = _event_node_id(target_obj, target_event)
                        _add_event_node(nodes, target, target_obj, target_event)
                        edges.append(
                            ViewEdge(
                                source=event_id,
                                target=target,
                                kind="drives",
                            )
                        )

    return ViewModel(
        name="timeline",
        nodes=nodes,
        edges=edges,
        rankdir="BT",
        graph_format="svg",
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
    """Render a timeline view as a small self-contained SVG."""

    if view.name != "timeline":
        raise ValueError("SVG rendering is currently only supported for timeline views")

    phase_nodes = [
        node
        for node in view.nodes.values()
        if node.kind in {"TimelineObject", "PhaseObject"}
    ]
    phase_nodes.sort(key=lambda node: _timeline_phase_order(view, node.id))

    width = 980
    row_height = 120
    margin = 40
    height = max(260, margin * 2 + row_height * max(len(phase_nodes), 1))
    axis_x = 120
    phase_x = 170
    event_x = 420

    y_positions: dict[str, int] = {}
    for index, node in enumerate(phase_nodes):
        y_positions[node.id] = height - margin - index * row_height

    event_positions: dict[str, tuple[int, int]] = {}
    for phase in phase_nodes:
        events = _timeline_events_for_phase(view, phase.id)
        base_y = y_positions[phase.id]
        for index, event in enumerate(events):
            event_positions[event.id] = (event_x, base_y - 24 + index * 28)

    lines = [
        '<?xml version="1.0" encoding="UTF-8"?>',
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
        "<style>",
        "text { font-family: Arial, sans-serif; font-size: 14px; fill: #1f2937; }",
        ".axis { stroke: #475569; stroke-width: 2; }",
        ".phase { fill: #eef2ff; stroke: #4f46e5; stroke-width: 1.5; }",
        ".event { fill: #ecfeff; stroke: #0891b2; stroke-width: 1.2; }",
        ".edge { stroke: #64748b; stroke-width: 1.2; fill: none; marker-end: url(#arrow); }",
        ".drive { stroke: #0f766e; stroke-dasharray: 5 4; }",
        "</style>",
        "<defs>",
        '<marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">',
        '<path d="M 0 0 L 10 5 L 0 10 z" fill="#64748b"/>',
        "</marker>",
        "</defs>",
        f'<line class="axis" x1="{axis_x}" y1="{height - margin}" x2="{axis_x}" y2="{margin}" />',
        f'<text x="{axis_x - 40}" y="{margin - 12}">time</text>',
    ]

    for phase in phase_nodes:
        y = y_positions[phase.id]
        lines.extend(
            [
                f'<circle cx="{axis_x}" cy="{y}" r="5" fill="#475569" />',
                f'<rect class="phase" x="{phase_x}" y="{y - 24}" width="190" height="48" rx="4" />',
                f'<text x="{phase_x + 12}" y="{y - 4}">{_xml_escape(phase.label)}</text>',
                f'<text x="{phase_x + 12}" y="{y + 15}" font-size="12">{_xml_escape(phase.kind)}</text>',
                f'<line class="edge" x1="{axis_x + 6}" y1="{y}" x2="{phase_x}" y2="{y}" />',
            ]
        )

    for event_id, (x, y) in event_positions.items():
        node = view.nodes[event_id]
        lines.extend(
            [
                f'<rect class="event" x="{x}" y="{y - 16}" width="210" height="32" rx="4" />',
                f'<text x="{x + 10}" y="{y + 5}">{_xml_escape(node.label)}</text>',
            ]
        )

    for edge in view.edges:
        if edge.kind == "has_event" and edge.target in event_positions and edge.source in y_positions:
            source_y = y_positions[edge.source]
            target_x, target_y = event_positions[edge.target]
            lines.append(
                f'<line class="edge" x1="{phase_x + 190}" y1="{source_y}" x2="{target_x}" y2="{target_y}" />'
            )
        elif edge.kind == "drives" and edge.source in event_positions and edge.target in event_positions:
            sx, sy = event_positions[edge.source]
            tx, ty = event_positions[edge.target]
            lines.append(
                f'<path class="edge drive" d="M {sx + 210} {sy} C {sx + 270} {sy}, {tx - 70} {ty}, {tx} {ty}" />'
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


def _event_node_id(object_name: str, event_name: str) -> str:
    return f"{object_name}.{event_name}"


def _add_event_node(
    nodes: dict[str, ViewNode], node_id: str, object_name: str, event_name: str
) -> None:
    if node_id not in nodes:
        nodes[node_id] = ViewNode(
            id=node_id,
            label=f"{object_name}.{event_name}",
            kind="Event",
        )


def _xml_escape(value: str) -> str:
    return (
        value.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace('"', "&quot;")
    )


__all__ = [
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
