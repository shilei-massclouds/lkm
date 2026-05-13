"""View model data types shared by LKM verification tools."""

from __future__ import annotations

from dataclasses import dataclass, field


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
class TimelineItem:
    """An object-level item placed on a timeline row."""

    object_name: str
    detail: str
    kind: str


@dataclass(frozen=True)
class TimelineRow:
    """One chronological row in the startup timeline view."""

    id: str
    phase: str
    subphase: str | None
    label: str
    detail: str
    items: tuple[TimelineItem, ...] = ()


@dataclass(frozen=True)
class TraceCell:
    """One occupied or empty cell in a trace layout grid."""

    id: str
    kind: str
    row: int
    column: int
    label: str = ""
    row_span: int = 1
    column_span: int = 1


@dataclass(frozen=True)
class TraceArrow:
    """A semantic arrow between trace layout cells."""

    source: str
    target: str
    kind: str


@dataclass(frozen=True)
class ViewModel:
    """A renderable model view."""

    name: str
    nodes: dict[str, ViewNode] = field(default_factory=dict)
    edges: list[ViewEdge] = field(default_factory=list)
    rankdir: str = "TB"
    graph_format: str = "dot"
    metadata: dict[str, object] = field(default_factory=dict)
