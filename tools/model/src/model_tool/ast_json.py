"""AST JSON deserialization for the model stage."""

from __future__ import annotations

from typing import Any

from common import AST_SCHEMA, AST_VERSION
from pyveri.ast import (
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


def ast_json_to_document(data: dict[str, Any]) -> SpecDocument:
    """Deserialize AST intermediate JSON into a pyveri syntax document."""

    _require_schema(data, AST_SCHEMA, AST_VERSION)
    document = _object(data, "document")
    return SpecDocument(
        enums=[_enum_from_json(item) for item in _list(document, "enums")],
        functions=[_function_from_json(item) for item in _list(document, "functions")],
        predicates=[_predicate_from_json(item) for item in _list(document, "predicates")],
        types=[_type_from_json(item) for item in _list(document, "types")],
        objects=[_object_from_json(item) for item in _list(document, "objects")],
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
    body = data.get("body")
    if body is not None and not isinstance(body, str):
        raise ValueError("predicate.body must be a string or null")
    return PredicateDecl(
        name=_string(data, "name"),
        signature=_string(data, "signature"),
        span=_span_from_json(data["span"]),
        body=body,
    )


def _type_from_json(item: Any) -> TypeDecl:
    data = _as_object(item, "type")
    return TypeDecl(
        name=_string(data, "name"),
        header=_string(data, "header"),
        span=_span_from_json(data["span"]),
        blocks=[_block_from_json(block) for block in _list(data, "blocks")],
    )


def _object_from_json(item: Any) -> ObjectDecl:
    data = _as_object(item, "object")
    initial_state = data.get("initial_state")
    parent = data.get("parent")
    if initial_state is not None and not isinstance(initial_state, str):
        raise ValueError("object.initial_state must be a string or null")
    if parent is not None and not isinstance(parent, str):
        raise ValueError("object.parent must be a string or null")
    properties = data.get("properties", {})
    if not isinstance(properties, dict):
        raise ValueError("object.properties must be an object")
    return ObjectDecl(
        name=_string(data, "name"),
        kind=_string(data, "kind"),
        span=_span_from_json(data["span"]),
        initial_state=initial_state,
        parent=parent,
        attrs=[_block_from_json(block) for block in _list(data, "attrs")],
        references=[_block_from_json(block) for block in _list(data, "references")],
        states=[_state_from_json(state) for state in _list(data, "states")],
        other_blocks=[_block_from_json(block) for block in _list(data, "other_blocks")],
        properties={str(key): str(value) for key, value in properties.items()},
    )


def _state_from_json(item: Any) -> StateDecl:
    data = _as_object(item, "state")
    return StateDecl(
        name=_string(data, "name"),
        span=_span_from_json(data["span"]),
        invariants=[_block_from_json(block) for block in _list(data, "invariants")],
        deferred=[_block_from_json(block) for block in _list(data, "deferred")],
        events=[_event_from_json(event) for event in _list(data, "events")],
        other_blocks=[_block_from_json(block) for block in _list(data, "other_blocks")],
    )


def _event_from_json(item: Any) -> EventDecl:
    data = _as_object(item, "event")
    return EventDecl(
        name=_string(data, "name"),
        target_state=_string(data, "target_state"),
        span=_span_from_json(data["span"]),
        depends_on=[_block_from_json(block) for block in _list(data, "depends_on")],
        drives=[_block_from_json(block) for block in _list(data, "drives")],
        may_change=[_block_from_json(block) for block in _list(data, "may_change")],
        deferred=[_block_from_json(block) for block in _list(data, "deferred")],
        other_blocks=[_block_from_json(block) for block in _list(data, "other_blocks")],
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
    value = data.get(key)
    if not isinstance(value, list):
        raise ValueError(f"{key} must be a list")
    return value


def _string(data: dict[str, Any], key: str) -> str:
    return _as_string(data.get(key), key)


def _integer(data: dict[str, Any], key: str) -> int:
    value = data.get(key)
    if not isinstance(value, int):
        raise ValueError(f"{key} must be an integer")
    return value


def _as_object(value: Any, name: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError(f"{name} must be an object")
    return value


def _as_string(value: Any, name: str) -> str:
    if not isinstance(value, str):
        raise ValueError(f"{name} must be a string")
    return value
