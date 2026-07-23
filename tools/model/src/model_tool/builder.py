"""Static model builder for parsed LKM specs."""

from __future__ import annotations

from dataclasses import dataclass
import re
from common.defaults import DEFAULT_TARGET
from common.emits import parse_emit_expression
from common.model_types import (
    BoundaryDef,
    BuildResult,
    Diagnostic,
    DeclarationSiteDef,
    TransitionDef,
    ExclusiveContextDef,
    ObjectDef,
    ObjectModel,
    Severity,
    StateDef,
)
from common.spec_ast import (
    BoundaryDecl,
    BodyMember,
    Block,
    ExclusiveContextDecl,
    EnumDecl,
    TransitionDecl,
    FunctionDecl,
    ObjectDecl,
    PredicateDecl,
    ProcessDecl,
    SourceSpan,
    SpecDocument,
    StateDecl,
    TypeDecl,
    WithinDecl,
)


_OBJECT_TRANSITION_RE = re.compile(r"\b([A-Z][A-Za-z0-9_]*)\.Transition::([A-Za-z_][A-Za-z0-9_]*)\b")
_OBJECT_ACTION_RE = re.compile(r"\b([A-Z][A-Za-z0-9_]*)\.Action::([A-Za-z_][A-Za-z0-9_]*)\b")
_OBJECT_TRANSITION_EXPR_RE = re.compile(
    r"\A([A-Z][A-Za-z0-9_]*)\.Transition::([A-Za-z_][A-Za-z0-9_]*)(?:\s*\((.*)\))?\Z",
    re.S,
)
_LOCAL_TRANSITION_EXPR_RE = re.compile(
    r"\ATransition::([A-Za-z_][A-Za-z0-9_]*)\Z"
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
_DECLARE_RE = re.compile(
    r"\Adeclare\s+([a-z][A-Za-z0-9_]*)\s+of\s+([A-Z][A-Za-z0-9_]*)\Z"
)
_LOCAL_MEMBER_REF_RE = re.compile(
    r"(?<!\.)\b([a-z][A-Za-z0-9_]*)\.(?:state|Transition::|Action::)"
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
_BOUNDARY_ID_RE = re.compile(r"\A[a-z][a-z0-9_]*\.[0-9]{3}\Z")
_UNPARSED_LEGACY_BOUNDARY_RE = re.compile(r"(?m)^\s*(?:deferred|trimmed)\s*\{")
_BOUNDARY_CATEGORIES = {
    "deferred": frozenset(
        {
            "DeferredCategory::Feature",
            "DeferredCategory::Protocol",
            "DeferredCategory::ModelDetail",
            "DeferredCategory::Proof",
            "DeferredCategory::AlternatePath",
        }
    ),
    "trimmed": frozenset(
        {
            "TrimmedCategory::BuildConfig",
            "TrimmedCategory::Architecture",
            "TrimmedCategory::ReferenceInput",
            "TrimmedCategory::CompileTimeNoOp",
        }
    ),
}


@dataclass(frozen=True)
class _ContextContribution:
    local_interrupts: bool | None = None
    preemption: bool | None = None
    voluntary_switching: bool | None = None
    cpu_concurrency: bool | None = None
    task_concurrency: bool | None = None
    exclusive_refs: frozenset[str] = frozenset()


def build_model(
    document: SpecDocument, *, allow_legacy_boundaries: bool = False
) -> BuildResult:
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
    _check_type_lifecycles(types, diagnostics)
    objects = _build_objects(document.objects, types, diagnostics)
    children = _build_children(objects, diagnostics)
    boundaries, legacy_boundary_count = _build_boundaries(
        document.objects,
        diagnostics,
        allow_legacy_boundaries=allow_legacy_boundaries,
    )
    legacy_boundary_count += _check_unparsed_legacy_boundaries(
        document,
        diagnostics,
        allow=allow_legacy_boundaries,
    )
    declaration_sites = _collect_declaration_sites(document)

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
        boundaries=boundaries,
        declaration_sites=declaration_sites,
        legacy_boundary_count=legacy_boundary_count,
    )

    _check_initial_states(model, diagnostics)
    _check_event_targets(model, diagnostics)
    _check_lock_references(model, diagnostics)
    _check_exclusive_context_references(model, diagnostics)
    _check_references(model, diagnostics)
    _check_process_references(model, diagnostics)
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
            f"declaration_sites: {len(model.declaration_sites)}",
            f"deferred: {model.deferred_count}",
            f"trimmed: {model.trimmed_count}",
            f"legacy_boundaries: {model.legacy_boundary_count}",
            f"errors: {len(result.errors)}",
            f"warnings: {len(result.warnings)}",
        ]
    )


def _collect_declaration_sites(document: SpecDocument) -> tuple[DeclarationSiteDef, ...]:
    sites: list[DeclarationSiteDef] = []

    def collect_members(members) -> None:
        for member in members:
            if member.block is not None and member.kind == "drives":
                for statement in member.block.statements:
                    if statement.kind != "declare":
                        continue
                    sites.append(
                        DeclarationSiteDef(
                            owner_process=statement.owner_process or "<unknown>",
                            ordinal=statement.ordinal,
                            alias=statement.alias or "",
                            declared_type=statement.declared_type or "",
                            span=statement.span,
                        )
                    )
            elif member.within is not None:
                collect_members(member.within.body_members)

    for type_decl in document.types:
        for process in type_decl.processes:
            collect_members(process.body_members)
        for state in type_decl.states:
            for transition in state.transitions:
                collect_members(transition.body_members)
    for obj in document.objects:
        for process in obj.processes:
            collect_members(process.body_members)
        for state in obj.states:
            for transition in state.transitions:
                collect_members(transition.body_members)
            for process in state.processes:
                collect_members(process.body_members)
    return tuple(sites)


def _build_boundaries(
    declarations: list[ObjectDecl],
    diagnostics: list[Diagnostic],
    *,
    allow_legacy_boundaries: bool,
) -> tuple[dict[str, BoundaryDef], int]:
    boundaries: dict[str, BoundaryDef] = {}
    legacy_count = 0

    for obj in declarations:
        for state in obj.states:
            legacy_count += _check_legacy_boundary_blocks(
                state.deferred,
                diagnostics,
                allow=allow_legacy_boundaries,
                owner=f"{obj.name}.State::{state.name}",
            )
            for boundary in state.boundaries:
                _add_boundary(
                    boundaries,
                    boundary,
                    diagnostics,
                    object_name=obj.name,
                    state_name=state.name,
                )
            for transition in state.transitions:
                owner = f"{obj.name}.Transition::{transition.name}"
                legacy_count += _check_legacy_boundary_blocks(
                    transition.deferred,
                    diagnostics,
                    allow=allow_legacy_boundaries,
                    owner=owner,
                )
                for boundary in transition.boundaries:
                    _add_boundary(
                        boundaries,
                        boundary,
                        diagnostics,
                        object_name=obj.name,
                        state_name=state.name,
                        transition_name=transition.name,
                    )
                for within in transition.within:
                    legacy_count += _add_within_boundaries(
                        boundaries,
                        within,
                        diagnostics,
                        allow_legacy_boundaries=allow_legacy_boundaries,
                        object_name=obj.name,
                        state_name=state.name,
                        transition_name=transition.name,
                        context_path=(),
                    )
    return boundaries, legacy_count


def _check_unparsed_legacy_boundaries(
    document: SpecDocument,
    diagnostics: list[Diagnostic],
    *,
    allow: bool,
) -> int:
    """Reject legacy blocks nested in raw type/action/unknown blocks.

    Type processes and state-local action declarations are intentionally still
    preserved as raw blocks by the parser. Scan only those unparsed blocks so
    the final migration gate cannot silently miss a nested ``deferred { ... }``.
    """

    blocks: list[Block] = []
    for type_decl in document.types:
        blocks.extend(type_decl.blocks)
    for object_decl in document.objects:
        blocks.extend(object_decl.other_blocks)
        for state_decl in object_decl.states:
            blocks.extend(state_decl.other_blocks)
            for transition_decl in state_decl.transitions:
                blocks.extend(transition_decl.other_blocks)
                for within in transition_decl.within:
                    blocks.extend(_unparsed_within_blocks(within))

    count = 0
    for block in blocks:
        matches = list(_UNPARSED_LEGACY_BOUNDARY_RE.finditer(block.body))
        count += len(matches)
        if allow:
            continue
        for _match in matches:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    "legacy deferred/trimmed block is forbidden inside an "
                    f"unparsed {block.kind} block",
                    block.span,
                )
            )
    return count


def _unparsed_within_blocks(within: WithinDecl) -> list[Block]:
    blocks = list(within.other_blocks)
    for child in within.within:
        blocks.extend(_unparsed_within_blocks(child))
    return blocks


def _add_within_boundaries(
    boundaries: dict[str, BoundaryDef],
    within: WithinDecl,
    diagnostics: list[Diagnostic],
    *,
    allow_legacy_boundaries: bool,
    object_name: str,
    state_name: str,
    transition_name: str,
    context_path: tuple[str, ...],
) -> int:
    path = (*context_path, within.context)
    owner = (
        f"{object_name}.Transition::{transition_name} within "
        + " / ".join(path)
    )
    legacy_count = _check_legacy_boundary_blocks(
        within.deferred,
        diagnostics,
        allow=allow_legacy_boundaries,
        owner=owner,
    )
    for boundary in within.boundaries:
        _add_boundary(
            boundaries,
            boundary,
            diagnostics,
            object_name=object_name,
            state_name=state_name,
            transition_name=transition_name,
            context_path=path,
        )
    for child in within.within:
        legacy_count += _add_within_boundaries(
            boundaries,
            child,
            diagnostics,
            allow_legacy_boundaries=allow_legacy_boundaries,
            object_name=object_name,
            state_name=state_name,
            transition_name=transition_name,
            context_path=path,
        )
    return legacy_count


def _check_legacy_boundary_blocks(
    blocks: list[Block],
    diagnostics: list[Diagnostic],
    *,
    allow: bool,
    owner: str,
) -> int:
    if not allow:
        for block in blocks:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    "legacy deferred block is forbidden; migrate to a structured "
                    f"deferred/trimmed boundary on {owner}",
                    block.span,
                )
            )
    return sum(
        max(len(block.entry_spans), 1)
        for block in blocks
        if block.body.strip()
    )


def _add_boundary(
    boundaries: dict[str, BoundaryDef],
    boundary: BoundaryDecl,
    diagnostics: list[Diagnostic],
    *,
    object_name: str,
    state_name: str,
    transition_name: str | None = None,
    context_path: tuple[str, ...] = (),
) -> None:
    valid = True
    if boundary.status not in _BOUNDARY_CATEGORIES:
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"unknown boundary status: {boundary.status}",
                boundary.span,
            )
        )
        valid = False
    if not _BOUNDARY_ID_RE.fullmatch(boundary.id):
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"invalid boundary ID: {boundary.id or '<missing>'}",
                boundary.span,
            )
        )
        valid = False
    elif boundary.id in boundaries:
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"duplicate boundary ID: {boundary.id}",
                boundary.span,
            )
        )
        valid = False

    allowed_categories = _BOUNDARY_CATEGORIES.get(boundary.status, frozenset())
    if boundary.category not in allowed_categories:
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"invalid {boundary.status} category on {boundary.id or '<missing>'}: "
                f"{boundary.category or '<missing>'}",
                boundary.span,
            )
        )
        valid = False
    if boundary.summary is None or not boundary.summary.strip():
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"boundary {boundary.id or '<missing>'} is missing summary",
                boundary.span,
            )
        )
        valid = False
    if boundary.resolution is None or not boundary.resolution.strip():
        field = "close_when" if boundary.status == "deferred" else "revisit_when"
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"boundary {boundary.id or '<missing>'} is missing {field}",
                boundary.span,
            )
        )
        valid = False
    if len(boundary.evidence) != 1 or not boundary.evidence[0].entry_spans:
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"boundary {boundary.id or '<missing>'} must contain one non-empty evidence block",
                boundary.span,
            )
        )
        valid = False
    if boundary.other_blocks:
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"boundary {boundary.id or '<missing>'} contains unsupported block: "
                f"{boundary.other_blocks[0].kind}",
                boundary.other_blocks[0].span,
            )
        )
        valid = False
    if boundary.unknown_properties:
        key = sorted(boundary.unknown_properties)[0]
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"boundary {boundary.id or '<missing>'} contains unsupported field: {key}",
                boundary.span,
            )
        )
        valid = False
    for key, count in boundary.property_counts.items():
        if count <= 1:
            continue
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"boundary {boundary.id or '<missing>'} repeats field: {key}",
                boundary.span,
            )
        )
        valid = False
    if not valid:
        return

    category = (boundary.category or "").split("::", 1)[-1]
    boundaries[boundary.id] = BoundaryDef(
        id=boundary.id,
        status=boundary.status,
        category=category,
        summary=boundary.summary or "",
        resolution=boundary.resolution or "",
        decl=boundary,
        object_name=object_name,
        state_name=state_name,
        transition_name=transition_name,
        context_path=context_path,
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
    declarations: list[ObjectDecl],
    types: dict[str, TypeDecl],
    diagnostics: list[Diagnostic],
) -> dict[str, ObjectDef]:
    objects: dict[str, ObjectDef] = {}
    has_kernel = any(decl.name == "Kernel" for decl in declarations)
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

        lifecycle_type = _nearest_type_lifecycle(types, decl.kind)
        object_declares_lifecycle = decl.initial_state is not None or bool(decl.states)
        override_value = decl.properties.get("lifecycle_override")
        if override_value is not None and override_value != "true":
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"lifecycle_override on {decl.name} must be true when present",
                    decl.span,
                )
            )
        if lifecycle_type is not None and object_declares_lifecycle and override_value != "true":
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    "object lifecycle must not shadow inherited type lifecycle without "
                    f"lifecycle_override: true: {decl.name} inherits {lifecycle_type.name}",
                    decl.span,
                )
            )
        if lifecycle_type is not None and not object_declares_lifecycle and override_value == "true":
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"lifecycle_override on {decl.name} requires a complete object lifecycle",
                    decl.span,
                )
            )
        if (
            lifecycle_type is not None
            and override_value == "true"
            and object_declares_lifecycle
            and (decl.initial_state is None or not decl.states)
        ):
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"lifecycle_override on {decl.name} requires both initial_state and states",
                    decl.span,
                )
            )

        if lifecycle_type is not None and not object_declares_lifecycle:
            initial_state = lifecycle_type.initial_state
            state_decls = lifecycle_type.states
            lifecycle_owner = lifecycle_type.name
        else:
            initial_state = decl.initial_state
            state_decls = decl.states
            lifecycle_owner = None

        _check_lifecycle_names(
            decl.name, initial_state, state_decls, diagnostics, decl.span
        )
        _check_event_uniqueness(decl.name, state_decls, diagnostics)
        states = _build_states(
            decl.name,
            state_decls,
            diagnostics,
            lifecycle_owner=lifecycle_owner,
        )
        attrs = _extract_attrs(decl, diagnostics)
        associations = _extract_associations(decl, diagnostics)
        effective_parent = decl.parent
        if (
            effective_parent is None
            and has_kernel
            and decl.name not in {"Computer", "Kernel"}
            and not _type_is_or_extends(types, decl.kind, "ProjectObject")
        ):
            effective_parent = "Kernel"
        objects[decl.name] = ObjectDef(
            name=decl.name,
            kind=decl.kind,
            decl=decl,
            initial_state=initial_state,
            parent=effective_parent,
            states=states,
            attrs=attrs,
            associations=associations,
        )
    return objects


def _check_type_lifecycles(
    types: dict[str, TypeDecl], diagnostics: list[Diagnostic]
) -> None:
    for decl in types.values():
        declares_lifecycle = decl.initial_state is not None or bool(decl.states)
        if not declares_lifecycle:
            continue
        if decl.initial_state is None or not decl.states:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"type lifecycle must declare both initial_state and states: {decl.name}",
                    decl.span,
                )
            )
        _check_lifecycle_names(
            decl.name, decl.initial_state, decl.states, diagnostics, decl.span
        )
        _check_event_uniqueness(decl.name, decl.states, diagnostics)
        states = _build_states(decl.name, decl.states, diagnostics)
        if decl.initial_state is not None and decl.initial_state not in states:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"unknown initial_state on type {decl.name}: State::{decl.initial_state}",
                    decl.span,
                )
            )
        for state in states.values():
            for transition in state.transitions.values():
                if transition.target_state not in states:
                    diagnostics.append(
                        Diagnostic(
                            Severity.ERROR,
                            "unknown transition target state: "
                            f"{decl.name}.Transition::{transition.name} -> "
                            f"State::{transition.target_state}",
                            transition.decl.span,
                        )
                    )


def _nearest_type_lifecycle(
    types: dict[str, TypeDecl], type_name: str
) -> TypeDecl | None:
    visited: set[str] = set()
    current: str | None = type_name
    while current is not None and current not in visited:
        visited.add(current)
        decl = types.get(current)
        if decl is None:
            return None
        if decl.initial_state is not None or decl.states:
            return decl
        current = _base_type_name(decl)
    return None


def _check_lifecycle_names(
    owner_name: str,
    initial_state: str | None,
    state_decls: list[StateDecl],
    diagnostics: list[Diagnostic],
    owner_span: SourceSpan,
) -> None:
    """Enforce SEM-NAME-001: lifecycle names come from controlled vocabularies."""

    if initial_state is not None and initial_state not in _ALLOWED_STATE_NAMES:
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"unknown lifecycle state name: {owner_name}.initial_state State::{initial_state}",
                owner_span,
            )
        )

    for state_decl in state_decls:
        if state_decl.name not in _ALLOWED_STATE_NAMES:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"unknown lifecycle state name: {owner_name}.State::{state_decl.name}",
                    state_decl.span,
                )
            )
        for transition_decl in state_decl.transitions:
            if transition_decl.name not in _ALLOWED_TRANSITION_NAMES:
                diagnostics.append(
                    Diagnostic(
                        Severity.ERROR,
                        f"unknown lifecycle transition name: {owner_name}.Transition::{transition_decl.name}",
                        transition_decl.span,
                    )
                )
            transition = (state_decl.name, transition_decl.name, transition_decl.target_state)
            if transition not in _ALLOWED_TRANSITIONS:
                diagnostics.append(
                    Diagnostic(
                        Severity.ERROR,
                        "invalid lifecycle transition: "
                        f"{owner_name}.State::{state_decl.name}.Transition::{transition_decl.name} -> State::{transition_decl.target_state}",
                        transition_decl.span,
                    )
                )
            if transition_decl.target_state not in _ALLOWED_STATE_NAMES:
                diagnostics.append(
                    Diagnostic(
                        Severity.ERROR,
                        "unknown lifecycle state name: "
                        f"{owner_name}.Transition::{transition_decl.name} -> State::{transition_decl.target_state}",
                        transition_decl.span,
                    )
                )


def _check_event_uniqueness(
    owner_name: str,
    state_decls: list[StateDecl],
    diagnostics: list[Diagnostic],
) -> None:
    """Enforce SEM-TRANSITION-001: transition identity is object-local."""

    seen: dict[str, TransitionDecl] = {}
    for state_decl in state_decls:
        for transition_decl in state_decl.transitions:
            existing = seen.get(transition_decl.name)
            if existing is None:
                seen[transition_decl.name] = transition_decl
                continue
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    "duplicate object transition declaration: "
                    f"{owner_name}.Transition::{transition_decl.name}",
                    transition_decl.span,
                )
            )


def _build_states(
    owner_name: str,
    state_decls: list[StateDecl],
    diagnostics: list[Diagnostic],
    *,
    lifecycle_owner: str | None = None,
) -> dict[str, StateDef]:
    states: dict[str, StateDef] = {}
    for state_decl in state_decls:
        if state_decl.name in states:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"duplicate state declaration: {owner_name}.State::{state_decl.name}",
                    state_decl.span,
                )
            )
            continue

        transitions = _build_events(
            owner_name,
            state_decl,
            diagnostics,
            object_wide=True,
            lifecycle_owner=lifecycle_owner,
        )
        states[state_decl.name] = StateDef(
            name=state_decl.name,
            object_name=owner_name,
            decl=state_decl,
            transitions=transitions,
            lifecycle_owner=lifecycle_owner,
        )
    return states


def _build_events(
    object_name: str,
    state_decl: StateDecl,
    diagnostics: list[Diagnostic],
    *,
    object_wide: bool = False,
    lifecycle_owner: str | None = None,
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
            lifecycle_owner=lifecycle_owner,
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


def _extract_associations(
    decl: ObjectDecl, diagnostics: list[Diagnostic]
) -> dict[str, str]:
    associations: dict[str, str] = {}
    pattern = re.compile(
        r"\A(?:mutable\s+)?([a-z][A-Za-z0-9_]*)\s*=\s*(\S+)\Z"
    )
    for block in decl.other_blocks:
        if block.kind != "associations":
            continue
        for entry, span in block.entry_spans:
            match = pattern.match(entry)
            if match is None:
                diagnostics.append(
                    Diagnostic(
                        Severity.ERROR,
                        f"cannot parse association binding on {decl.name}: {entry}",
                        span,
                    )
                )
                continue
            name, value = match.groups()
            if name in associations:
                diagnostics.append(
                    Diagnostic(
                        Severity.ERROR,
                        f"duplicate association binding: {decl.name}.{name}",
                        span,
                    )
                )
                continue
            associations[name] = value
    return associations


def _build_children(
    objects: dict[str, ObjectDef], diagnostics: list[Diagnostic]
) -> dict[str, list[str]]:
    children: dict[str, list[str]] = {name: [] for name in objects}
    for obj in objects.values():
        if obj.parent is None:
            continue
        if obj.parent == obj.name:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"self parent object for {obj.name}: {obj.parent}",
                    obj.decl.span,
                )
            )
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

    reported_cycles: set[tuple[str, ...]] = set()
    for obj in objects.values():
        path: list[str] = []
        positions: dict[str, int] = {}
        current: str | None = obj.name
        while current is not None and current in objects:
            if current in positions:
                cycle = path[positions[current] :]
                canonical = _canonical_cycle(cycle)
                if canonical not in reported_cycles:
                    reported_cycles.add(canonical)
                    diagnostics.append(
                        Diagnostic(
                            Severity.ERROR,
                            "parent cycle contains objects: " + " -> ".join([*canonical, canonical[0]]),
                            objects[current].decl.span,
                        )
                    )
                break
            positions[current] = len(path)
            path.append(current)
            current = objects[current].parent
    return children


def _canonical_cycle(cycle: list[str]) -> tuple[str, ...]:
    if not cycle:
        return ()
    variants = [tuple(cycle[index:] + cycle[:index]) for index in range(len(cycle))]
    return min(variants)


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
        for process in obj.decl.processes:
            if not _process_contains_declare(process):
                _check_process_lexical_captures(process, diagnostics)
                continue
            _check_one_process(model, process, diagnostics, receiver_type=obj.kind)
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
    _check_context_guard_owner_pairing(context, diagnostics)


def _check_context_guard_owner_pairing(
    context: ExclusiveContextDef,
    diagnostics: list[Diagnostic],
) -> None:
    guard = context.guard
    if context.kind != "ResourceExclusiveContext" or guard is None or guard.lock_ref is None:
        return

    entered_owners = _guard_boundary_owner_args(guard.entered_by, guard.lock_ref)
    exited_owners = _guard_boundary_owner_args(guard.exited_by, guard.lock_ref)
    if not entered_owners and not exited_owners:
        return
    if not entered_owners or not exited_owners:
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                "context guard owner arguments must appear on both entered_by and exited_by: "
                f"{context.name}",
                guard.span,
            )
        )
        return
    if len(entered_owners) > 1:
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                "context guard entered_by must not mix owner arguments: "
                f"{context.name}",
                guard.span,
            )
        )
    if len(exited_owners) > 1:
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                "context guard exited_by must not mix owner arguments: "
                f"{context.name}",
                guard.span,
            )
        )
    if len(entered_owners) == 1 and len(exited_owners) == 1 and entered_owners != exited_owners:
        entered = _format_owner_args(next(iter(entered_owners)))
        exited = _format_owner_args(next(iter(exited_owners)))
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                "context guard entered_by/exited_by owner arguments must match: "
                f"{context.name} entered_by({entered}) exited_by({exited})",
                guard.span,
            )
        )


def _guard_boundary_owner_args(blocks: list[Block], lock_ref: str) -> set[tuple[str, ...]]:
    owners: set[tuple[str, ...]] = set()
    for block in blocks:
        if _is_never_guard_block(block):
            continue
        for entry, _span in block.entry_spans:
            parsed = _parse_guard_boundary_expr(entry)
            if parsed is None:
                continue
            receiver, args = parsed
            if receiver != lock_ref or args is None:
                continue
            owner_args = _split_top_level_args(args)
            if owner_args:
                owners.add(owner_args)
    return owners


def _parse_guard_boundary_expr(entry: str) -> tuple[str, str | None] | None:
    match = _OBJECT_TRANSITION_EXPR_RE.match(entry)
    if match is not None:
        return match.group(1), match.group(3)
    match = _OBJECT_ACTION_EXPR_RE.match(entry)
    if match is not None:
        return match.group(1), match.group(3)
    return None


def _split_top_level_args(args: str) -> tuple[str, ...]:
    entries: list[str] = []
    start = 0
    depth = 0
    for index, char in enumerate(args):
        if char in "([{":
            depth += 1
        elif char in ")]}":
            depth = max(depth - 1, 0)
        elif char == "," and depth == 0:
            arg = _normalize_guard_arg(args[start:index])
            if arg:
                entries.append(arg)
            start = index + 1
    arg = _normalize_guard_arg(args[start:])
    if arg:
        entries.append(arg)
    return tuple(entries)


def _normalize_guard_arg(arg: str) -> str:
    return re.sub(r"\s+", " ", arg.strip())


def _format_owner_args(args: tuple[str, ...]) -> str:
    return ", ".join(args)


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
                bindings: dict[str, str] = {
                    "self": obj.kind,
                    **dict(transition.decl.parameters),
                }
                _check_body_member_references(
                    model,
                    _ordered_body_members(transition.decl),
                    diagnostics,
                    bindings=bindings,
                )
                _check_emit_references(model, transition, diagnostics)


def _check_process_references(
    model: ObjectModel, diagnostics: list[Diagnostic]
) -> None:
    for type_decl in model.types.values():
        for state in type_decl.states:
            for transition in state.transitions:
                _check_process_emit_references(
                    model,
                    transition,
                    diagnostics,
                    receiver_type=type_decl.name,
                )
                _check_one_process(
                    model,
                    transition,
                    diagnostics,
                    receiver_type=type_decl.name,
                )
        for process in type_decl.processes:
            _check_process_emit_references(
                model,
                process,
                diagnostics,
                receiver_type=type_decl.name,
            )
            if not _process_contains_declare(process):
                _check_process_lexical_captures(process, diagnostics)
                continue
            _check_one_process(model, process, diagnostics, receiver_type=type_decl.name)
    for obj in model.objects.values():
        for state in obj.states.values():
            for process in state.decl.processes:
                _check_process_emit_references(
                    model,
                    process,
                    diagnostics,
                    receiver_type=obj.kind,
                )
                if not _process_contains_declare(process):
                    _check_process_lexical_captures(process, diagnostics)
                    continue
                _check_one_process(model, process, diagnostics, receiver_type=obj.kind)


def _process_contains_declare(process: ProcessDecl) -> bool:
    def members_contain(members) -> bool:
        for member in members:
            if member.block is not None and any(
                statement.kind == "declare" for statement in member.block.statements
            ):
                return True
            if member.within is not None and members_contain(member.within.body_members):
                return True
        return False

    return members_contain(process.body_members)


def _check_process_lexical_captures(
    process: ProcessDecl, diagnostics: list[Diagnostic]
) -> None:
    """Reject process-local receivers that were not passed as parameters."""

    bindings = {"self", *(name for name, _type_name in process.parameters)}

    def check_members(members, visible: set[str]) -> None:
        for member in members:
            if member.block is not None:
                for entry, span in member.block.entry_spans:
                    for name in _LOCAL_MEMBER_REF_RE.findall(entry):
                        if name not in visible:
                            diagnostics.append(
                                Diagnostic(
                                    Severity.ERROR,
                                    "use before declaration or unknown lexical alias: "
                                    f"{name}",
                                    span,
                                )
                            )
                    bind = _ACTION_BIND_RE.match(entry)
                    if bind is not None:
                        visible.add(bind.group(1))
                continue
            child = member.within
            if child is None:
                continue
            child_bindings = set(visible)
            child_bindings.update(child.parameters)
            check_members(child.body_members, child_bindings)

    check_members(process.body_members, bindings)


def _check_one_process(
    model: ObjectModel,
    process: ProcessDecl,
    diagnostics: list[Diagnostic],
    *,
    receiver_type: str,
) -> None:
    bindings: dict[str, str] = {"self": receiver_type}
    for name, type_name in process.parameters:
        if name in bindings:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"duplicate process parameter: {name}",
                    process.span,
                )
            )
            continue
        if name in model.objects:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"process parameter conflicts with static object: {name}",
                    process.span,
                )
            )
        bindings[name] = type_name
    _check_body_member_references(
        model,
        process.body_members,
        diagnostics,
        bindings=bindings,
    )


def _check_process_emit_references(
    model: ObjectModel,
    process: ProcessDecl | TransitionDecl,
    diagnostics: list[Diagnostic],
    *,
    receiver_type: str,
) -> None:
    bindings = {"self": receiver_type, **dict(process.parameters)}
    blocks = [
        member.block
        for member in _ordered_body_members(process)
        if member.kind == "emits" and member.block is not None
    ]
    _check_emit_blocks(
        model,
        blocks,
        diagnostics,
        default_object=None,
        default_type=receiver_type,
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
    root = _default_transition_target()
    if root is None:
        return {}
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
        for callee in _transition_call_edges(transition):
            visit(callee)
        visiting.remove(transition_key)

    visit(root)
    return counts


def _default_transition_target() -> tuple[str, str] | None:
    match = _OBJECT_TRANSITION_EXPR_RE.match(DEFAULT_TARGET)
    if match is None:
        return None
    return match.group(1), match.group(2)


def _transition_def(model: ObjectModel, object_name: str, transition_name: str) -> TransitionDef | None:
    obj = model.objects.get(object_name)
    if obj is None:
        return None
    for state in obj.states.values():
        transition = state.transitions.get(transition_name)
        if transition is not None:
            return transition
    return None


def _transition_call_edges(transition: TransitionDef) -> list[tuple[str, str]]:
    return [
        *_driven_transitions_from_body_members(_ordered_body_members(transition.decl)),
        *_emitted_transitions(transition),
    ]


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


def _emitted_transitions(transition: TransitionDef) -> list[tuple[str, str]]:
    transitions: list[tuple[str, str]] = []
    for block in transition.decl.emits:
        for entry, _span in block.entry_spans:
            match = _LOCAL_TRANSITION_EXPR_RE.match(entry)
            if match is not None:
                transitions.append((transition.object_name, match.group(1)))
                continue
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
    for block in getattr(decl, "emits", []):
        members.append(_block_body_member(block))
    for within in decl.within:
        members.append(_within_body_member(within))
    for block in getattr(decl, "exited_by", []):
        members.append(_block_body_member(block))
    for block in decl.may_change:
        members.append(_block_body_member(block))
    for block in decl.ensures:
        members.append(_block_body_member(block))
    for block in getattr(decl, "deferred", []):
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
    parameter_bindings = _within_parameter_bindings(within.parameters, bindings)
    for name, type_name in parameter_bindings.items():
        if name in bindings:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"duplicate or shadowed lexical alias: {name}",
                    within.span,
                )
            )
            continue
        if name in model.objects:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"within parameter conflicts with static object: {name}",
                    within.span,
                )
            )
            continue
        bindings[name] = type_name
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
                _check_local_alias_references(member.block, bindings, diagnostics)
            elif member.kind == "drives":
                _check_drive_references(
                    model,
                    member.block,
                    diagnostics,
                    context=context,
                    bindings=bindings,
                )
            else:
                _check_local_alias_references(member.block, bindings, diagnostics)
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


def _check_local_alias_references(
    block: Block,
    bindings: dict[str, str],
    diagnostics: list[Diagnostic],
) -> None:
    for entry, span in block.entry_spans:
        for name in _LOCAL_MEMBER_REF_RE.findall(entry):
            if name == "self" or name in bindings:
                continue
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"use before declaration or unknown lexical alias: {name}",
                    span,
                )
            )


def _check_emit_references(
    model: ObjectModel,
    transition: TransitionDef,
    diagnostics: list[Diagnostic],
) -> None:
    _check_emit_blocks(
        model,
        transition.decl.emits,
        diagnostics,
        default_object=transition.object_name,
        default_type=model.objects[transition.object_name].kind,
        bindings={
            "self": model.objects[transition.object_name].kind,
            **dict(transition.decl.parameters),
        },
        source_transition=transition,
    )


def _check_emit_blocks(
    model: ObjectModel,
    blocks: list[Block],
    diagnostics: list[Diagnostic],
    *,
    default_object: str | None,
    default_type: str,
    bindings: dict[str, str],
    source_transition: TransitionDef | None = None,
) -> None:
    for block in blocks:
        for entry, entry_span in block.entry_spans:
            emitted = parse_emit_expression(entry)
            if emitted is None:
                diagnostics.append(
                    Diagnostic(
                        Severity.ERROR,
                        "emits must reference a transition through a local, "
                        f"static, or association-path receiver: {entry}",
                        entry_span,
                    )
                )
                continue

            emitted_object: str | None = None
            if emitted.receiver is None:
                emitted_object = default_object
                emitted_type = default_type
            elif emitted.receiver in model.objects:
                emitted_object = emitted.receiver
                emitted_type = model.objects[emitted.receiver].kind
            else:
                emitted_type = _resolve_association_receiver_type(
                    model, emitted.receiver, bindings
                )

            target = (
                _transition_def(model, emitted_object, emitted.transition)
                if emitted_object is not None
                else None
            )
            if target is None and (
                emitted_type is None
                or _type_process_decl(
                    model, emitted_type, "Transition", emitted.transition
                )
                is None
            ):
                receiver_label = emitted.receiver or default_object or default_type
                diagnostics.append(
                    Diagnostic(
                        Severity.ERROR,
                        "unknown emitted transition: "
                        f"{receiver_label}.Transition::{emitted.transition}",
                        entry_span,
                    )
                )
                continue

            if (
                source_transition is not None
                and target is not None
                and emitted_object == source_transition.object_name
                and target.source_state != source_transition.target_state
            ):
                diagnostics.append(
                    Diagnostic(
                        Severity.ERROR,
                        "emitted transition is not enabled from target state: "
                        f"{emitted_object}.Transition::{emitted.transition} "
                        f"requires State::{target.source_state}, "
                        f"but {source_transition.object_name}.Transition::"
                        f"{source_transition.name} targets "
                        f"State::{source_transition.target_state}",
                        entry_span,
                    )
                )


def _resolve_association_receiver_type(
    model: ObjectModel,
    receiver: str,
    bindings: dict[str, str],
) -> str | None:
    parts = receiver.split(".")
    if not parts or any(not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", part) for part in parts):
        return None
    root = parts.pop(0)
    if root in bindings:
        current_type = bindings[root]
    elif root in model.objects:
        current_type = model.objects[root].kind
    else:
        return None

    for member in parts:
        current_type = _REF_TARGET_PROCESS_TYPES.get(current_type, current_type)
        current_type = _association_type(model, current_type, member)
        if current_type is None:
            return None
    return _REF_TARGET_PROCESS_TYPES.get(current_type, current_type)


def _association_type(
    model: ObjectModel, type_name: str, association_name: str
) -> str | None:
    visited: set[str] = set()
    current = type_name
    pattern = re.compile(
        r"\A(?:mutable\s+)?"
        + re.escape(association_name)
        + r"\s*:\s*([A-Z][A-Za-z0-9_]*)\Z"
    )
    while current and current not in visited:
        visited.add(current)
        type_decl = model.types.get(current)
        if type_decl is None:
            return None
        for block in type_decl.blocks:
            if block.kind != "associations":
                continue
            for entry in block.entries:
                match = pattern.match(entry)
                if match is not None:
                    return match.group(1)
        current = _base_type_name(type_decl)
    return None


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
        declare = _DECLARE_RE.match(entry)
        if declare is not None:
            alias, type_name = declare.group(1, 2)
            if type_name not in model.types:
                diagnostics.append(
                    Diagnostic(
                        Severity.ERROR,
                        f"unknown declared type: {type_name}",
                        entry_span,
                    )
                )
            if alias in bindings:
                diagnostics.append(
                    Diagnostic(
                        Severity.ERROR,
                        f"duplicate or shadowed lexical alias: {alias}",
                        entry_span,
                    )
                )
            elif alias in model.objects:
                diagnostics.append(
                    Diagnostic(
                        Severity.ERROR,
                        f"declaration alias conflicts with static object: {alias}",
                        entry_span,
                    )
                )
            else:
                bindings[alias] = type_name
            continue
        bind = _ACTION_BIND_RE.match(entry)
        if bind is not None:
            name, type_name, object_name, action_name, args = bind.group(1, 2, 3, 4, 5)
            if name in bindings or name in model.objects:
                diagnostics.append(
                    Diagnostic(
                        Severity.ERROR,
                        f"duplicate or shadowed lexical alias: {name}",
                        entry_span,
                    )
                )
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
            elif receiver_type is not None and _type_process_decl(
                model, receiver_type, "Action", action_name
            ) is not None:
                return_type = _type_process_return_type(
                    model, receiver_type, "Action", action_name
                )
                _check_process_arguments(
                    model,
                    receiver_type,
                    "Action",
                    action_name,
                    args,
                    diagnostics,
                    entry_span,
                    bindings=bindings,
                )
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
                object_process = _object_process_decl(obj, "Action", action_name)
                if object_process is not None and _args_reference_bindings(args, bindings):
                    _check_process_decl_arguments(
                        model,
                        object_process,
                        args,
                        diagnostics,
                        entry_span,
                        bindings=bindings,
                    )
                elif object_process is None:
                    _check_process_arguments(
                        model,
                        obj.kind,
                        "Action",
                        action_name,
                        args,
                        diagnostics,
                        entry_span,
                        bindings=bindings,
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
        if _type_process_decl(
            model, receiver_type, "Transition", transition_name
        ) is not None:
            _check_process_arguments(
                model,
                receiver_type,
                "Transition",
                transition_name,
                args,
                diagnostics,
                span,
                bindings=bindings,
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
    object_transition = any(
        transition_name in state.transitions for state in obj.states.values()
    )
    if not object_transition and _type_process_decl(
        model, obj.kind, "Transition", transition_name
    ) is None:
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"unknown transition reference: {object_name}.Transition::{transition_name}",
                span,
            )
        )
        return True
    if object_transition:
        transition = _transition_def(model, object_name, transition_name)
        if transition is not None:
            _check_signature_arguments(
                model,
                transition.decl.parameters,
                f"{object_name}.Transition::{transition_name}",
                args,
                diagnostics,
                span,
                bindings=bindings,
            )
    else:
        _check_process_arguments(
            model,
            obj.kind,
            "Transition",
            transition_name,
            args,
            diagnostics,
            span,
            bindings=bindings,
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
        if _type_process_decl(model, receiver_type, "Action", action_name) is not None:
            _check_process_arguments(
                model,
                receiver_type,
                "Action",
                action_name,
                args,
                diagnostics,
                span,
                bindings=bindings,
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
        object_process = _object_process_decl(obj, "Action", action_name)
        if object_process is not None and _args_reference_bindings(args, bindings):
            _check_process_decl_arguments(
                model, object_process, args, diagnostics, span, bindings=bindings
            )
        elif object_process is None:
            _check_process_arguments(
                model,
                obj.kind,
                "Action",
                action_name,
                args,
                diagnostics,
                span,
                bindings=bindings,
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
        if _type_process_decl(
            model, receiver_type, "Transition", transition_name
        ) is not None:
            return
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
            if object_name in model.types and _type_process_decl(
                model, object_name, "Transition", transition_name
            ) is not None:
                continue
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
    return any(
        process.kind == "Transition" and process.name == transition_name
        for process in type_decl.processes
    ) or any(
        transition.name == transition_name
        for state in type_decl.states
        for transition in state.transitions
    )


def _type_declares_action(type_decl: TypeDecl, action_name: str) -> bool:
    return any(
        process.kind == "Action" and process.name == action_name
        for process in type_decl.processes
    )


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
    *,
    bindings: dict[str, str] | None = None,
) -> None:
    signature = _process_signature(model, type_name, process_kind, process_name)
    if signature is None:
        return
    _check_signature_arguments(
        model,
        signature,
        f"{type_name}.{process_kind}::{process_name}",
        args,
        diagnostics,
        span,
        bindings=bindings,
    )


def _check_process_decl_arguments(
    model: ObjectModel,
    process: ProcessDecl,
    args: str | None,
    diagnostics: list[Diagnostic],
    span: SourceSpan,
    *,
    bindings: dict[str, str] | None = None,
) -> None:
    _check_signature_arguments(
        model,
        process.parameters,
        f"{process.kind}::{process.name}",
        args,
        diagnostics,
        span,
        bindings=bindings,
    )


def _check_signature_arguments(
    model: ObjectModel,
    signature: tuple[tuple[str, str], ...],
    label: str,
    args: str | None,
    diagnostics: list[Diagnostic],
    span: SourceSpan,
    *,
    bindings: dict[str, str] | None,
) -> None:
    if args is None:
        if signature:
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"missing process argument: {label}",
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
                    f"missing process argument: {label}",
                    span,
                )
            )
        return
    if _uses_named_args(args):
        if not _args_reference_bindings(args, bindings or {}):
            return
        provided: dict[str, str] = {}
        for item in _split_process_args(args):
            name, sep, value = item.partition(":")
            if not sep:
                diagnostics.append(
                    Diagnostic(
                        Severity.ERROR,
                        f"mixed or malformed named process argument: {item.strip()}",
                        span,
                    )
                )
                continue
            provided[name.strip()] = value.strip()
        expected = dict(signature)
        if set(provided) != set(expected):
            diagnostics.append(
                Diagnostic(
                    Severity.ERROR,
                    f"named process arguments mismatch: {label}",
                    span,
                )
            )
            return
        for name, value in provided.items():
            actual = _expression_type(model, value, bindings or {})
            if value in (bindings or {}) and actual is not None and not _type_assignable(model, actual, expected[name]):
                diagnostics.append(
                    Diagnostic(
                        Severity.ERROR,
                        "process argument type mismatch: "
                        f"{name}: expected {expected[name]}, got {actual}",
                        span,
                    )
                )
        return
    raw_args = [item.strip() for item in args.split(",") if item.strip()]
    if len(signature) != len(raw_args):
        diagnostics.append(
            Diagnostic(
                Severity.ERROR,
                f"positional process argument count mismatch: {label}",
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
    process = _type_process_decl(model, type_name, process_kind, process_name)
    return process.parameters if process is not None else None


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


def _type_process_decl(
    model: ObjectModel,
    type_name: str,
    process_kind: str,
    process_name: str,
) -> ProcessDecl | TransitionDecl | None:
    visited: set[str] = set()
    current = type_name
    while current and current not in visited:
        visited.add(current)
        type_decl = model.types.get(current)
        if type_decl is None:
            return None
        if process_kind == "Transition":
            for state in type_decl.states:
                for transition in state.transitions:
                    if transition.name == process_name:
                        return transition
        for process in type_decl.processes:
            if process.kind == process_kind and process.name == process_name:
                return process
        current = _base_type_name(type_decl)
    return None


def _object_process_decl(
    obj: ObjectDef, process_kind: str, process_name: str
) -> ProcessDecl | None:
    for process in obj.decl.processes:
        if process.kind == process_kind and process.name == process_name:
            return process
    for state in obj.states.values():
        for process in state.decl.processes:
            if process.kind == process_kind and process.name == process_name:
                return process
    return None


def _type_process_return_type(
    model: ObjectModel,
    type_name: str,
    process_kind: str,
    process_name: str,
) -> str | None:
    process = _type_process_decl(model, type_name, process_kind, process_name)
    return getattr(process, "return_type", None) if process is not None else None


def _base_type_name(type_decl: TypeDecl) -> str | None:
    match = re.search(r":\s*([A-Z][A-Za-z0-9_]*)", type_decl.header)
    return match.group(1) if match is not None else None


def _type_is_or_extends(
    types: dict[str, TypeDecl], actual: str, expected: str
) -> bool:
    visited: set[str] = set()
    current: str | None = actual
    while current is not None and current not in visited:
        if current == expected:
            return True
        visited.add(current)
        declaration = types.get(current)
        current = None if declaration is None else _base_type_name(declaration)
    return False


def _split_process_args(args: str) -> list[str]:
    entries: list[str] = []
    start = 0
    depth = 0
    for index, char in enumerate(args):
        if char in "([{<":
            depth += 1
        elif char in ")]}>" :
            depth = max(0, depth - 1)
        elif char == "," and depth == 0:
            entries.append(args[start:index].strip())
            start = index + 1
    tail = args[start:].strip()
    if tail:
        entries.append(tail)
    return entries


def _args_reference_bindings(
    args: str | None, bindings: dict[str, str]
) -> bool:
    if not args:
        return False
    return any(
        re.search(r"\b" + re.escape(name) + r"\b", args) is not None
        for name in bindings
        if name != "self"
    )


def _expression_type(
    model: ObjectModel, expression: str, bindings: dict[str, str]
) -> str | None:
    value = expression.strip()
    if value in bindings:
        return bindings[value]
    obj = model.objects.get(value)
    if obj is not None:
        return obj.kind
    variant = re.match(r"([A-Z][A-Za-z0-9_]*)::", value)
    if variant is not None and variant.group(1) in model.enums:
        return variant.group(1)
    return _known_ref_value_type(value)


def _type_assignable(model: ObjectModel, actual: str, expected: str) -> bool:
    if actual == expected:
        return True
    visited: set[str] = set()
    current = actual
    while current and current not in visited:
        visited.add(current)
        type_decl = model.types.get(current)
        if type_decl is None:
            return False
        current = _base_type_name(type_decl)
        if current == expected:
            return True
    return False


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
    object_process = _object_process_decl(obj, "Action", action_name)
    if object_process is not None:
        return object_process.return_type
    return _type_process_return_type(model, obj.kind, "Action", action_name)


def _ref_action_return_type(
    model: ObjectModel, receiver_type: str, action_name: str
) -> str | None:
    process_type = _REF_TARGET_PROCESS_TYPES.get(receiver_type)
    return _type_process_return_type(
        model, process_type or receiver_type, "Action", action_name
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
