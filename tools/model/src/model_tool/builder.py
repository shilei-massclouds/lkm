"""Static model builder for parsed LKM specs."""

from __future__ import annotations

import re
from common.model_types import (
    BuildResult,
    Diagnostic,
    EventDef,
    ObjectDef,
    ObjectModel,
    Severity,
    StateDef,
)
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
)


_OBJECT_EVENT_RE = re.compile(r"\b([A-Z][A-Za-z0-9_]*)\.Event::([A-Za-z_][A-Za-z0-9_]*)\b")
_OBJECT_STATE_RE = re.compile(
    r"\b([A-Z][A-Za-z0-9_]*)\.state\s*==\s*State::([A-Za-z_][A-Za-z0-9_]*)\b"
)
_ATTR_RE = re.compile(r"\A([A-Za-z_][A-Za-z0-9_]*)\s*:\s*(.+)\Z", re.S)


def build_model(document: SpecDocument) -> BuildResult:
    """Build an indexed static model and run first-pass checks."""

    diagnostics: list[Diagnostic] = []

    enums = _index_by_name(document.enums, "enum", diagnostics)
    functions = _index_overloads(document.functions)
    predicates = _index_overloads(document.predicates)
    types = _index_by_name(document.types, "type", diagnostics)
    objects = _build_objects(document.objects, diagnostics)
    children = _build_children(objects, diagnostics)

    for parent, child_names in children.items():
        parent_obj = objects.get(parent)
        if parent_obj is None:
            continue
        parent_obj.children.extend(child_names)

    model = ObjectModel(
        enums=enums,
        functions=functions,
        predicates=predicates,
        types=types,
        objects=objects,
        children=children,
    )

    _check_initial_states(model, diagnostics)
    _check_event_targets(model, diagnostics)
    _check_references(model, diagnostics)

    return BuildResult(model=model, diagnostics=diagnostics)


def summarize_model(result: BuildResult) -> str:
    """Return a human-readable model summary."""

    model = result.model
    status = "ok" if result.ok else "failed"
    return "\n".join(
        [
            f"model: {status}",
            f"objects: {len(model.objects)}",
            f"states: {model.state_count}",
            f"events: {model.event_count}",
            f"errors: {len(result.errors)}",
            f"warnings: {len(result.warnings)}",
        ]
    )


def _index_by_name(items, kind: str, diagnostics: list[Diagnostic]) -> dict[str, object]:
    indexed: dict[str, object] = {}
    for item in items:
        existing = indexed.get(item.name)
        if existing is not None:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"duplicate {kind} declaration: {item.name}",
                    item.span,
                )
            )
            continue
        indexed[item.name] = item
    return indexed


def _index_overloads(items) -> dict[str, list[object]]:
    indexed: dict[str, list[object]] = {}
    for item in items:
        indexed.setdefault(item.name, []).append(item)
    return indexed


def _build_objects(
    declarations: list[ObjectDecl], diagnostics: list[Diagnostic]
) -> dict[str, ObjectDef]:
    objects: dict[str, ObjectDef] = {}
    for decl in declarations:
        if decl.name in objects:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"duplicate object declaration: {decl.name}",
                    decl.span,
                )
            )
            continue

        states = _build_states(decl, diagnostics)
        attrs = _extract_attrs(decl, diagnostics)
        objects[decl.name] = ObjectDef(
            name=decl.name,
            kind=decl.kind,
            decl=decl,
            initial_state=decl.initial_state,
            parent=decl.parent,
            states=states,
            attrs=attrs,
        )
    return objects


def _build_states(decl: ObjectDecl, diagnostics: list[Diagnostic]) -> dict[str, StateDef]:
    states: dict[str, StateDef] = {}
    for state_decl in decl.states:
        if state_decl.name in states:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"duplicate state declaration: {decl.name}.State::{state_decl.name}",
                    state_decl.span,
                )
            )
            continue

        events = _build_events(decl.name, state_decl, diagnostics)
        states[state_decl.name] = StateDef(
            name=state_decl.name,
            object_name=decl.name,
            decl=state_decl,
            events=events,
        )
    return states


def _build_events(
    object_name: str, state_decl: StateDecl, diagnostics: list[Diagnostic]
) -> dict[str, EventDef]:
    events: dict[str, EventDef] = {}
    for event_decl in state_decl.events:
        if event_decl.name in events:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    "duplicate event declaration: "
                    f"{object_name}.State::{state_decl.name}.Event::{event_decl.name}",
                    event_decl.span,
                )
            )
            continue

        events[event_decl.name] = EventDef(
            name=event_decl.name,
            object_name=object_name,
            source_state=state_decl.name,
            target_state=event_decl.target_state,
            decl=event_decl,
        )
    return events


def _extract_attrs(decl: ObjectDecl, diagnostics: list[Diagnostic]) -> dict[str, str]:
    attrs: dict[str, str] = {}
    for block in decl.attrs:
        for entry in block.entries:
            match = _ATTR_RE.match(entry)
            if not match:
                diagnostics.append(
                    Diagnostic(
                        Severity.WARNING,
                        f"cannot parse attribute entry on {decl.name}: {entry}",
                        block.span,
                    )
                )
                continue

            name = match.group(1)
            if name in attrs:
                diagnostics.append(
                    Diagnostic(
                        Severity.ERROR,
                        f"duplicate attribute declaration: {decl.name}.{name}",
                        block.span,
                    )
                )
                continue
            attrs[name] = match.group(2).strip()
    return attrs


def _build_children(
    objects: dict[str, ObjectDef], diagnostics: list[Diagnostic]
) -> dict[str, list[str]]:
    children: dict[str, list[str]] = {name: [] for name in objects}
    for obj in objects.values():
        if obj.parent is None:
            continue
        if obj.parent not in objects:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"unknown parent object for {obj.name}: {obj.parent}",
                    obj.decl.span,
                )
            )
            continue
        children[obj.parent].append(obj.name)
    return children


def _check_initial_states(model: ObjectModel, diagnostics: list[Diagnostic]) -> None:
    for obj in model.objects.values():
        if obj.initial_state is None:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"object {obj.name} is missing initial_state",
                    obj.decl.span,
                )
            )
            continue
        if obj.initial_state not in obj.states:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"unknown initial_state on {obj.name}: State::{obj.initial_state}",
                    obj.decl.span,
                )
            )


def _check_event_targets(model: ObjectModel, diagnostics: list[Diagnostic]) -> None:
    for obj in model.objects.values():
        for state in obj.states.values():
            for event in state.events.values():
                if event.target_state not in obj.states:
                    diagnostics.append(
                        Diagnostic(
                            Severity.ERROR,
                            "unknown event target state: "
                            f"{obj.name}.Event::{event.name} -> State::{event.target_state}",
                            event.decl.span,
                        )
                    )


def _check_references(model: ObjectModel, diagnostics: list[Diagnostic]) -> None:
    for obj in model.objects.values():
        for state in obj.states.values():
            for block in state.decl.invariants:
                _check_state_references(model, block, diagnostics)
            for event in state.events.values():
                for block in event.decl.depends_on:
                    _check_state_references(model, block, diagnostics)
                for block in event.decl.drives:
                    _check_event_references(model, block, diagnostics)


def _check_event_references(
    model: ObjectModel, block: Block, diagnostics: list[Diagnostic]
) -> None:
    for object_name, event_name in _OBJECT_EVENT_RE.findall(block.body):
        obj = model.objects.get(object_name)
        if obj is None:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"unknown object in event reference: {object_name}.Event::{event_name}",
                    block.span,
                )
            )
            continue
        if not any(event_name in state.events for state in obj.states.values()):
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"unknown event reference: {object_name}.Event::{event_name}",
                    block.span,
                )
            )


def _check_state_references(
    model: ObjectModel, block: Block, diagnostics: list[Diagnostic]
) -> None:
    for object_name, state_name in _OBJECT_STATE_RE.findall(block.body):
        obj = model.objects.get(object_name)
        if obj is None:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"unknown object in state reference: {object_name}.state",
                    block.span,
                )
            )
            continue
        if state_name not in obj.states:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"unknown state reference: {object_name}.state == State::{state_name}",
                    block.span,
                )
            )


__all__ = [
    "BuildResult",
    "Diagnostic",
    "EventDef",
    "ObjectDef",
    "ObjectModel",
    "Severity",
    "StateDef",
    "build_model",
    "summarize_model",
]
