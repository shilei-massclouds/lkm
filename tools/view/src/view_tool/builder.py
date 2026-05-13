"""Build view models from object models."""

from __future__ import annotations

import re

from common.model_types import EventDef, ObjectModel, StateDef
from common.view_types import TimelineItem, TimelineRow, ViewEdge, ViewModel, ViewNode


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
