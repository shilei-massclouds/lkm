"""Static model builder for parsed LKM specs."""

from __future__ import annotations

from dataclasses import dataclass
import re
from common.model_types import (
    BuildResult,
    Diagnostic,
    TransitionDef,
    ExclusiveContextDef,
    ObjectDef,
    ObjectModel,
    Severity,
    StateDef,
)
from common.spec_ast import (
    BodyMember,
    Block,
    ExclusiveContextDecl,
    EnumDecl,
    TransitionDecl,
    FunctionDecl,
    ObjectDecl,
    PredicateDecl,
    SourceSpan,
    SpecDocument,
    StateDecl,
    TypeDecl,
)


_OBJECT_TRANSITION_RE = re.compile(r"\b([A-Z][A-Za-z0-9_]*)\.Transition::([A-Za-z_][A-Za-z0-9_]*)\b")
_OBJECT_ACTION_RE = re.compile(r"\b([A-Z][A-Za-z0-9_]*)\.Action::([A-Za-z_][A-Za-z0-9_]*)\b")
_OBJECT_TRANSITION_EXPR_RE = re.compile(
    r"\A([A-Z][A-Za-z0-9_]*)\.Transition::([A-Za-z_][A-Za-z0-9_]*)(?:\s*\((.*)\))?\Z",
    re.S,
)
_OBJECT_ACTION_EXPR_RE = re.compile(
    r"\A([A-Z][A-Za-z0-9_]*)\.Action::([A-Za-z_][A-Za-z0-9_]*)(?:\s*\((.*)\))?\Z",
    re.S,
)
_ACTION_BIND_RE = re.compile(
    r"\Alet\s+([a-z][A-Za-z0-9_]*)\s*:\s*([A-Z][A-Za-z0-9_]*)\s*<-\s*"
    r"([A-Za-z][A-Za-z0-9_]*)\.Action::([A-Za-z_][A-Za-z0-9_]*)(?:\s*\((.*)\))?\Z",
    re.S,
)
_REF_TRANSITION_RE = re.compile(r"\b([a-z][A-Za-z0-9_]*)\.Transition::([A-Za-z_][A-Za-z0-9_]*)\b")
_REF_TRANSITION_EXPR_RE = re.compile(
    r"\A([a-z][A-Za-z0-9_]*)\.Transition::([A-Za-z_][A-Za-z0-9_]*)(?:\s*\((.*)\))?\Z",
    re.S,
)
_REF_ACTION_EXPR_RE = re.compile(
    r"\A([a-z][A-Za-z0-9_]*)\.Action::([A-Za-z_][A-Za-z0-9_]*)(?:\s*\((.*)\))?\Z",
    re.S,
)
_LOCK_TRANSITION_RE = re.compile(r"\b([A-Z][A-Za-z0-9_]*)\.Transition::([A-Za-z_][A-Za-z0-9_]*)\b")
_LOCK_ACTION_RE = re.compile(r"\b([A-Z][A-Za-z0-9_]*)\.Action::([A-Za-z_][A-Za-z0-9_]*)\b")
_OBJECT_STATE_RE = re.compile(
    r"\b([A-Z][A-Za-z0-9_]*)\.state\s*==\s*State::([A-Za-z_][A-Za-z0-9_]*)\b"
)
_TYPE_TRANSITION_RE_TEMPLATE = r"\bTransition::{}\b"
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
_ALLOWED_TRANSITION_NAMES = frozenset(
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
_LEGACY_CONTEXT_BOOLEAN_EFFECTS = ("interruptible", "preemptible", "sleepable")
_CONTRIBUTION_KEYS = (
    "local_interrupts",
    "preemption",
    "voluntary_switching",
    "cpu_concurrency",
    "task_concurrency",
)
_LEGACY_EFFECT_KEY_MAP = {
    "interruptible": "local_interrupts",
    "preemptible": "preemption",
    "sleepable": "voluntary_switching",
}
_FALSE_HOLD_VALUES = frozenset({"false", "disabled", "closed", "single", "single_cpu", "single_task"})
_TRUE_HOLD_VALUES = frozenset({"true", "enabled", "open", "multi", "smp", "multi_cpu", "multi_task"})


@dataclass(frozen=True)
class _ContextContribution:
    local_interrupts: bool | None = None
    preemption: bool | None = None
    voluntary_switching: bool | None = None
    cpu_concurrency: bool | None = None
    task_concurrency: bool | None = None
    exclusive_refs: frozenset[str] = frozenset()


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
    _check_only_once_withins(model, diagnostics)

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
            f"transitions: {model.transition_count}",
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
        if decl.kind == "ResourceExclusiveContext" and not decl.obj_refs:
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
        for transition_decl in state_decl.transitions:
            if transition_decl.name not in _ALLOWED_TRANSITION_NAMES:
                diagnostics.append(
                    Diagnostic(
                        Severity.ERROR,
                        f"unknown lifecycle transition name: {decl.name}.Transition::{transition_decl.name}",
                        transition_decl.span,
                    )
                )
            transition = (state_decl.name, transition_decl.name, transition_decl.target_state)
            if transition not in _ALLOWED_TRANSITIONS:
                diagnostics.append(
                    Diagnostic(
                        Severity.ERROR,
                        "invalid lifecycle transition: "
                        f"{decl.name}.State::{state_decl.name}.Transition::{transition_decl.name} -> State::{transition_decl.target_state}",
                        transition_decl.span,
                    )
                )
            if transition_decl.target_state not in _ALLOWED_STATE_NAMES:
                diagnostics.append(
                    Diagnostic(
                        Severity.ERROR,
                        "unknown lifecycle state name: "
                        f"{decl.name}.Transition::{transition_decl.name} -> State::{transition_decl.target_state}",
                        transition_decl.span,
                    )
                )


def _check_object_event_uniqueness(
    decl: ObjectDecl, diagnostics: list[Diagnostic]
) -> None:
    """Enforce SEM-TRANSITION-001: transition identity is object-local."""

    seen: dict[str, TransitionDecl] = {}
    for state_decl in decl.states:
        for transition_decl in state_decl.transitions:
            existing = seen.get(transition_decl.name)
            if existing is None:
                seen[transition_decl.name] = transition_decl
                continue
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    "duplicate object transition declaration: "
                    f"{decl.name}.Transition::{transition_decl.name}",
                    transition_decl.span,
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

        transitions = _build_events(decl.name, state_decl, diagnostics, object_wide=True)
        states[state_decl.name] = StateDef(
            name=state_decl.name,
            object_name=decl.name,
            decl=state_decl,
            transitions=transitions,
        )
    return states


def _build_events(
    object_name: str,
    state_decl: StateDecl,
    diagnostics: list[Diagnostic],
    *,
    object_wide: bool = False,
) -> dict[str, TransitionDef]:
    transitions: dict[str, TransitionDef] = {}
    for transition_decl in state_decl.transitions:
        if not object_wide and transition_decl.name in transitions:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    "duplicate transition declaration: "
                    f"{object_name}.State::{state_decl.name}.Transition::{transition_decl.name}",
                    transition_decl.span,
                )
            )
            continue

        transitions[transition_decl.name] = TransitionDef(
            name=transition_decl.name,
            object_name=object_name,
            source_state=state_decl.name,
            target_state=transition_decl.target_state,
            decl=transition_decl,
        )
    return transitions


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
            for transition in state.transitions.values():
                if transition.target_state not in obj.states:
                    diagnostics.append(
                        Diagnostic(
                            Severity.ERROR,
                            "unknown transition target state: "
                            f"{obj.name}.Transition::{transition.name} -> State::{transition.target_state}",
                            transition.decl.span,
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
        if context.guard is not None:
            _check_context_guard_references(model, context, diagnostics)
        contribution = _derive_context_contribution(model, context, diagnostics)
        legacy_effects = _parse_legacy_context_effects(context, diagnostics)
        if legacy_effects is not None:
            _check_legacy_effects_compatible(
                context, derived=contribution, legacy=legacy_effects, diagnostics=diagnostics
            )
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
    has_entered_by = bool(guard.entered_by)
    has_exited_by = bool(guard.exited_by)
    has_holds = bool(guard.holds)
    if has_entered_by != has_exited_by:
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"context guard on {context.name} must pair entered_by with exited_by",
                guard.span,
            )
        )
    if has_entered_by and has_exited_by and has_holds:
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"context guard on {context.name} must not mix entered_by/exited_by with holds",
                guard.span,
            )
        )
    if not has_entered_by and not has_exited_by and not has_holds:
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"context guard on {context.name} must declare paired entered_by/exited_by or holds",
                guard.span,
            )
        )
    if guard.lock_ref is None and (has_entered_by or has_exited_by):
        _check_event_blocks(model, guard.entered_by, diagnostics)
        _check_event_blocks(model, guard.exited_by, diagnostics)
        return
    if guard.lock_ref is None:
        return
    elif guard.lock_ref != context.lock_ref:
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"context {context.name} guard lock_ref does not match context lock_ref",
                guard.span,
            )
        )
    if guard.lock_ref not in model.locks and guard.lock_ref not in model.objects:
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"unknown guard lock_ref on context {context.name}: {guard.lock_ref}",
                guard.span,
            )
        )
    if not guard.entered_by:
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"context guard on {context.name} is missing entered_by",
                guard.span,
            )
        )
    if not guard.exited_by:
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"context guard on {context.name} is missing exited_by",
                guard.span,
            )
        )
    _check_lock_event_blocks(model, guard.entered_by, diagnostics, context=context)
    _check_lock_event_blocks(model, guard.exited_by, diagnostics, context=context)


def _check_event_blocks(
    model: ObjectModel, blocks: list[Block], diagnostics: list[Diagnostic]
) -> None:
    for block in blocks:
        if _is_never_guard_block(block):
            continue
        _check_event_references(model, block, diagnostics)


def _is_never_guard_block(block: Block) -> bool:
    entries = [entry.strip() for entry, _span in block.entry_spans]
    if entries:
        return entries == ["Never"]
    return block.body.strip().rstrip(";").strip() == "Never"


def _derive_context_contribution(
    model: ObjectModel, context: ExclusiveContextDef, diagnostics: list[Diagnostic]
) -> _ContextContribution:
    contribution = _schema_context_contribution(model, context)
    hold_entries = _context_guard_holds(context)
    for key, raw in hold_entries.items():
        normalized = _normalize_hold_value(raw)
        if normalized is None:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"context guard hold has unsupported value on {context.name}: {key}: {raw}",
                    context.guard.span if context.guard is not None else context.decl.span,
                )
            )
            continue
        if key not in _CONTRIBUTION_KEYS:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"unsupported context guard hold on {context.name}: {key}",
                    context.guard.span if context.guard is not None else context.decl.span,
                )
            )
            continue
        contribution = _set_context_contribution_value(
            contribution, key, normalized, context=context, diagnostics=diagnostics
        )
    return contribution


def _schema_context_contribution(
    model: ObjectModel, context: ExclusiveContextDef
) -> _ContextContribution:
    guard = context.guard
    exclusive_refs = (
        frozenset(context.obj_refs)
        if context.kind == "ResourceExclusiveContext"
        else frozenset()
    )
    if guard is None:
        return _ContextContribution(exclusive_refs=exclusive_refs)
    contribution = _ContextContribution(exclusive_refs=exclusive_refs)
    boundary_events = _guard_boundary_events(model, guard.entered_by)
    if any(receiver_kind == "RawSpinLock" and process_kind == "Transition" and process_name == "LockIrqSave"
           for receiver_kind, process_kind, process_name in boundary_events):
        contribution = _replace_context_contribution(
            contribution,
            local_interrupts=False,
            preemption=False,
            voluntary_switching=False,
        )
    if any(receiver_kind == "PreemptionControl" and process_kind == "Transition" and process_name == "Disable"
           for receiver_kind, process_kind, process_name in boundary_events):
        contribution = _replace_context_contribution(
            contribution,
            preemption=False,
            voluntary_switching=False,
        )
    if any(receiver_kind == "LocalInterruptControl" and process_kind == "Transition" and process_name in {"Disable", "SaveAndDisable"}
           for receiver_kind, process_kind, process_name in boundary_events):
        contribution = _replace_context_contribution(
            contribution,
            local_interrupts=False,
        )
    return contribution


def _guard_boundary_events(model: ObjectModel, blocks: list[Block]) -> list[tuple[str, str, str]]:
    transitions: list[tuple[str, str, str]] = []
    for block in blocks:
        if _is_never_guard_block(block):
            continue
        for receiver_name, transition_name in _LOCK_TRANSITION_RE.findall(block.body):
            receiver_kind = _guard_receiver_kind(model, receiver_name)
            if receiver_kind is not None:
                transitions.append((receiver_kind, "Transition", transition_name))
        for receiver_name, action_name in _LOCK_ACTION_RE.findall(block.body):
            receiver_kind = _guard_receiver_kind(model, receiver_name)
            if receiver_kind is not None:
                transitions.append((receiver_kind, "Action", action_name))
    return transitions


def _guard_receiver_kind(model: ObjectModel, receiver_name: str) -> str | None:
    lock = model.locks.get(receiver_name)
    if lock is not None:
        return lock.kind
    obj = model.objects.get(receiver_name)
    if obj is not None:
        return obj.kind
    return None


def _context_guard_holds(context: ExclusiveContextDef) -> dict[str, str]:
    guard = context.guard
    if guard is None:
        return {}
    entries: dict[str, str] = {}
    for block in guard.holds:
        for entry, _span in block.entry_spans:
            match = _ATTR_RE.match(entry)
            if match is not None:
                entries[match.group(1)] = match.group(2).strip()
    return entries


def _normalize_hold_value(raw: str) -> bool | None:
    normalized = raw.strip()
    if normalized.startswith("State::"):
        normalized = normalized.removeprefix("State::")
    normalized = normalized.lower()
    if normalized in _FALSE_HOLD_VALUES:
        return False
    if normalized in _TRUE_HOLD_VALUES:
        return True
    return None


def _set_context_contribution_value(
    contribution: _ContextContribution,
    key: str,
    value: bool,
    *,
    context: ExclusiveContextDef,
    diagnostics: list[Diagnostic],
) -> _ContextContribution:
    existing = getattr(contribution, key)
    if existing is not None and existing is not value:
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"context guard hold conflicts with guard schema on {context.name}: {key}",
                context.guard.span if context.guard is not None else context.decl.span,
            )
        )
        return contribution
    return _replace_context_contribution(contribution, **{key: value})


def _replace_context_contribution(
    contribution: _ContextContribution, **changes: bool | frozenset[str] | None
) -> _ContextContribution:
    values = {
        "local_interrupts": contribution.local_interrupts,
        "preemption": contribution.preemption,
        "voluntary_switching": contribution.voluntary_switching,
        "cpu_concurrency": contribution.cpu_concurrency,
        "task_concurrency": contribution.task_concurrency,
        "exclusive_refs": contribution.exclusive_refs,
    }
    values.update(changes)
    return _ContextContribution(**values)


def _parse_legacy_context_effects(
    context: ExclusiveContextDef, diagnostics: list[Diagnostic]
) -> _ContextContribution | None:
    entries: dict[str, str] = {}
    for block in context.decl.effects:
        for entry, _span in block.entry_spans:
            match = _ATTR_RE.match(entry)
            if match is not None:
                entries[match.group(1)] = match.group(2).strip()
    if not entries:
        return None

    values: dict[str, bool | None] = {}
    for key in _LEGACY_CONTEXT_BOOLEAN_EFFECTS:
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
        # Legacy true means "not constrained by this context" during migration.
        values[_LEGACY_EFFECT_KEY_MAP[key]] = False if raw == "false" else None

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

    return _ContextContribution(
        local_interrupts=values["local_interrupts"],
        preemption=values["preemption"],
        voluntary_switching=values["voluntary_switching"],
        exclusive_refs=exclusive_refs,
    )


def _check_legacy_effects_compatible(
    context: ExclusiveContextDef,
    *,
    derived: _ContextContribution,
    legacy: _ContextContribution,
    diagnostics: list[Diagnostic],
) -> None:
    for key in _CONTRIBUTION_KEYS:
        derived_value = getattr(derived, key)
        legacy_value = getattr(legacy, key)
        if derived_value is not None and legacy_value is not None and derived_value is not legacy_value:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    "context legacy effects conflict with guard-derived contribution "
                    f"on {context.name}: {key}",
                    context.decl.span,
                )
            )
    if legacy.exclusive_refs != derived.exclusive_refs:
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                "context legacy effects conflict with guard-derived exclusive_refs "
                f"on {context.name}",
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
            for transition in state.transitions.values():
                bindings: dict[str, str] = {}
                _check_body_member_references(
                    model,
                    _ordered_body_members(transition.decl),
                    diagnostics,
                    bindings=bindings,
                )


def _check_only_once_withins(model: ObjectModel, diagnostics: list[Diagnostic]) -> None:
    transition_counts = _reachable_transition_call_counts(model)
    for obj in model.objects.values():
        for state in obj.states.values():
            for transition in state.transitions.values():
                transition_key = (obj.name, transition.name)
                transition_count = transition_counts.get(transition_key, 0)
                for within, local_count in _within_local_entries(transition.decl.within):
                    if not within.only_once:
                        continue
                    total_count = transition_count * local_count
                    if total_count != 1:
                        diagnostics.append(
                            Diagnostic(
                                Severity.ERROR,
                                "within only-once proof failed: "
                                f"{within.context} is reachable {total_count} times",
                                within.span,
                            )
                        )


def _reachable_transition_call_counts(model: ObjectModel) -> dict[tuple[str, str], int]:
    root = ("StartupTimeline", "Setup")
    if _transition_def(model, *root) is None:
        return {}
    counts: dict[tuple[str, str], int] = {}
    visiting: set[tuple[str, str]] = set()

    def visit(transition_key: tuple[str, str]) -> None:
        counts[transition_key] = counts.get(transition_key, 0) + 1
        if transition_key in visiting:
            return
        transition = _transition_def(model, *transition_key)
        if transition is None:
            return
        visiting.add(transition_key)
        for callee in _driven_transitions(transition.decl):
            visit(callee)
        visiting.remove(transition_key)

    visit(root)
    return counts


def _transition_def(model: ObjectModel, object_name: str, transition_name: str) -> TransitionDef | None:
    obj = model.objects.get(object_name)
    if obj is None:
        return None
    for state in obj.states.values():
        transition = state.transitions.get(transition_name)
        if transition is not None:
            return transition
    return None


def _driven_transitions(transition: TransitionDecl) -> list[tuple[str, str]]:
    return _driven_transitions_from_body_members(_ordered_body_members(transition))


def _driven_transitions_from_body_members(members) -> list[tuple[str, str]]:
    transitions: list[tuple[str, str]] = []
    for member in members:
        if member.block is not None and member.kind == "drives":
            transitions.extend(_driven_transitions_from_block(member.block))
        elif member.within is not None:
            transitions.extend(_driven_transitions_from_within(member.within))
    return transitions


def _driven_transitions_from_within(within) -> list[tuple[str, str]]:
    return _driven_transitions_from_body_members(_ordered_body_members(within))


def _driven_transitions_from_block(block: Block) -> list[tuple[str, str]]:
    transitions: list[tuple[str, str]] = []
    for entry, _span in block.entry_spans:
        match = _OBJECT_TRANSITION_EXPR_RE.match(entry)
        if match is not None:
            transitions.append((match.group(1), match.group(2)))
    return transitions


def _within_local_entries(withins) -> list[tuple[object, int]]:
    entries: list[tuple[object, int]] = []
    for within in withins:
        entries.append((within, 1))
        entries.extend(_within_local_entries(_ordered_child_withins(within)))
    return entries


def _ordered_body_members(decl):
    if decl.body_members:
        return decl.body_members
    members = []
    for block in decl.depends_on:
        members.append(_block_body_member(block))
    for block in decl.drives:
        members.append(_block_body_member(block))
    for within in decl.within:
        members.append(_within_body_member(within))
    for block in getattr(decl, "exited_by", []):
        members.append(_block_body_member(block))
    for block in decl.may_change:
        members.append(_block_body_member(block))
    for block in decl.ensures:
        members.append(_block_body_member(block))
    for block in decl.deferred:
        members.append(_block_body_member(block))
    for block in decl.other_blocks:
        members.append(_block_body_member(block))
    return members


def _ordered_child_withins(decl) -> list[object]:
    return [member.within for member in _ordered_body_members(decl) if member.within is not None]


def _block_body_member(block: Block):
    return BodyMember(kind=block.kind, span=block.span, block=block)


def _within_body_member(within):
    return BodyMember(kind="within", span=within.span, within=within)




def _check_within_references(
    model: ObjectModel,
    within,
    diagnostics: list[Diagnostic],
    *,
    inherited_bindings: dict[str, str] | None = None,
    inherited_context: _ContextContribution | None = None,
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
    contribution = _derive_context_contribution(model, context, diagnostics)
    if inherited_context is not None:
        _check_context_nesting_contribution(
            parent_context=inherited_context,
            child_contribution=contribution,
            child_context=context,
            span=within.span,
            diagnostics=diagnostics,
        )
    cumulative_context = _compose_context_contribution(
        inherited_context, contribution
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
    bindings: dict[str, str] = dict(inherited_bindings or {})
    bindings.update(_within_parameter_bindings(within.parameters, bindings))
    _check_body_member_references(
        model,
        _ordered_body_members(within),
        diagnostics,
        context=context,
        bindings=bindings,
        inherited_context=cumulative_context,
    )
    _check_lock_event_blocks(model, within.exited_by, diagnostics, context=context)


def _check_body_member_references(
    model: ObjectModel,
    members,
    diagnostics: list[Diagnostic],
    *,
    context: ExclusiveContextDef | None = None,
    bindings: dict[str, str],
    inherited_context: _ContextContribution | None = None,
) -> None:
    for member in members:
        if member.block is not None:
            if member.kind == "depends_on":
                _check_state_references(model, member.block, diagnostics)
            elif member.kind == "drives":
                _check_drive_references(
                    model,
                    member.block,
                    diagnostics,
                    context=context,
                    bindings=bindings,
                )
            continue

        child_within = member.within
        if child_within is None:
            continue
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
            inherited_context=inherited_context,
        )


def _check_context_nesting_contribution(
    *,
    parent_context: _ContextContribution,
    child_contribution: _ContextContribution,
    child_context: ExclusiveContextDef,
    span: SourceSpan,
    diagnostics: list[Diagnostic],
) -> None:
    for key in _CONTRIBUTION_KEYS:
        parent_value = getattr(parent_context, key)
        child_value = getattr(child_contribution, key)
        if parent_value is False and child_value is True:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    "invalid context nesting: inner context "
                    f"{child_context.name} weakens {key} from constrained to open",
                    span,
                )
            )


def _compose_context_contribution(
    parent: _ContextContribution | None, child: _ContextContribution
) -> _ContextContribution:
    if parent is None:
        return child
    return _ContextContribution(
        local_interrupts=_compose_constraint(parent.local_interrupts, child.local_interrupts),
        preemption=_compose_constraint(parent.preemption, child.preemption),
        voluntary_switching=_compose_constraint(
            parent.voluntary_switching, child.voluntary_switching
        ),
        cpu_concurrency=_compose_constraint(parent.cpu_concurrency, child.cpu_concurrency),
        task_concurrency=_compose_constraint(parent.task_concurrency, child.task_concurrency),
        exclusive_refs=parent.exclusive_refs | child.exclusive_refs,
    )


def _compose_constraint(parent: bool | None, child: bool | None) -> bool | None:
    if parent is False or child is False:
        return False
    if parent is True or child is True:
        return True
    return None


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
    if _is_never_guard_block(block):
        return
    for lock_name, transition_name in _LOCK_TRANSITION_RE.findall(block.body):
        if lock_name != context.lock_ref:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    "lock transition reference outside exclusive_context lock_ref: "
                    f"{lock_name}.Transition::{transition_name} not bound to {context.name}",
                    block.span,
                )
            )
            continue
        lock = model.locks.get(lock_name)
        obj = model.objects.get(lock_name)
        if lock is None and obj is None:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"unknown lock_ref in transition reference: {lock_name}.Transition::{transition_name}",
                    block.span,
                )
            )
            continue
        if lock is not None and lock.kind is None:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"lock transition reference requires typed lock: {lock_name}.Transition::{transition_name}",
                    block.span,
                )
            )
            continue
        if lock is not None:
            lock_type = model.types.get(lock.kind)
            if lock_type is None:
                continue
            if not _type_declares_event(lock_type, transition_name):
                diagnostics.append(
                    Diagnostic(
                        Severity.ERROR,
                        f"unknown lock type transition reference: {lock_name}.Transition::{transition_name}",
                        block.span,
                    )
                )
            continue
        assert obj is not None
        if not any(transition_name in state.transitions for state in obj.states.values()) and not (
            obj.kind in model.types and _type_declares_event(model.types[obj.kind], transition_name)
        ):
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"unknown lock object transition reference: {lock_name}.Transition::{transition_name}",
                    block.span,
                )
            )
    for lock_name, action_name in _LOCK_ACTION_RE.findall(block.body):
        if lock_name != context.lock_ref:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    "lock action reference outside exclusive_context lock_ref: "
                    f"{lock_name}.Action::{action_name} not bound to {context.name}",
                    block.span,
                )
            )
            continue
        lock = model.locks.get(lock_name)
        obj = model.objects.get(lock_name)
        if lock is None and obj is None:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"unknown lock_ref in action reference: {lock_name}.Action::{action_name}",
                    block.span,
                )
            )
            continue
        if lock is not None and lock.kind is None:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"lock action reference requires typed lock: {lock_name}.Action::{action_name}",
                    block.span,
                )
            )
            continue
        if lock is not None:
            lock_type = model.types.get(lock.kind)
            if lock_type is None:
                continue
            if not _type_declares_action(lock_type, action_name):
                diagnostics.append(
                    Diagnostic(
                        Severity.ERROR,
                        f"unknown lock type action reference: {lock_name}.Action::{action_name}",
                        block.span,
                    )
                )
            continue
        assert obj is not None
        if not (
            obj.kind in model.types and _type_declares_action(model.types[obj.kind], action_name)
        ):
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"unknown lock object action reference: {lock_name}.Action::{action_name}",
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
    allowed = _context_allowed_object_refs(context)
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


def _context_allowed_object_refs(context: ExclusiveContextDef | None) -> set[str] | None:
    if context is None or not context.obj_refs:
        return None
    return set(context.obj_refs)


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
            receiver_type = _binding_or_ref_value_type(object_name, bindings)
            if object_name in model.objects:
                _check_action_reference(
                    model,
                    object_name,
                    action_name,
                    diagnostics,
                    entry_span,
                    context=context,
                )
                return_type = _action_return_type(model, object_name, action_name)
            elif receiver_type is not None and _is_supported_ref_type_action(
                receiver_type, action_name
            ):
                return_type = _ref_action_return_type(model, receiver_type, action_name)
                process_type = _REF_TARGET_PROCESS_TYPES.get(receiver_type)
                if process_type is not None:
                    _check_process_arguments(
                        model,
                        process_type,
                        "Action",
                        action_name,
                        args,
                        diagnostics,
                        entry_span,
                    )
            else:
                diagnostics.append(
                    Diagnostic(
                        Severity.ERROR,
                        f"unknown action receiver: {object_name}.Action::{action_name}",
                        entry_span,
                    )
                )
                return_type = None
            if type_name != return_type:
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
        if _check_drive_transition_entry(
            model,
            entry,
            entry_span,
            diagnostics,
            context=context,
            bindings=bindings,
        ):
            continue
        if _check_drive_action_entry(
            model,
            entry,
            entry_span,
            diagnostics,
            context=context,
            bindings=bindings,
        ):
            continue
        _check_event_references(model, block, diagnostics, bindings=bindings)
        _check_action_references(model, block, diagnostics, context=context)


def _check_drive_transition_entry(
    model: ObjectModel,
    entry: str,
    span: SourceSpan,
    diagnostics: list[Diagnostic],
    *,
    context: ExclusiveContextDef | None = None,
    bindings: dict[str, str],
) -> bool:
    ref_transition = _REF_TRANSITION_EXPR_RE.match(entry)
    if ref_transition is not None:
        receiver_name, transition_name, args = ref_transition.group(1, 2, 3)
        receiver_type = bindings.get(receiver_name)
        if receiver_type is None:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"unknown ref binding in transition reference: {receiver_name}.Transition::{transition_name}",
                    span,
                )
            )
            return True
        if not _is_supported_ref_type_transition(receiver_type, transition_name):
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    "unsupported ref transition reference: "
                    f"{receiver_name}: {receiver_type}.Transition::{transition_name}",
                    span,
                )
            )
            return True
        process_type = _REF_TARGET_PROCESS_TYPES.get(receiver_type)
        if process_type is not None:
            _check_process_arguments(
                model,
                process_type,
                "Transition",
                transition_name,
                args,
                diagnostics,
                span,
            )
        return True

    match = _OBJECT_TRANSITION_EXPR_RE.match(entry)
    if match is None:
        return False
    object_name, transition_name, args = match.group(1, 2, 3)
    allowed = _context_allowed_object_refs(context)
    obj = model.objects.get(object_name)
    if obj is None:
        if _is_supported_ref_transition(object_name, transition_name):
            return True
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"unknown object in transition reference: {object_name}.Transition::{transition_name}",
                span,
            )
        )
        return True
    if allowed is not None and object_name not in allowed:
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                "transition reference outside exclusive_context obj_refs: "
                f"{object_name}.Transition::{transition_name} not in {context.name}",
                span,
            )
        )
    if not any(transition_name in state.transitions for state in obj.states.values()) and not (
        obj.kind in model.types and _type_declares_event(model.types[obj.kind], transition_name)
    ):
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"unknown transition reference: {object_name}.Transition::{transition_name}",
                span,
            )
        )
        return True
    _check_process_arguments(
        model,
        obj.kind,
        "Transition",
        transition_name,
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
    bindings: dict[str, str],
) -> bool:
    ref_action = _REF_ACTION_EXPR_RE.match(entry)
    if ref_action is not None:
        receiver_name, action_name, args = ref_action.group(1, 2, 3)
        receiver_type = _binding_or_ref_value_type(receiver_name, bindings)
        if receiver_type is None:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"unknown ref binding in action reference: {receiver_name}.Action::{action_name}",
                    span,
                )
            )
            return True
        if not _is_supported_ref_type_action(receiver_type, action_name):
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    "unsupported ref action reference: "
                    f"{receiver_name}: {receiver_type}.Action::{action_name}",
                    span,
                )
            )
            return True
        process_type = _REF_TARGET_PROCESS_TYPES.get(receiver_type)
        if process_type is not None:
            _check_process_arguments(
                model,
                process_type,
                "Action",
                action_name,
                args,
                diagnostics,
                span,
            )
        return True

    action = _OBJECT_ACTION_EXPR_RE.match(entry)
    if action is None:
        return False
    object_name, action_name, args = action.group(1, 2, 3)
    if object_name not in model.objects:
        receiver_type = _binding_or_ref_value_type(object_name, bindings)
        if receiver_type is not None and _is_supported_ref_type_action(
            receiver_type, action_name
        ):
            process_type = _REF_TARGET_PROCESS_TYPES.get(receiver_type)
            if process_type is not None:
                _check_process_arguments(
                    model,
                    process_type,
                    "Action",
                    action_name,
                    args,
                    diagnostics,
                    span,
                )
            return True
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
    allowed = _context_allowed_object_refs(context)
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
    for receiver_name, transition_name in _REF_TRANSITION_RE.findall(block.body):
        receiver_type = bindings.get(receiver_name)
        if receiver_type is None:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"unknown ref binding in transition reference: {receiver_name}.Transition::{transition_name}",
                    block.span,
                )
            )
            continue
        if not _is_supported_ref_type_transition(receiver_type, transition_name):
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    "unsupported ref transition reference: "
                    f"{receiver_name}: {receiver_type}.Transition::{transition_name}",
                    block.span,
                )
            )
        return
    for object_name, transition_name in _OBJECT_TRANSITION_RE.findall(block.body):
        obj = model.objects.get(object_name)
        if obj is None:
            if _is_supported_ref_transition(object_name, transition_name):
                continue
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"unknown object in transition reference: {object_name}.Transition::{transition_name}",
                    block.span,
                )
            )
            continue
        if not any(transition_name in state.transitions for state in obj.states.values()) and not (
            obj.kind in model.types
            and _type_declares_event(model.types[obj.kind], transition_name)
        ):
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"unknown transition reference: {object_name}.Transition::{transition_name}",
                    block.span,
                )
            )


def _type_declares_event(type_decl: TypeDecl, transition_name: str) -> bool:
    pattern = re.compile(_TYPE_TRANSITION_RE_TEMPLATE.format(re.escape(transition_name)))
    return any(pattern.search(block.body) for block in type_decl.blocks)


def _type_declares_action(type_decl: TypeDecl, action_name: str) -> bool:
    pattern = re.compile(r"\bAction::{}\b".format(re.escape(action_name)))
    return any(pattern.search(block.body) for block in type_decl.blocks)


def _is_supported_ref_transition(receiver_name: str, transition_name: str) -> bool:
    receiver_type = _known_ref_value_type(receiver_name)
    return receiver_type is not None and _is_supported_ref_type_transition(
        receiver_type, transition_name
    )


def _is_supported_ref_type_transition(type_name: str, transition_name: str) -> bool:
    return type_name == "RunQueueRef" and transition_name == "EnqueueTask"


def _is_supported_ref_type_action(type_name: str, action_name: str) -> bool:
    if type_name == "RunQueueRef":
        return action_name == "PickNextTask"
    if type_name == "TaskRef":
        return action_name in {"SaveCoreContext", "RestoreCoreContext"}
    return False


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
    raw_args = [item.strip() for item in args.split(",") if item.strip()]
    if len(signature) != len(raw_args):
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                "positional process argument count mismatch: "
                f"{type_name}.{process_kind}::{process_name}",
                span,
            )
        )


def _is_known_ref_value(value: str) -> bool:
    return _known_ref_value_type(value) is not None


def _binding_or_ref_value_type(
    value: str, bindings: dict[str, str]
) -> str | None:
    if value in bindings:
        return bindings[value]
    return _known_ref_value_type(value)


def _known_ref_value_type(value: str) -> str | None:
    if value.endswith("RunQueueRef") or value == "CurrentRunQ":
        return "RunQueueRef"
    if value.endswith("TaskRef"):
        return "TaskRef"
    return None


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
        + r"\s*(?:\(([^{};]*)\))?(?:\s*->\s*[A-Z][A-Za-z0-9_]*)?\s*\{",
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
    return _process_return_type(candidates, "Action", action_name)


def _ref_action_return_type(
    model: ObjectModel, receiver_type: str, action_name: str
) -> str | None:
    process_type = _REF_TARGET_PROCESS_TYPES.get(receiver_type)
    type_decl = model.types.get(process_type or receiver_type)
    if type_decl is None:
        return None
    return _process_return_type(
        [block.body for block in type_decl.blocks], "Action", action_name
    )


def _process_return_type(
    candidates: list[str], process_kind: str, process_name: str
) -> str | None:
    pattern = re.compile(
        r"\b"
        + re.escape(process_kind)
        + r"::"
        + re.escape(process_name)
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
    "TransitionDef",
    "ObjectDef",
    "ObjectModel",
    "Severity",
    "StateDef",
    "build_model",
    "summarize_model",
]
