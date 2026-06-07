"""Static model builder for parsed LKM specs."""

from __future__ import annotations

from dataclasses import dataclass
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
_OBJECT_EVENT_EXPR_RE = re.compile(
    r"\A([A-Z][A-Za-z0-9_]*)\.Event::([A-Za-z_][A-Za-z0-9_]*)(?:\s*\((.*)\))?\Z",
    re.S,
)
_OBJECT_ACTION_EXPR_RE = re.compile(
    r"\A([A-Z][A-Za-z0-9_]*)\.Action::([A-Za-z_][A-Za-z0-9_]*)(?:\s*\((.*)\))?\Z",
    re.S,
)
_ACTION_BIND_RE = re.compile(
    r"\Alet\s+([a-z][A-Za-z0-9_]*)\s*:\s*([A-Z][A-Za-z0-9_]*)\s*<-\s*"
    r"([A-Z][A-Za-z0-9_]*)\.Action::([A-Za-z_][A-Za-z0-9_]*)(?:\s*\((.*)\))?\Z",
    re.S,
)
_REF_EVENT_RE = re.compile(r"\b([a-z][A-Za-z0-9_]*)\.Event::([A-Za-z_][A-Za-z0-9_]*)\b")
_REF_EVENT_EXPR_RE = re.compile(
    r"\A([a-z][A-Za-z0-9_]*)\.Event::([A-Za-z_][A-Za-z0-9_]*)(?:\s*\((.*)\))?\Z",
    re.S,
)
_LOCK_EVENT_RE = re.compile(r"\b([A-Z][A-Za-z0-9_]*)\.Event::([A-Za-z_][A-Za-z0-9_]*)\b")
_OBJECT_STATE_RE = re.compile(
    r"\b([A-Z][A-Za-z0-9_]*)\.state\s*==\s*State::([A-Za-z_][A-Za-z0-9_]*)\b"
)
_TYPE_EVENT_RE_TEMPLATE = r"\bEvent::{}\b"
_REF_TARGET_PROCESS_TYPES = {
    "RunQueueRef": "RunQueue",
    "TaskRef": "Task",
}
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
_CONTEXT_BOOLEAN_EFFECTS = ("interruptible", "preemptible", "sleepable")


@dataclass(frozen=True)
class _ContextEffects:
    interruptible: bool
    preemptible: bool
    sleepable: bool
    exclusive_refs: frozenset[str]


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
        if decl.kind == "ResourceExclusiveContext" and decl.lock_ref is None:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"exclusive_context {decl.name} is missing lock_ref",
                    decl.span,
                )
            )
        elif decl.lock_ref is not None and decl.lock_ref not in locks:
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
            kind=decl.kind,
            guard=decl.guard,
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
        if context.kind is not None and context.kind not in (
            "ResourceExclusiveContext",
            "Context",
        ):
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"unsupported context kind on {context.name}: {context.kind}",
                    context.decl.span,
                )
            )
        if context.kind == "ResourceExclusiveContext" and context.guard is None:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"ResourceExclusiveContext {context.name} is missing guard",
                    context.decl.span,
                )
            )
        if (
            context.kind in ("ResourceExclusiveContext", "Context")
            and not context.decl.effects
        ):
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"context {context.name} is missing effects",
                    context.decl.span,
                )
            )
        effects = _parse_context_effects(context, diagnostics)
        if context.kind == "ResourceExclusiveContext" and effects is not None:
            _check_resource_exclusive_context_effects(context, effects, diagnostics)
        if context.guard is not None:
            _check_context_guard_references(model, context, diagnostics)
        for object_name in context.obj_refs:
            if object_name not in model.objects:
                diagnostics.append(
                    Diagnostic(
                        Severity.ERROR,
                        f"unknown object reference in exclusive_context {context.name}: {object_name}",
                        context.decl.span,
                )
            )


def _check_context_guard_references(
    model: ObjectModel,
    context: ExclusiveContextDef,
    diagnostics: list[Diagnostic],
) -> None:
    guard = context.guard
    if guard is None:
        return
    if guard.kind not in ("RawSpinLockIrqSaveGuard", "PreemptionGuard"):
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"unsupported guard kind on context {context.name}: {guard.kind}",
                guard.span,
            )
        )
    if guard.kind == "PreemptionGuard":
        if guard.lock_ref is not None:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"PreemptionGuard on context {context.name} must not declare lock_ref",
                    guard.span,
                )
            )
        _check_event_blocks(model, guard.entered_by, diagnostics)
        _check_event_blocks(model, guard.exited_by, diagnostics)
        return

    if guard.lock_ref is None:
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"context guard on {context.name} is missing lock_ref",
                guard.span,
            )
        )
    elif guard.lock_ref != context.lock_ref:
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"context {context.name} guard lock_ref does not match context lock_ref",
                guard.span,
            )
        )
    if guard.kind == "RawSpinLockIrqSaveGuard" and guard.lock_ref is not None:
        lock = model.locks.get(guard.lock_ref)
        if lock is not None and lock.kind != "RawSpinLock":
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    "RawSpinLockIrqSaveGuard requires RawSpinLock lock_ref: "
                    f"{guard.lock_ref}",
                    guard.span,
                )
            )
    _check_lock_event_blocks(model, guard.entered_by, diagnostics, context=context)
    _check_lock_event_blocks(model, guard.exited_by, diagnostics, context=context)


def _check_event_blocks(
    model: ObjectModel, blocks: list[Block], diagnostics: list[Diagnostic]
) -> None:
    for block in blocks:
        _check_event_references(model, block, diagnostics)


def _parse_context_effects(
    context: ExclusiveContextDef, diagnostics: list[Diagnostic]
) -> _ContextEffects | None:
    entries: dict[str, str] = {}
    for block in context.decl.effects:
        for entry, _span in block.entry_spans:
            match = _ATTR_RE.match(entry)
            if match is not None:
                entries[match.group(1)] = match.group(2).strip()
    if not entries:
        return None

    bools: dict[str, bool] = {}
    for key in _CONTEXT_BOOLEAN_EFFECTS:
        raw = entries.get(key)
        if raw not in ("true", "false"):
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"context effect must declare {key}: true|false on {context.name}",
                    context.decl.span,
                )
            )
            return None
        bools[key] = raw == "true"

    raw_exclusive_refs = entries.get("exclusive_refs")
    if raw_exclusive_refs == "obj_refs":
        exclusive_refs = frozenset(context.obj_refs)
    elif raw_exclusive_refs == "none":
        exclusive_refs = frozenset()
    else:
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                "context effect must declare exclusive_refs: obj_refs|none "
                f"on {context.name}",
                context.decl.span,
            )
        )
        return None

    return _ContextEffects(
        interruptible=bools["interruptible"],
        preemptible=bools["preemptible"],
        sleepable=bools["sleepable"],
        exclusive_refs=exclusive_refs,
    )


def _check_resource_exclusive_context_effects(
    context: ExclusiveContextDef,
    effects: _ContextEffects,
    diagnostics: list[Diagnostic],
) -> None:
    required = {
        "interruptible": False,
        "preemptible": False,
        "sleepable": False,
    }
    for key, value in required.items():
        if getattr(effects, key) != value:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    "ResourceExclusiveContext effect must declare "
                    f"{key}: {str(value).lower()}",
                    context.decl.span,
                )
            )
    if effects.exclusive_refs != frozenset(context.obj_refs):
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                "ResourceExclusiveContext effect must declare exclusive_refs: obj_refs",
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
                bindings: dict[str, str] = {}
                for block in event.decl.drives:
                    _check_drive_references(
                        model, block, diagnostics, bindings=bindings
                    )
                for within in event.decl.within:
                    _check_within_references(model, within, diagnostics)




def _check_within_references(
    model: ObjectModel,
    within,
    diagnostics: list[Diagnostic],
    *,
    inherited_bindings: dict[str, str] | None = None,
    inherited_effects: _ContextEffects | None = None,
) -> None:
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
    context_effects = _parse_context_effects(context, diagnostics)
    cumulative_effects = inherited_effects
    if context_effects is not None:
        if inherited_effects is not None:
            _check_context_nesting_effects(
                parent_effects=inherited_effects,
                child_effects=context_effects,
                child_context=context,
                span=within.span,
                diagnostics=diagnostics,
            )
        cumulative_effects = _compose_context_effects(
            inherited_effects, context_effects
        )

    if context.guard is not None and within.entered_by:
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"within {within.context} must not override context guard entered_by",
                within.entered_by[0].span,
            )
        )
    if context.guard is not None and within.exited_by:
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"within {within.context} must not override context guard exited_by",
                within.exited_by[0].span,
            )
        )

    _check_lock_event_blocks(model, within.entered_by, diagnostics, context=context)
    for block in within.depends_on:
        _check_state_references(model, block, diagnostics)
    bindings: dict[str, str] = dict(inherited_bindings or {})
    bindings.update(_within_parameter_bindings(within.parameters, bindings))
    for block in within.drives:
        _check_drive_references(model, block, diagnostics, context=context, bindings=bindings)
    for child_within in within.within:
        for name, value in child_within.parameters.items():
            if value not in bindings and not _is_known_ref_value(value):
                diagnostics.append(
                    Diagnostic(
                        Severity.ERROR,
                        f"unknown within parameter value: {name}: {value}",
                        child_within.span,
                    )
                )
        _check_within_references(
            model,
            child_within,
            diagnostics,
            inherited_bindings=bindings,
            inherited_effects=cumulative_effects,
        )
    _check_lock_event_blocks(model, within.exited_by, diagnostics, context=context)


def _check_context_nesting_effects(
    *,
    parent_effects: _ContextEffects,
    child_effects: _ContextEffects,
    child_context: ExclusiveContextDef,
    span: SourceSpan,
    diagnostics: list[Diagnostic],
) -> None:
    for key in _CONTEXT_BOOLEAN_EFFECTS:
        if getattr(parent_effects, key) is False and getattr(child_effects, key) is True:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    "invalid context nesting: inner context "
                    f"{child_context.name} weakens {key} from false to true",
                    span,
                )
            )


def _compose_context_effects(
    parent: _ContextEffects | None, child: _ContextEffects
) -> _ContextEffects:
    if parent is None:
        return child
    return _ContextEffects(
        interruptible=parent.interruptible and child.interruptible,
        preemptible=parent.preemptible and child.preemptible,
        sleepable=parent.sleepable and child.sleepable,
        exclusive_refs=parent.exclusive_refs | child.exclusive_refs,
    )


def _check_lock_event_blocks(
    model: ObjectModel,
    blocks: list[Block],
    diagnostics: list[Diagnostic],
    *,
    context: ExclusiveContextDef,
) -> None:
    for block in blocks:
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


def _check_drive_references(
    model: ObjectModel,
    block: Block,
    diagnostics: list[Diagnostic],
    *,
    context: ExclusiveContextDef | None = None,
    bindings: dict[str, str],
) -> None:
    for entry, entry_span in block.entry_spans:
        bind = _ACTION_BIND_RE.match(entry)
        if bind is not None:
            name, type_name, object_name, action_name, args = bind.group(1, 2, 3, 4, 5)
            _check_action_reference(
                model,
                object_name,
                action_name,
                diagnostics,
                entry_span,
                context=context,
            )
            if type_name != _action_return_type(model, object_name, action_name):
                diagnostics.append(
                    Diagnostic(
                        Severity.ERROR,
                        "action result binding type mismatch: "
                        f"{name}: {type_name} <- {object_name}.Action::{action_name}",
                        entry_span,
                    )
                )
            bindings[name] = type_name
            obj = model.objects.get(object_name)
            if obj is not None:
                _check_process_arguments(
                    model,
                    obj.kind,
                    "Action",
                    action_name,
                    args,
                    diagnostics,
                    entry_span,
                )
            continue
        if _check_drive_event_entry(
            model, entry, entry_span, diagnostics, bindings=bindings
        ):
            continue
        if _check_drive_action_entry(
            model, entry, entry_span, diagnostics, context=context
        ):
            continue
        _check_event_references(model, block, diagnostics, bindings=bindings)
        _check_action_references(model, block, diagnostics, context=context)


def _check_drive_event_entry(
    model: ObjectModel,
    entry: str,
    span: SourceSpan,
    diagnostics: list[Diagnostic],
    *,
    bindings: dict[str, str],
) -> bool:
    ref_event = _REF_EVENT_EXPR_RE.match(entry)
    if ref_event is not None:
        receiver_name, event_name, args = ref_event.group(1, 2, 3)
        receiver_type = bindings.get(receiver_name)
        if receiver_type is None:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"unknown ref binding in event reference: {receiver_name}.Event::{event_name}",
                    span,
                )
            )
            return True
        if not _is_supported_ref_type_event(receiver_type, event_name):
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    "unsupported ref event reference: "
                    f"{receiver_name}: {receiver_type}.Event::{event_name}",
                    span,
                )
            )
            return True
        process_type = _REF_TARGET_PROCESS_TYPES.get(receiver_type)
        if process_type is not None:
            _check_process_arguments(
                model,
                process_type,
                "Event",
                event_name,
                args,
                diagnostics,
                span,
            )
        return True

    match = _OBJECT_EVENT_EXPR_RE.match(entry)
    if match is None:
        return False
    object_name, event_name, args = match.group(1, 2, 3)
    obj = model.objects.get(object_name)
    if obj is None:
        if _is_supported_ref_event(object_name, event_name):
            return True
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"unknown object in event reference: {object_name}.Event::{event_name}",
                span,
            )
        )
        return True
    if not any(event_name in state.events for state in obj.states.values()) and not (
        obj.kind in model.types and _type_declares_event(model.types[obj.kind], event_name)
    ):
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"unknown event reference: {object_name}.Event::{event_name}",
                span,
            )
        )
        return True
    _check_process_arguments(
        model,
        obj.kind,
        "Event",
        event_name,
        args,
        diagnostics,
        span,
    )
    return True


def _check_drive_action_entry(
    model: ObjectModel,
    entry: str,
    span: SourceSpan,
    diagnostics: list[Diagnostic],
    *,
    context: ExclusiveContextDef | None = None,
) -> bool:
    action = _OBJECT_ACTION_EXPR_RE.match(entry)
    if action is None:
        return False
    object_name, action_name, args = action.group(1, 2, 3)
    _check_action_reference(
        model,
        object_name,
        action_name,
        diagnostics,
        span,
        context=context,
    )
    obj = model.objects.get(object_name)
    if obj is not None:
        _check_process_arguments(
            model,
            obj.kind,
            "Action",
            action_name,
            args,
            diagnostics,
            span,
        )
    return True


def _check_action_reference(
    model: ObjectModel,
    object_name: str,
    action_name: str,
    diagnostics: list[Diagnostic],
    span: SourceSpan,
    *,
    context: ExclusiveContextDef | None = None,
) -> None:
    allowed = set(context.obj_refs) if context is not None else None
    if object_name not in model.objects:
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"unknown object in action reference: {object_name}.Action::{action_name}",
                span,
            )
        )
        return
    if allowed is not None and object_name not in allowed:
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                "action reference outside exclusive_context obj_refs: "
                f"{object_name}.Action::{action_name} not in {context.name}",
                span,
            )
        )


def _check_event_references(
    model: ObjectModel,
    block: Block,
    diagnostics: list[Diagnostic],
    *,
    bindings: dict[str, str] | None = None,
) -> None:
    bindings = bindings or {}
    for receiver_name, event_name in _REF_EVENT_RE.findall(block.body):
        receiver_type = bindings.get(receiver_name)
        if receiver_type is None:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"unknown ref binding in event reference: {receiver_name}.Event::{event_name}",
                    block.span,
                )
            )
            continue
        if not _is_supported_ref_type_event(receiver_type, event_name):
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    "unsupported ref event reference: "
                    f"{receiver_name}: {receiver_type}.Event::{event_name}",
                    block.span,
                )
            )
        return
    for object_name, event_name in _OBJECT_EVENT_RE.findall(block.body):
        obj = model.objects.get(object_name)
        if obj is None:
            if _is_supported_ref_event(object_name, event_name):
                continue
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


def _is_supported_ref_event(receiver_name: str, event_name: str) -> bool:
    return receiver_name.endswith("RunQueueRef") and event_name == "EnqueueTask"


def _is_supported_ref_type_event(type_name: str, event_name: str) -> bool:
    return type_name == "RunQueueRef" and event_name == "EnqueueTask"


def _check_process_arguments(
    model: ObjectModel,
    type_name: str,
    process_kind: str,
    process_name: str,
    args: str | None,
    diagnostics: list[Diagnostic],
    span: SourceSpan,
) -> None:
    signature = _process_signature(model, type_name, process_kind, process_name)
    if signature is None:
        return
    if args is None:
        if signature:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    "missing process argument: "
                    f"{type_name}.{process_kind}::{process_name}",
                    span,
                )
            )
        return
    args = args.strip()
    if not args:
        if signature:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    "missing process argument: "
                    f"{type_name}.{process_kind}::{process_name}",
                    span,
                )
            )
        return
    if _uses_named_args(args):
        return
    if len(signature) != 1:
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                "positional process arguments require a single-parameter signature: "
                f"{type_name}.{process_kind}::{process_name}",
                span,
            )
        )
        return
    if "," in args:
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                "positional process arguments are only supported for one argument: "
                f"{type_name}.{process_kind}::{process_name}",
                span,
            )
        )


def _is_known_ref_value(value: str) -> bool:
    return value.endswith("Ref")


def _within_parameter_bindings(
    parameters: dict[str, str], inherited_bindings: dict[str, str]
) -> dict[str, str]:
    bindings: dict[str, str] = {}
    for name, value in parameters.items():
        if value in inherited_bindings:
            bindings[name] = inherited_bindings[value]
        elif value.endswith("RunQueueRef"):
            bindings[name] = "RunQueueRef"
        elif value.endswith("TaskRef"):
            bindings[name] = "TaskRef"
    return bindings


def _process_signature(
    model: ObjectModel, type_name: str, process_kind: str, process_name: str
) -> tuple[tuple[str, str], ...] | None:
    type_decl = model.types.get(type_name)
    if type_decl is None:
        return None
    pattern = re.compile(
        r"\b"
        + re.escape(process_kind)
        + r"::"
        + re.escape(process_name)
        + r"\s*(?:\(([^{};]*)\))?",
        re.S,
    )
    for block in type_decl.blocks:
        match = pattern.search(block.body)
        if match is not None:
            return _parse_process_parameters(match.group(1) or "")
    return None


def _parse_process_parameters(params: str) -> tuple[tuple[str, str], ...]:
    params = params.strip()
    if not params:
        return ()
    parsed: list[tuple[str, str]] = []
    for item in params.split(","):
        name, sep, type_name = item.strip().partition(":")
        if sep:
            parsed.append((name.strip(), type_name.strip()))
    return tuple(parsed)


def _uses_named_args(args: str) -> bool:
    return any(
        re.match(r"\s*[a-z][A-Za-z0-9_]*\s*:(?!:)", item) is not None
        for item in args.split(",")
    )


def _action_return_type(
    model: ObjectModel, object_name: str, action_name: str
) -> str | None:
    obj = model.objects.get(object_name)
    if obj is None:
        return None
    candidates: list[str] = []
    if obj.kind in model.types:
        candidates.extend(block.body for block in model.types[obj.kind].blocks)
    candidates.extend(block.body for block in obj.decl.other_blocks)
    pattern = re.compile(
        r"\bAction::"
        + re.escape(action_name)
        + r"\s*(?:\([^{};]*\))?\s*->\s*([A-Z][A-Za-z0-9_]*)\b",
        re.S,
    )
    for body in candidates:
        match = pattern.search(body)
        if match is not None:
            return match.group(1)
    return None


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
