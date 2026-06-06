"""Static model builder for parsed LKM specs."""

from __future__ import annotations

import re
from common.model_types import (
    BuildResult,
    Diagnostic,
    EventDef,
    ExclusiveContextDef,
    ObjectDef,
    ObjectModel,
    Severity,
    StateDef,
)
from common.spec_ast import (
    Block,
    ExclusiveContextDecl,
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
_OBJECT_ACTION_RE = re.compile(r"\b([A-Z][A-Za-z0-9_]*)\.Action::([A-Za-z_][A-Za-z0-9_]*)\b")
_LOCK_EVENT_RE = re.compile(r"\b([A-Z][A-Za-z0-9_]*)\.Event::([A-Za-z_][A-Za-z0-9_]*)\b")
_OBJECT_STATE_RE = re.compile(
    r"\b([A-Z][A-Za-z0-9_]*)\.state\s*==\s*State::([A-Za-z_][A-Za-z0-9_]*)\b"
)
_TYPE_EVENT_RE_TEMPLATE = r"\bEvent::{}\b"
_ATTR_RE = re.compile(r"\A([A-Za-z_][A-Za-z0-9_]*)\s*:\s*(.+)\Z", re.S)
_ALLOWED_STATE_NAMES = frozenset(
    {
        "Base",
        "Prepared",
        "Ready",
        "Online",
        "Offline",
        "Destroyed",
    }
)
_ALLOWED_EVENT_NAMES = frozenset(
    {
        "Preset",
        "Setup",
        "Enable",
        "Disable",
        "Cleanup",
    }
)
_ALLOWED_TRANSITIONS = frozenset(
    {
        ("Base", "Preset", "Prepared"),
        ("Base", "Preset", "Ready"),
        ("Base", "Setup", "Ready"),
        ("Prepared", "Setup", "Ready"),
        ("Prepared", "Enable", "Online"),
        ("Ready", "Enable", "Online"),
        ("Ready", "Cleanup", "Destroyed"),
        ("Online", "Disable", "Offline"),
        ("Online", "Cleanup", "Destroyed"),
        ("Offline", "Cleanup", "Destroyed"),
    }
)


def build_model(document: SpecDocument) -> BuildResult:
    """Build an indexed static model and run first-pass checks."""

    diagnostics: list[Diagnostic] = []

    enums = _index_by_name(document.enums, "enum", diagnostics)
    functions = _index_overloads(document.functions)
    predicates = _index_overloads(document.predicates)
    types = _index_by_name(document.types, "type", diagnostics)
    locks = _index_by_name(document.locks, "lock", diagnostics)
    exclusive_contexts = _build_exclusive_contexts(
        document.exclusive_contexts, locks, diagnostics
    )
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
        locks=locks,
        exclusive_contexts=exclusive_contexts,
        objects=objects,
        children=children,
    )

    _check_initial_states(model, diagnostics)
    _check_event_targets(model, diagnostics)
    _check_lock_references(model, diagnostics)
    _check_exclusive_context_references(model, diagnostics)
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




def _build_exclusive_contexts(
    declarations: list[ExclusiveContextDecl],
    locks: dict[str, object],
    diagnostics: list[Diagnostic],
) -> dict[str, ExclusiveContextDef]:
    contexts: dict[str, ExclusiveContextDef] = {}
    for decl in declarations:
        if decl.name in contexts:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"duplicate exclusive_context declaration: {decl.name}",
                    decl.span,
                )
            )
            continue
        if decl.lock_ref is None:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"exclusive_context {decl.name} is missing lock_ref",
                    decl.span,
                )
            )
        elif decl.lock_ref not in locks:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"unknown lock_ref on exclusive_context {decl.name}: {decl.lock_ref}",
                    decl.span,
                )
            )
        if not decl.obj_refs:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"exclusive_context {decl.name} must reference at least one object",
                    decl.span,
                )
            )
        contexts[decl.name] = ExclusiveContextDef(
            name=decl.name,
            decl=decl,
            lock_ref=decl.lock_ref,
            obj_refs=tuple(decl.obj_refs),
        )
    return contexts


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

        _check_object_lifecycle_names(decl, diagnostics)
        _check_object_event_uniqueness(decl, diagnostics)
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


def _check_object_lifecycle_names(
    decl: ObjectDecl, diagnostics: list[Diagnostic]
) -> None:
    """Enforce SEM-NAME-001: lifecycle names come from controlled vocabularies."""

    if decl.initial_state is not None and decl.initial_state not in _ALLOWED_STATE_NAMES:
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"unknown lifecycle state name: {decl.name}.initial_state State::{decl.initial_state}",
                decl.span,
            )
        )

    for state_decl in decl.states:
        if state_decl.name not in _ALLOWED_STATE_NAMES:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"unknown lifecycle state name: {decl.name}.State::{state_decl.name}",
                    state_decl.span,
                )
            )
        for event_decl in state_decl.events:
            if event_decl.name not in _ALLOWED_EVENT_NAMES:
                diagnostics.append(
                    Diagnostic(
                        Severity.ERROR,
                        f"unknown lifecycle event name: {decl.name}.Event::{event_decl.name}",
                        event_decl.span,
                    )
                )
            transition = (state_decl.name, event_decl.name, event_decl.target_state)
            if transition not in _ALLOWED_TRANSITIONS:
                diagnostics.append(
                    Diagnostic(
                        Severity.ERROR,
                        "invalid lifecycle transition: "
                        f"{decl.name}.State::{state_decl.name}.Event::{event_decl.name} -> State::{event_decl.target_state}",
                        event_decl.span,
                    )
                )
            if event_decl.target_state not in _ALLOWED_STATE_NAMES:
                diagnostics.append(
                    Diagnostic(
                        Severity.ERROR,
                        "unknown lifecycle state name: "
                        f"{decl.name}.Event::{event_decl.name} -> State::{event_decl.target_state}",
                        event_decl.span,
                    )
                )


def _check_object_event_uniqueness(
    decl: ObjectDecl, diagnostics: list[Diagnostic]
) -> None:
    """Enforce SEM-EVENT-001: event identity is object-local."""

    seen: dict[str, EventDecl] = {}
    for state_decl in decl.states:
        for event_decl in state_decl.events:
            existing = seen.get(event_decl.name)
            if existing is None:
                seen[event_decl.name] = event_decl
                continue
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    "duplicate object event declaration: "
                    f"{decl.name}.Event::{event_decl.name}",
                    event_decl.span,
                )
            )


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

        events = _build_events(decl.name, state_decl, diagnostics, object_wide=True)
        states[state_decl.name] = StateDef(
            name=state_decl.name,
            object_name=decl.name,
            decl=state_decl,
            events=events,
        )
    return states


def _build_events(
    object_name: str,
    state_decl: StateDecl,
    diagnostics: list[Diagnostic],
    *,
    object_wide: bool = False,
) -> dict[str, EventDef]:
    events: dict[str, EventDef] = {}
    for event_decl in state_decl.events:
        if not object_wide and event_decl.name in events:
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


def _check_exclusive_context_references(
    model: ObjectModel, diagnostics: list[Diagnostic]
) -> None:
    for context in model.exclusive_contexts.values():
        for object_name in context.obj_refs:
            if object_name not in model.objects:
                diagnostics.append(
                    Diagnostic(
                        Severity.ERROR,
                        f"unknown object reference in exclusive_context {context.name}: {object_name}",
                        context.decl.span,
                )
            )


def _check_lock_references(model: ObjectModel, diagnostics: list[Diagnostic]) -> None:
    for lock in model.locks.values():
        if lock.kind is None:
            continue
        if lock.kind not in model.types:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"unknown lock type on lock {lock.name}: {lock.kind}",
                    lock.span,
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
                    _check_action_references(model, block, diagnostics)
                for within in event.decl.within:
                    _check_within_references(model, within, diagnostics)




def _check_within_references(model: ObjectModel, within, diagnostics: list[Diagnostic]) -> None:
    context = model.exclusive_contexts.get(within.context)
    if context is None:
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"unknown exclusive_context reference: {within.context}",
                within.span,
            )
        )
        return

    for block in within.entered_by:
        _check_lock_event_references(model, block, diagnostics, context=context)
    for block in within.depends_on:
        _check_state_references(model, block, diagnostics)
    for block in within.drives:
        _check_event_references(model, block, diagnostics)
        _check_action_references(model, block, diagnostics, context=context)
    for block in within.exited_by:
        _check_lock_event_references(model, block, diagnostics, context=context)


def _check_lock_event_references(
    model: ObjectModel,
    block: Block,
    diagnostics: list[Diagnostic],
    *,
    context: ExclusiveContextDef,
) -> None:
    for lock_name, event_name in _LOCK_EVENT_RE.findall(block.body):
        if lock_name != context.lock_ref:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    "lock event reference outside exclusive_context lock_ref: "
                    f"{lock_name}.Event::{event_name} not bound to {context.name}",
                    block.span,
                )
            )
            continue
        lock = model.locks.get(lock_name)
        if lock is None:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"unknown lock in event reference: {lock_name}.Event::{event_name}",
                    block.span,
                )
            )
            continue
        if lock.kind is None:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"lock event reference requires typed lock: {lock_name}.Event::{event_name}",
                    block.span,
                )
            )
            continue
        lock_type = model.types.get(lock.kind)
        if lock_type is None:
            continue
        if not _type_declares_event(lock_type, event_name):
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"unknown lock type event reference: {lock_name}.Event::{event_name}",
                    block.span,
                )
            )


def _check_action_references(
    model: ObjectModel,
    block: Block,
    diagnostics: list[Diagnostic],
    *,
    context: ExclusiveContextDef | None = None,
) -> None:
    allowed = set(context.obj_refs) if context is not None else None
    for object_name, action_name in _OBJECT_ACTION_RE.findall(block.body):
        if object_name not in model.objects:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"unknown object in action reference: {object_name}.Action::{action_name}",
                    block.span,
                )
            )
            continue
        if allowed is not None and object_name not in allowed:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    "action reference outside exclusive_context obj_refs: "
                    f"{object_name}.Action::{action_name} not in {context.name}",
                    block.span,
                )
            )


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
        if not any(event_name in state.events for state in obj.states.values()) and not (
            obj.kind in model.types
            and _type_declares_event(model.types[obj.kind], event_name)
        ):
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"unknown event reference: {object_name}.Event::{event_name}",
                    block.span,
                )
            )


def _type_declares_event(type_decl: TypeDecl, event_name: str) -> bool:
    pattern = re.compile(_TYPE_EVENT_RE_TEMPLATE.format(re.escape(event_name)))
    return any(pattern.search(block.body) for block in type_decl.blocks)


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
