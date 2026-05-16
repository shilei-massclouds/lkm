"""Model JSON deserialization shared by stage tools."""

from __future__ import annotations

from typing import Any

from common.schemas import MODEL_SCHEMA, MODEL_VERSION
from common.model_types import EventDef, ObjectDef, ObjectModel, StateDef
from common.spec_ast import (
    Block,
    EnumDecl,
    EventDecl,
    FunctionDecl,
    ObjectDecl,
    PredicateDecl,
    SourceSpan,
    StateDecl,
    TypeDecl,
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
    objects = {
        name: _object_def_from_json(item)
        for name, item in _object(model_data, "objects").items()
    }
    children = {
        name: [_as_string(child, f"children.{name}") for child in _as_list(items, f"children.{name}")]
        for name, items in _object(model_data, "children").items()
    }
    return ObjectModel(
        enums=enums,
        functions=functions,
        predicates=predicates,
        types=types,
        objects=objects,
        children=children,
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
    events = {
        name: _event_def_from_json(event)
        for name, event in _object(data, "events").items()
    }
    decl = StateDecl(
        name=_string(data, "name"),
        span=_span_from_json(data["span"]),
        invariants=[_block_from_json(block) for block in _list(data, "invariants")],
        deferred=[_block_from_json(block) for block in _list(data, "deferred")],
        events=[event.decl for event in events.values()],
        other_blocks=[_block_from_json(block) for block in _list(data, "other_blocks")],
    )
    return StateDef(
        name=decl.name,
        object_name=_string(data, "object_name"),
        decl=decl,
        events=events,
    )


def _event_def_from_json(item: Any) -> EventDef:
    data = _as_object(item, "event")
    decl = EventDecl(
        name=_string(data, "name"),
        target_state=_string(data, "target_state"),
        span=_span_from_json(data["span"]),
        depends_on=[_block_from_json(block) for block in _list(data, "depends_on")],
        drives=[_block_from_json(block) for block in _list(data, "drives")],
        may_change=[_block_from_json(block) for block in _list(data, "may_change")],
        ensures=[_block_from_json(block) for block in _list(data, "ensures")],
        deferred=[_block_from_json(block) for block in _list(data, "deferred")],
        other_blocks=[_block_from_json(block) for block in _list(data, "other_blocks")],
    )
    return EventDef(
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
    return TypeDecl(
        name=_string(data, "name"),
        header=_string(data, "header"),
        span=_span_from_json(data["span"]),
        blocks=[_block_from_json(block) for block in _list(data, "blocks")],
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
    )


def _span_from_json(item: Any) -> SourceSpan:
    data = _as_object(item, "span")
    return SourceSpan(
        start_line=_integer(data, "start_line"),
        end_line=_integer(data, "end_line"),
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
