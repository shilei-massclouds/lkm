"""Build view models from object models."""

from __future__ import annotations

import re
from typing import Any

from common.model_types import EventDef, ObjectModel, StateDef
from common.view_types import (
    TimelineItem,
    TimelineRow,
    TraceArrow,
    TraceCell,
    ViewEdge,
    ViewModel,
    ViewNode,
)


_OBJECT_EVENT_RE = re.compile(r"\b([A-Z][A-Za-z0-9_]*)\.Event::([A-Za-z_][A-Za-z0-9_]*)\b")
_OBJECT_STATE_RE = re.compile(
    r"\b([A-Z][A-Za-z0-9_]*)\.state\s*==\s*State::([A-Za-z_][A-Za-z0-9_]*)\b"
)


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
    phase_parents = {
        name: model.objects[name].parent for name in phase_objects if name in model.objects
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
        metadata={
            "timeline_rows": _build_timeline_rows(model, phase_objects),
            "phase_parents": phase_parents,
        },
    )


def build_trace_view(derive_data: dict[str, Any]) -> ViewModel:
    """Build a trace layout view from derive JSON."""

    builder = _TraceLayoutBuilder()
    roots = derive_data.get("trace", [])
    builder.build(roots, _verified_states_by_event(derive_data, roots))
    return ViewModel(
        name="trace",
        graph_format="text",
        metadata={
            "trace_columns": builder.columns,
            "trace_rows": builder.rows,
            "trace_cells": tuple(builder.cells),
            "trace_arrows": tuple(builder.arrows),
        },
    )


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


def _build_timeline_rows(
    model: ObjectModel, phase_objects: set[str]
) -> tuple[TimelineRow, ...]:
    states = {
        name: obj.initial_state
        for name, obj in model.objects.items()
        if obj.initial_state is not None
    }
    sequence: list[tuple[str, str, str, str, str]] = []
    rows_by_phase_state: dict[tuple[str, str], list[TimelineItem]] = {}
    row_order: list[tuple[str, str]] = []
    processed_events: set[tuple[str, str]] = set()

    def ensure_row(phase: str, state: str) -> None:
        row_key = (phase, state)
        if phase == "StartupTimeline" or row_key in rows_by_phase_state:
            return
        rows_by_phase_state[row_key] = []
        row_order.append(row_key)

    def process_event(
        object_name: str,
        event_name: str,
        current_phase: str,
        current_phase_state: str | None,
    ) -> None:
        obj = model.objects.get(object_name)
        if obj is None:
            return

        event = _find_event(obj, event_name, states.get(object_name))
        if event is None or event.decl.deferred:
            return

        key = (object_name, event_name)
        if key in processed_events:
            return
        processed_events.add(key)

        if object_name in phase_objects:
            next_phase = object_name
            next_phase_state = event.target_state
        else:
            next_phase = current_phase
            next_phase_state = current_phase_state

        for target_obj, target_event in _driven_events(event):
            process_event(target_obj, target_event, next_phase, next_phase_state)

        states[object_name] = event.target_state

        if object_name in phase_objects:
            ensure_row(object_name, event.target_state)
        elif current_phase_state is not None:
            sequence.append(
                (
                    current_phase,
                    current_phase_state,
                    object_name,
                    event.target_state,
                    event.name,
                )
            )

    process_event("StartupTimeline", "Setup", "StartupTimeline", None)

    final_by_object: dict[str, tuple[str, str, str, str]] = {}
    for phase, phase_state, object_name, target_state, event_name in sequence:
        final_by_object[object_name] = (phase, phase_state, target_state, event_name)

    placed_objects: set[str] = set()
    for phase, phase_state, object_name, _target_state, _event_name in sequence:
        final = final_by_object[object_name]
        if (phase, phase_state, _target_state, _event_name) != final:
            continue
        if object_name in placed_objects:
            continue

        row_key = (phase, phase_state)
        if row_key not in rows_by_phase_state:
            rows_by_phase_state[row_key] = []
            row_order.append(row_key)
        rows_by_phase_state[row_key].append(
            TimelineItem(
                object_name=object_name,
                detail=f"State::{final[2]}",
                kind="state",
            )
        )
        placed_objects.add(object_name)

    for phase, phase_state in row_order:
        for object_name, state_name in _phase_state_object_refs(model, phase, phase_state):
            if object_name in phase_objects or object_name in placed_objects:
                continue
            rows_by_phase_state[(phase, phase_state)].append(
                TimelineItem(
                    object_name=object_name,
                    detail=f"State::{state_name}",
                    kind="state",
                )
            )
            placed_objects.add(object_name)

    return tuple(
        _make_timeline_row(index, phase, state, rows_by_phase_state[(phase, state)])
        for index, (phase, state) in enumerate(row_order)
    )


def _phase_state_object_refs(
    model: ObjectModel, phase: str, phase_state: str
) -> list[tuple[str, str]]:
    obj = model.objects.get(phase)
    if obj is None:
        return []
    state = obj.states.get(phase_state)
    if state is None:
        return []

    refs: list[tuple[str, str]] = []
    seen: set[tuple[str, str]] = set()
    for block in state.decl.invariants:
        for object_name, state_name in _OBJECT_STATE_RE.findall(block.body):
            if state_name != phase_state:
                continue
            key = (object_name, state_name)
            if key in seen:
                continue
            seen.add(key)
            refs.append(key)
    return refs


def _make_timeline_row(
    index: int, phase: str, state: str, items: list[TimelineItem]
) -> TimelineRow:
    parent_phase = _parent_timeline_phase(phase)
    subphase = phase if parent_phase != phase else None
    return TimelineRow(
        id=f"{phase}.{state}.{index}",
        phase=parent_phase,
        subphase=subphase,
        label=state.lower(),
        detail=f"State::{state}",
        items=tuple(items),
    )


def _parent_timeline_phase(phase: str) -> str:
    if phase == "EntryPreludePhase":
        return "BootPhase"
    return phase


def _find_event(obj, event_name: str, current_state: str | None) -> EventDef | None:
    if current_state is not None:
        state = obj.states.get(current_state)
        if state is not None and event_name in state.events:
            return state.events[event_name]

    for state in obj.states.values():
        event = state.events.get(event_name)
        if event is not None:
            return event
    return None


def _driven_events(event: EventDef) -> list[tuple[str, str]]:
    driven: list[tuple[str, str]] = []
    for block in event.decl.drives:
        driven.extend(_OBJECT_EVENT_RE.findall(block.body))
    return driven


class _TraceLayoutBuilder:
    def __init__(self) -> None:
        self.columns: list[dict[str, object]] = []
        self.rows: list[dict[str, object]] = []
        self.cells: list[TraceCell] = []
        self.arrows: list[TraceArrow] = []
        self._event_index = 0
        self._max_phase_lane = 0
        self._max_object_lane = -1
        self._object_column_base = 0

    def build(
        self,
        roots: list[Any],
        verified_states_by_event: dict[tuple[str, str], list[tuple[str, str]]],
    ) -> None:
        self._max_phase_lane = _max_trace_phase_lane(roots, verified_states_by_event)
        self._object_column_base = self._max_phase_lane + 2
        for node in roots:
            if _should_skip_trace_node(node, verified_states_by_event):
                continue
            self._place_node(
                node,
                phase_lane=0,
                object_lane=0,
                parent_event_id=None,
                verified_states_by_event=verified_states_by_event,
            )
        self._build_columns()

    def _place_node(
        self,
        node: Any,
        *,
        phase_lane: int,
        object_lane: int,
        parent_event_id: str | None,
        verified_states_by_event: dict[tuple[str, str], list[tuple[str, str]]],
    ) -> None:
        data = _trace_node_object(node)
        is_phase = _is_trace_phase_object(str(data["object"]))
        index = self._event_index
        self._event_index += 1
        label = _trace_label(data)
        event_id = f"event-{index}"
        enter_id = f"{event_id}-source"
        exit_id = f"{event_id}-target"
        span_id = f"{event_id}-span"
        column = phase_lane if is_phase else self._object_column(object_lane)
        gap_column = None if is_phase else self._object_gap_column(object_lane)
        self._max_phase_lane = max(self._max_phase_lane, phase_lane)
        if not is_phase:
            self._max_object_lane = max(self._max_object_lane, object_lane)

        source_label = f"{data['object']}.State::{data['source_state']}"
        event_row = len(self.rows)
        adjacent_source_id = self._adjacent_state_cell_id(column, source_label)
        if adjacent_source_id is not None:
            enter_id = adjacent_source_id
            event_row = self.rows[-1]["index"]
        else:
            self._add_row(
                "state",
                event_row,
                f"{label}.source",
                group_id=event_id if not is_phase else None,
                group_role="source" if not is_phase else None,
            )
            self.cells.append(
                TraceCell(
                    id=enter_id,
                    kind="state",
                    row=event_row,
                    column=column,
                    label=source_label,
                )
            )
            self._add_gap_cell(
                TraceCell(
                    id=f"{event_id}-vertical-gap-before",
                    kind="gap",
                    row=event_row,
                    column=gap_column if gap_column is not None else column,
                )
            )

        event_body_start = len(self.rows)
        self._add_row(
            "gap",
            event_body_start,
            f"{label}.body.start",
            group_id=event_id if not is_phase else None,
            group_role="body_start" if not is_phase else None,
        )
        self.cells.append(
            TraceCell(
                id=f"{event_id}-body-gap",
                kind="gap",
                row=event_body_start,
                column=column,
            )
        )
        self._add_gap_cell(
            TraceCell(
                id=f"{event_id}-drive-gap",
                kind="gap",
                row=event_body_start,
                column=gap_column if gap_column is not None else column,
            )
        )

        verified_states = verified_states_by_event.get(
            (str(data["object"]), str(data["event"])), []
        )
        verified_lane = object_lane if is_phase else object_lane + 1
        child_object_lane = object_lane if is_phase else object_lane + (
            2 if verified_states else 1
        )
        verified_column = self._object_column(verified_lane)
        verified_gap_column = self._object_gap_column(verified_lane)
        if verified_states:
            self._max_object_lane = max(self._max_object_lane, verified_lane)
        for verified_index, (object_name, state_name) in enumerate(verified_states):
            verified_id = f"{event_id}-verified-{verified_index}"
            verified_row = len(self.rows)
            self._add_row("state", verified_row, f"{label}.verified.{object_name}")
            self.cells.append(
                TraceCell(
                    id=verified_id,
                    kind="verified_state",
                    row=verified_row,
                    column=verified_column,
                    label=f"{object_name}.State::{state_name}",
                )
            )
            self.cells.append(
                TraceCell(
                    id=f"{verified_id}-gap",
                    kind="gap",
                    row=verified_row,
                    column=verified_gap_column,
                )
            )
            self.arrows.append(
                TraceArrow(source=span_id, target=verified_id, kind="depends_on")
            )

        for child in _trace_children(data):
            if _should_skip_trace_node(child, verified_states_by_event):
                continue
            child_event_id = f"event-{self._event_index}"
            self.arrows.append(
                TraceArrow(source=span_id, target=f"{child_event_id}-span", kind="drives")
            )
            child_data = _trace_node_object(child)
            child_is_phase = _is_trace_phase_object(str(child_data.get("object")))
            self._place_node(
                child,
                phase_lane=phase_lane + 1 if child_is_phase else phase_lane,
                object_lane=object_lane if child_is_phase else child_object_lane,
                parent_event_id=event_id,
                verified_states_by_event=verified_states_by_event,
            )

        event_exit_gap_row = len(self.rows)
        self._add_row(
            "gap",
            event_exit_gap_row,
            f"{label}.body.end",
            group_id=event_id if not is_phase else None,
            group_role="body_end" if not is_phase else None,
        )
        self.cells.append(
            TraceCell(
                id=f"{event_id}-body-end-gap",
                kind="gap",
                row=event_exit_gap_row,
                column=column,
            )
        )
        self._add_gap_cell(
            TraceCell(
                id=f"{event_id}-drive-end-gap",
                kind="gap",
                row=event_exit_gap_row,
                column=gap_column if gap_column is not None else column,
            )
        )

        exit_row = len(self.rows)
        self._add_row(
            "state",
            exit_row,
            f"{label}.target",
            group_id=event_id if not is_phase else None,
            group_role="target" if not is_phase else None,
        )
        self.cells.append(
            TraceCell(
                id=exit_id,
                kind="state",
                row=exit_row,
                column=column,
                label=f"{data['object']}.State::{data['target_state']}",
            )
        )
        self._add_gap_cell(
            TraceCell(
                id=f"{event_id}-vertical-gap-after",
                kind="gap",
                row=exit_row,
                column=gap_column if gap_column is not None else column,
            )
        )

        self.cells.append(
            TraceCell(
                id=span_id,
                kind="event_span",
                row=event_body_start,
                column=column,
                label=label,
                row_span=event_exit_gap_row - event_body_start + 1,
            )
        )
        self.arrows.append(TraceArrow(source=enter_id, target=exit_id, kind="state"))
        if parent_event_id is not None:
            self._add_gap_cell(
                TraceCell(
                    id=f"{parent_event_id}-to-{event_id}-gap",
                    kind="gap",
                    row=event_row,
                    column=gap_column if gap_column is not None else column,
                )
            )

    def _add_row(
        self,
        kind: str,
        index: int,
        label: str,
        *,
        group_id: str | None = None,
        group_role: str | None = None,
    ) -> None:
        row: dict[str, object] = {"index": index, "kind": kind, "label": label}
        if group_id is not None:
            row["group_id"] = group_id
        if group_role is not None:
            row["group_role"] = group_role
        self.rows.append(row)

    def _add_gap_cell(self, cell: TraceCell) -> None:
        if cell.column < self._object_column_base:
            return
        self.cells.append(cell)

    def _adjacent_state_cell_id(self, column: int, label: str) -> str | None:
        if not self.rows or self.rows[-1]["kind"] != "state":
            return None
        row = self.rows[-1]["index"]
        for cell in reversed(self.cells):
            if (
                cell.kind == "state"
                and cell.row == row
                and cell.column == column
                and cell.label == label
            ):
                return cell.id
        return None

    def _build_columns(self) -> None:
        for phase_lane in range(self._max_phase_lane + 1):
            self.columns.append(
                {
                    "index": phase_lane,
                    "kind": "phase",
                    "depth": phase_lane,
                }
            )
        self.columns.append(
            {
                "index": self._object_column_base - 1,
                "kind": "phase_object_gap",
                "depth": 0,
            }
        )
        for object_lane in range(self._max_object_lane + 1):
            self.columns.append(
                {
                    "index": self._object_column(object_lane),
                    "kind": "object",
                    "depth": object_lane,
                }
            )
            self.columns.append(
                {
                    "index": self._object_gap_column(object_lane),
                    "kind": "gap",
                    "depth": object_lane,
                }
            )

    def _object_column(self, object_lane: int) -> int:
        return self._object_column_base + object_lane * 2

    def _object_gap_column(self, object_lane: int) -> int:
        return self._object_column(object_lane) + 1


def _trace_node_object(node: Any) -> dict[str, Any]:
    if not isinstance(node, dict):
        raise ValueError("trace node must be an object")
    return node


def _trace_children(node: dict[str, Any]) -> list[Any]:
    children = node.get("children", [])
    return children if isinstance(children, list) else []


def _trace_label(node: dict[str, Any]) -> str:
    label = node.get("label")
    if isinstance(label, str):
        return label
    return f"{node.get('object')}.Event::{node.get('event')}"


def _should_skip_trace_node(
    node: Any, verified_states_by_event: dict[tuple[str, str], list[tuple[str, str]]]
) -> bool:
    data = _trace_node_object(node)
    object_name = data.get("object")
    event_name = data.get("event")
    if not isinstance(object_name, str) or not isinstance(event_name, str):
        return False
    if not _is_trace_phase_object(object_name):
        return False
    if _trace_children(data):
        return False
    return not verified_states_by_event.get((object_name, event_name))


def _is_trace_phase_object(object_name: str) -> bool:
    return object_name == "StartupTimeline" or object_name.endswith("Phase")


def _max_trace_phase_lane(
    roots: list[Any],
    verified_states_by_event: dict[tuple[str, str], list[tuple[str, str]]],
) -> int:
    max_lane = 0

    def visit(node: Any, phase_lane: int) -> None:
        nonlocal max_lane
        if _should_skip_trace_node(node, verified_states_by_event):
            return
        data = _trace_node_object(node)
        is_phase = _is_trace_phase_object(str(data.get("object")))
        current_phase_lane = phase_lane
        if is_phase:
            max_lane = max(max_lane, current_phase_lane)
        for child in _trace_children(data):
            child_data = _trace_node_object(child)
            child_phase_lane = (
                current_phase_lane + 1
                if _is_trace_phase_object(str(child_data.get("object")))
                else current_phase_lane
            )
            visit(child, child_phase_lane)

    for root in roots:
        visit(root, 0)
    return max_lane


def _verified_states_by_event(
    derive_data: dict[str, Any], roots: list[Any]
) -> dict[tuple[str, str], list[tuple[str, str]]]:
    transitioned_objects = _trace_transitioned_objects(roots)
    seen_states: set[tuple[str, str]] = set()
    verified: dict[tuple[str, str], list[tuple[str, str]]] = {}
    for record in derive_data.get("records", []):
        if not isinstance(record, dict):
            continue
        if record.get("status") != "proved":
            continue
        message = record.get("message")
        if not isinstance(message, str) or not message.startswith("depends_on: "):
            continue
        expression = record.get("expression")
        if not isinstance(expression, str):
            continue
        match = _OBJECT_STATE_RE.search(expression)
        if match is None:
            continue

        object_name, state_name = match.group(1), match.group(2)
        if object_name in transitioned_objects:
            continue
        state_key = (object_name, state_name)
        if state_key in seen_states:
            continue
        event_object = record.get("object")
        event_name = record.get("event")
        if not isinstance(event_object, str) or not isinstance(event_name, str):
            continue

        seen_states.add(state_key)
        verified.setdefault((event_object, event_name), []).append(state_key)
    return verified


def _trace_transitioned_objects(roots: list[Any]) -> set[str]:
    objects: set[str] = set()

    def visit(node: Any) -> None:
        if not isinstance(node, dict):
            return
        object_name = node.get("object")
        if isinstance(object_name, str):
            objects.add(object_name)
        for child in _trace_children(node):
            visit(child)

    for root in roots:
        visit(root)
    return objects
