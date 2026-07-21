"""Model JSON deserialization shared by stage tools."""

from __future__ import annotations

from typing import Any

from common.schemas import MODEL_SCHEMA, MODEL_VERSION
from common.model_types import (
    BoundaryDef,
    DeclarationSiteDef,
    TransitionDef,
    ExclusiveContextDef,
    ObjectDef,
    ObjectModel,
    StateDef,
)
from common.spec_ast import (
    BoundaryDecl,
    BodyMember,
    Block,
    ContextGuardDecl,
    EnumDecl,
    TransitionDecl,
    ExclusiveContextDecl,
    FunctionDecl,
    LockDecl,
    ObjectDecl,
    ProcessDecl,
    PredicateDecl,
    SourceSpan,
    StateDecl,
    TypeDecl,
    WithinDecl,
    DriveStatement,
)


def model_json_to_object_model(data: dict[str, Any]) -> ObjectModel:
    """Deserialize model intermediate JSON into an ObjectModel."""

    _require_schema(data, MODEL_SCHEMA, MODEL_VERSION)
    if not _object(data, "summary").get("ok", False):
        raise ValueError("model summary is not ok")

    model_data = _object(data, "model")
    enums = {
        name: _enum_from_json(item)
        for name, item in _object(model_data, "enums").items()
    }
    functions = {
        name: [_function_from_json(item) for item in _as_list(items, f"functions.{name}")]
        for name, items in _object(model_data, "functions").items()
    }
    predicates = {
        name: [_predicate_from_json(item) for item in _as_list(items, f"predicates.{name}")]
        for name, items in _object(model_data, "predicates").items()
    }
    types = {
        name: _type_from_json(item)
        for name, item in _object(model_data, "types").items()
    }
    locks = {
        name: _lock_from_json(item)
        for name, item in _object(model_data, "locks").items()
    }
    exclusive_contexts = {
        name: _exclusive_context_from_json(item)
        for name, item in _object(model_data, "exclusive_contexts").items()
    }
    objects = {
        name: _object_def_from_json(item)
        for name, item in _object(model_data, "objects").items()
    }
    boundaries = {
        boundary_id: _boundary_def_from_json(item)
        for boundary_id, item in _object(model_data, "boundaries").items()
    }
    declaration_sites = tuple(
        _declaration_site_from_json(item)
        for item in _list(model_data, "declaration_sites")
    )
    children = {
        name: [_as_string(child, f"children.{name}") for child in _as_list(items, f"children.{name}")]
        for name, items in _object(model_data, "children").items()
    }
    return ObjectModel(
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
        legacy_boundary_count=_integer(
            _object(data, "summary"), "legacy_boundaries"
        ),
    )


def _lock_from_json(item: Any) -> LockDecl:
    data = _as_object(item, "lock")
    kind = data.get("kind")
    if kind is not None and not isinstance(kind, str):
        raise ValueError("lock.kind must be a string or null")
    return LockDecl(
        name=_string(data, "name"),
        span=_span_from_json(data["span"]),
        kind=kind,
    )


def _exclusive_context_from_json(item: Any) -> ExclusiveContextDef:
    data = _as_object(item, "exclusive_context")
    kind = data.get("kind")
    if kind is not None and not isinstance(kind, str):
        raise ValueError("exclusive_context.kind must be a string or null")
    guard = data.get("guard")
    if guard is not None:
        guard = _context_guard_from_json(guard)
    lock_ref = data.get("lock_ref")
    if lock_ref is not None and not isinstance(lock_ref, str):
        raise ValueError("exclusive_context.lock_ref must be a string or null")
    properties = data.get("properties", {})
    if not isinstance(properties, dict):
        raise ValueError("exclusive_context.properties must be an object")
    decl = ExclusiveContextDecl(
        name=_string(data, "name"),
        span=_span_from_json(data["span"]),
        kind=kind,
        guard=guard,
        lock_ref=lock_ref,
        obj_refs=[_as_string(value, "obj_ref") for value in _list(data, "obj_refs")],
        effects=[_block_from_json(block) for block in _list(data, "effects")],
        other_blocks=[_block_from_json(block) for block in _list(data, "other_blocks")],
        properties={str(key): str(value) for key, value in properties.items()},
    )
    return ExclusiveContextDef(
        name=decl.name,
        decl=decl,
        kind=decl.kind,
        guard=decl.guard,
        lock_ref=decl.lock_ref,
        obj_refs=tuple(decl.obj_refs),
    )


def _context_guard_from_json(item: Any) -> ContextGuardDecl:
    data = _as_object(item, "context_guard")
    lock_ref = data.get("lock_ref")
    if lock_ref is not None and not isinstance(lock_ref, str):
        raise ValueError("context_guard.lock_ref must be a string or null")
    properties = data.get("properties", {})
    if not isinstance(properties, dict):
        raise ValueError("context_guard.properties must be an object")
    return ContextGuardDecl(
        span=_span_from_json(data["span"]),
        lock_ref=lock_ref,
        entered_by=[_block_from_json(block) for block in _list(data, "entered_by")],
        exited_by=[_block_from_json(block) for block in _list(data, "exited_by")],
        holds=[_block_from_json(block) for block in data.get("holds", [])],
        other_blocks=[_block_from_json(block) for block in _list(data, "other_blocks")],
        properties={str(key): str(value) for key, value in properties.items()},
    )


def _object_def_from_json(item: Any) -> ObjectDef:
    data = _as_object(item, "object")
    states = {
        name: _state_def_from_json(state)
        for name, state in _object(data, "states").items()
    }
    decl = ObjectDecl(
        name=_string(data, "name"),
        kind=_string(data, "kind"),
        span=_span_from_json(data["span"]),
        initial_state=_optional_string(data.get("initial_state"), "initial_state"),
        parent=_optional_string(data.get("parent"), "parent"),
        attrs=[],
        references=[],
        states=[state.decl for state in states.values()],
        processes=[_process_from_json(process) for process in _list(data, "processes")],
        other_blocks=[],
        properties={str(key): str(value) for key, value in _object(data, "properties").items()},
    )
    return ObjectDef(
        name=decl.name,
        kind=decl.kind,
        decl=decl,
        initial_state=decl.initial_state,
        parent=decl.parent,
        states=states,
        children=[
            _as_string(child, "object.children")
            for child in _list(data, "children")
        ],
        attrs={str(key): str(value) for key, value in _object(data, "attrs").items()},
    )


def _state_def_from_json(item: Any) -> StateDef:
    data = _as_object(item, "state")
    transitions = {
        name: _event_def_from_json(transition)
        for name, transition in _object(data, "transitions").items()
    }
    decl = StateDecl(
        name=_string(data, "name"),
        span=_span_from_json(data["span"]),
        invariants=[_block_from_json(block) for block in _list(data, "invariants")],
        boundaries=[
            _boundary_decl_from_json(boundary)
            for boundary in data.get("boundaries", [])
        ],
        deferred=[_block_from_json(block) for block in _list(data, "deferred")],
        transitions=[transition.decl for transition in transitions.values()],
        processes=[_process_from_json(process) for process in _list(data, "processes")],
        other_blocks=[_block_from_json(block) for block in _list(data, "other_blocks")],
    )
    return StateDef(
        name=decl.name,
        object_name=_string(data, "object_name"),
        decl=decl,
        transitions=transitions,
    )


def _event_def_from_json(item: Any) -> TransitionDef:
    data = _as_object(item, "transition")
    depends_on = [_block_from_json(block) for block in _list(data, "depends_on")]
    drives = [_block_from_json(block) for block in _list(data, "drives")]
    emits = [_block_from_json(block) for block in data.get("emits", [])]
    within = [_within_from_json(block) for block in _list(data, "within")]
    may_change = [_block_from_json(block) for block in _list(data, "may_change")]
    ensures = [_block_from_json(block) for block in _list(data, "ensures")]
    boundaries = [
        _boundary_decl_from_json(boundary)
        for boundary in data.get("boundaries", [])
    ]
    deferred = [_block_from_json(block) for block in _list(data, "deferred")]
    other_blocks = [_block_from_json(block) for block in _list(data, "other_blocks")]
    body_members = _body_members_from_json(
        data,
        fallback=[
            *(_block_body_member(block) for block in depends_on),
            *(_block_body_member(block) for block in drives),
            *(_block_body_member(block) for block in emits),
            *(_within_body_member(block) for block in within),
            *(_block_body_member(block) for block in may_change),
            *(_block_body_member(block) for block in ensures),
            *(_boundary_body_member(boundary) for boundary in boundaries),
            *(_block_body_member(block) for block in deferred),
            *(_block_body_member(block) for block in other_blocks),
        ],
    )
    decl = TransitionDecl(
        name=_string(data, "name"),
        target_state=_string(data, "target_state"),
        span=_span_from_json(data["span"]),
        depends_on=depends_on,
        drives=drives,
        emits=emits,
        within=within,
        may_change=may_change,
        ensures=ensures,
        boundaries=boundaries,
        deferred=deferred,
        other_blocks=other_blocks,
        body_members=body_members,
    )
    return TransitionDef(
        name=decl.name,
        object_name=_string(data, "object_name"),
        source_state=_string(data, "source_state"),
        target_state=decl.target_state,
        decl=decl,
    )


def _enum_from_json(item: Any) -> EnumDecl:
    data = _as_object(item, "enum")
    return EnumDecl(
        name=_string(data, "name"),
        variants=[_as_string(value, "enum variant") for value in _list(data, "variants")],
        span=_span_from_json(data["span"]),
    )


def _function_from_json(item: Any) -> FunctionDecl:
    data = _as_object(item, "function")
    return FunctionDecl(
        name=_string(data, "name"),
        signature=_string(data, "signature"),
        span=_span_from_json(data["span"]),
    )


def _predicate_from_json(item: Any) -> PredicateDecl:
    data = _as_object(item, "predicate")
    return PredicateDecl(
        name=_string(data, "name"),
        signature=_string(data, "signature"),
        span=_span_from_json(data["span"]),
        body=_optional_string(data.get("body"), "body"),
    )


def _type_from_json(item: Any) -> TypeDecl:
    data = _as_object(item, "type")
    properties = data.get("properties", {})
    if not isinstance(properties, dict):
        raise ValueError("type.properties must be an object")
    return TypeDecl(
        name=_string(data, "name"),
        header=_string(data, "header"),
        span=_span_from_json(data["span"]),
        blocks=[_block_from_json(block) for block in _list(data, "blocks")],
        processes=[_process_from_json(process) for process in _list(data, "processes")],
        properties={str(key): str(value) for key, value in properties.items()},
    )


def _process_from_json(item: Any) -> ProcessDecl:
    data = _as_object(item, "process")
    depends_on = [_block_from_json(block) for block in _list(data, "depends_on")]
    drives = [_block_from_json(block) for block in _list(data, "drives")]
    within = [_within_from_json(block) for block in _list(data, "within")]
    may_change = [_block_from_json(block) for block in _list(data, "may_change")]
    ensures = [_block_from_json(block) for block in _list(data, "ensures")]
    result = [_block_from_json(block) for block in _list(data, "result")]
    other_blocks = [_block_from_json(block) for block in _list(data, "other_blocks")]
    body_members = _body_members_from_json(
        data,
        fallback=[
            *(_block_body_member(block) for block in depends_on),
            *(_block_body_member(block) for block in drives),
            *(_within_body_member(block) for block in within),
            *(_block_body_member(block) for block in may_change),
            *(_block_body_member(block) for block in ensures),
            *(_block_body_member(block) for block in result),
            *(_block_body_member(block) for block in other_blocks),
        ],
    )
    parameters: list[tuple[str, str]] = []
    for parameter in _list(data, "parameters"):
        param = _as_object(parameter, "process parameter")
        parameters.append((_string(param, "name"), _string(param, "type")))
    return ProcessDecl(
        kind=_string(data, "kind"),
        name=_string(data, "name"),
        span=_span_from_json(data["span"]),
        parameters=tuple(parameters),
        return_type=_optional_string(data.get("return_type"), "process.return_type"),
        depends_on=depends_on,
        drives=drives,
        within=within,
        may_change=may_change,
        ensures=ensures,
        result=result,
        other_blocks=other_blocks,
        body_members=body_members,
        properties=_string_map(data.get("properties", {}), "process.properties"),
    )


def _declaration_site_from_json(item: Any) -> DeclarationSiteDef:
    data = _as_object(item, "declaration site")
    if data.get("kind") != "declare" or data.get("static_object") is not None:
        raise ValueError("invalid declaration site shape")
    return DeclarationSiteDef(
        owner_process=_string(data, "owner_process"),
        ordinal=_integer(data, "source_ordinal"),
        alias=_string(data, "alias"),
        declared_type=_string(data, "declared_type"),
        span=_span_from_json(data["span"]),
    )


def _within_from_json(item: Any) -> WithinDecl:
    data = _as_object(item, "within")
    entered_by = [_block_from_json(block) for block in _list(data, "entered_by")]
    depends_on = [_block_from_json(block) for block in _list(data, "depends_on")]
    drives = [_block_from_json(block) for block in _list(data, "drives")]
    within = [_within_from_json(block) for block in _list(data, "within")]
    exited_by = [_block_from_json(block) for block in _list(data, "exited_by")]
    may_change = [_block_from_json(block) for block in _list(data, "may_change")]
    ensures = [_block_from_json(block) for block in _list(data, "ensures")]
    boundaries = [
        _boundary_decl_from_json(boundary)
        for boundary in data.get("boundaries", [])
    ]
    deferred = [_block_from_json(block) for block in _list(data, "deferred")]
    other_blocks = [_block_from_json(block) for block in _list(data, "other_blocks")]
    body_members = _body_members_from_json(
        data,
        fallback=[
            *(_block_body_member(block) for block in entered_by),
            *(_block_body_member(block) for block in depends_on),
            *(_block_body_member(block) for block in drives),
            *(_within_body_member(block) for block in within),
            *(_block_body_member(block) for block in exited_by),
            *(_block_body_member(block) for block in may_change),
            *(_block_body_member(block) for block in ensures),
            *(_boundary_body_member(boundary) for boundary in boundaries),
            *(_block_body_member(block) for block in deferred),
            *(_block_body_member(block) for block in other_blocks),
        ],
    )
    return WithinDecl(
        context=_string(data, "context"),
        span=_span_from_json(data["span"]),
        only_once=_bool(data.get("only_once", False), "within.only_once"),
        parameters=_string_map(data.get("parameters", {}), "within.parameters"),
        entered_by=entered_by,
        depends_on=depends_on,
        drives=drives,
        within=within,
        exited_by=exited_by,
        may_change=may_change,
        ensures=ensures,
        boundaries=boundaries,
        deferred=deferred,
        other_blocks=other_blocks,
        body_members=body_members,
    )


def _body_members_from_json(
    data: dict[str, Any], *, fallback: list[BodyMember]
) -> list[BodyMember]:
    if "body_members" not in data:
        return fallback
    return [_body_member_from_json(member) for member in _list(data, "body_members")]


def _body_member_from_json(item: Any) -> BodyMember:
    data = _as_object(item, "body_member")
    block = data.get("block")
    within = data.get("within")
    boundary = data.get("boundary")
    return BodyMember(
        kind=_string(data, "kind"),
        span=_span_from_json(data["span"]),
        block=_block_from_json(block) if block is not None else None,
        within=_within_from_json(within) if within is not None else None,
        boundary=(
            _boundary_decl_from_json(boundary)
            if boundary is not None
            else None
        ),
    )


def _block_body_member(block: Block) -> BodyMember:
    return BodyMember(kind=block.kind, span=block.span, block=block)


def _within_body_member(within: WithinDecl) -> BodyMember:
    return BodyMember(kind="within", span=within.span, within=within)


def _boundary_body_member(boundary: BoundaryDecl) -> BodyMember:
    return BodyMember(
        kind=boundary.status,
        span=boundary.span,
        boundary=boundary,
    )


def _boundary_decl_from_json(item: Any) -> BoundaryDecl:
    data = _as_object(item, "boundary")
    return BoundaryDecl(
        status=_string(data, "status"),
        id=_string(data, "id"),
        span=_span_from_json(data["span"]),
        category=_optional_string(data.get("category"), "boundary.category"),
        summary=_optional_string(data.get("summary"), "boundary.summary"),
        evidence=[
            _block_from_json(block) for block in data.get("evidence", [])
        ],
        resolution=_optional_string(
            data.get("resolution"), "boundary.resolution"
        ),
        property_counts={
            str(key): int(value)
            for key, value in data.get("property_counts", {}).items()
        },
        other_blocks=[
            _block_from_json(block) for block in data.get("other_blocks", [])
        ],
        unknown_properties={
            str(key): str(value)
            for key, value in data.get("unknown_properties", {}).items()
        },
    )


def _boundary_def_from_json(item: Any) -> BoundaryDef:
    data = _as_object(item, "boundary")
    decl_data = dict(data)
    decl_data["category"] = data.get("source_category")
    decl = _boundary_decl_from_json(decl_data)
    return BoundaryDef(
        id=decl.id,
        status=decl.status,
        category=_string(data, "category"),
        summary=_string(data, "summary"),
        resolution=_string(data, "resolution"),
        decl=decl,
        object_name=_string(data, "object_name"),
        state_name=_string(data, "state_name"),
        transition_name=_optional_string(
            data.get("transition_name"), "boundary.transition_name"
        ),
        context_path=tuple(
            _as_string(value, "boundary.context_path")
            for value in data.get("context_path", [])
        ),
    )


def _block_from_json(item: Any) -> Block:
    data = _as_object(item, "block")
    body_start_line = data.get("body_start_line")
    if body_start_line is not None and not isinstance(body_start_line, int):
        raise ValueError("block.body_start_line must be an integer or null")
    return Block(
        kind=_string(data, "kind"),
        body=_string(data, "body"),
        span=_span_from_json(data["span"]),
        header=_string(data, "header"),
        body_start_line=body_start_line,
        statements=[_drive_statement_from_json(value) for value in data.get("statements", [])],
    )


def _drive_statement_from_json(item: Any) -> DriveStatement:
    data = _as_object(item, "drive statement")
    return DriveStatement(
        kind=_string(data, "kind"),
        text=_string(data, "text"),
        span=_span_from_json(data["span"]),
        ordinal=_integer(data, "ordinal"),
        alias=_optional_string(data.get("alias"), "drive statement.alias"),
        declared_type=_optional_string(
            data.get("declared_type"), "drive statement.declared_type"
        ),
        owner_process=_optional_string(
            data.get("owner_process"), "drive statement.owner_process"
        ),
    )


def _span_from_json(item: Any) -> SourceSpan:
    data = _as_object(item, "span")
    source_file = data.get("source_file")
    source_line = data.get("source_line")
    if source_file is not None and not isinstance(source_file, str):
        raise ValueError("span.source_file must be a string or null")
    if source_line is not None and not isinstance(source_line, int):
        raise ValueError("span.source_line must be an integer or null")
    return SourceSpan(
        start_line=_integer(data, "start_line"),
        end_line=_integer(data, "end_line"),
        source_file=source_file,
        source_line=source_line,
    )


def _require_schema(data: dict[str, Any], schema: str, version: int) -> None:
    if data.get("schema") != schema:
        raise ValueError(f"expected schema {schema!r}")
    if data.get("version") != version:
        raise ValueError(f"expected version {version}")


def _object(data: dict[str, Any], key: str) -> dict[str, Any]:
    return _as_object(data.get(key), key)


def _list(data: dict[str, Any], key: str) -> list[Any]:
    return _as_list(data.get(key), key)


def _string_map(value: Any, name: str) -> dict[str, str]:
    if not isinstance(value, dict):
        raise ValueError(f"{name} must be an object")
    return {str(key): str(item) for key, item in value.items()}


def _bool(value: Any, name: str) -> bool:
    if not isinstance(value, bool):
        raise ValueError(f"{name} must be a boolean")
    return value


def _string(data: dict[str, Any], key: str) -> str:
    return _as_string(data.get(key), key)


def _integer(data: dict[str, Any], key: str) -> int:
    value = data.get(key)
    if not isinstance(value, int):
        raise ValueError(f"{key} must be an integer")
    return value


def _optional_string(value: Any, name: str) -> str | None:
    if value is None:
        return None
    return _as_string(value, name)


def _as_object(value: Any, name: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError(f"{name} must be an object")
    return value


def _as_list(value: Any, name: str) -> list[Any]:
    if not isinstance(value, list):
        raise ValueError(f"{name} must be a list")
    return value


def _as_string(value: Any, name: str) -> str:
    if not isinstance(value, str):
        raise ValueError(f"{name} must be a string")
    return value
