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


_OBJECT_EVENT_RE = re.compile(r"\b([A-Z][A-Za-z0-9_]*)\.Event::([A-Za-z_][A-Za-z0-9_]*)\b")


def build_object_view(model: ObjectModel) -> ViewModel:
    """Build an object-level view with parent and drives edges."""

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

    for obj in model.objects.values():
        for state in obj.states.values():
            for event in state.events.values():
                for block in event.decl.drives:
                    for target_obj, target_event in _OBJECT_EVENT_RE.findall(block.body):
                        if target_obj not in model.objects:
                            continue
                        edges.append(
                            ViewEdge(
                                source=obj.name,
                                target=target_obj,
                                kind="drives",
                                label=f"drives {event.name}->{target_event}",
                            )
                        )

    return ViewModel(name="object", nodes=nodes, edges=edges)


def render_text(view: ViewModel) -> str:
    """Render a model view as plain text."""

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

    lines = [f"digraph {view.name.title()}View {{", "  rankdir=TB;"]
    for node in view.nodes.values():
        lines.append(
            f'  "{_dot_escape(node.id)}" '
            f'[label="{_dot_escape(node.label)}\\n{_dot_escape(node.kind)}"];'
        )

    for edge in view.edges:
        attrs = []
        if edge.label:
            attrs.append(f'label="{_dot_escape(edge.label)}"')
        if edge.kind == "drives":
            attrs.append('style="dashed"')
        attr_text = f" [{', '.join(attrs)}]" if attrs else ""
        lines.append(
            f'  "{_dot_escape(edge.source)}" -> "{_dot_escape(edge.target)}"{attr_text};'
        )

    lines.append("}")
    return "\n".join(lines)


def _dot_escape(value: str) -> str:
    return value.replace("\\", "\\\\").replace('"', '\\"')


__all__ = [
    "ViewEdge",
    "ViewModel",
    "ViewNode",
    "build_object_view",
    "render_dot",
    "render_text",
]
