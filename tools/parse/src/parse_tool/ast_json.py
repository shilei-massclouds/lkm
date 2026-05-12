"""AST JSON serialization for the parse stage."""

from __future__ import annotations

from pathlib import Path
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


def document_to_ast_json(document: SpecDocument, source: str | Path) -> dict[str, Any]:
    """Serialize a parsed spec document to the AST intermediate format."""

    return {
        "schema": AST_SCHEMA,
        "version": AST_VERSION,
        "source": str(source),
        "document": {
            "enums": [_enum_to_json(item) for item in document.enums],
            "functions": [_function_to_json(item) for item in document.functions],
            "predicates": [_predicate_to_json(item) for item in document.predicates],
            "types": [_type_to_json(item) for item in document.types],
            "objects": [_object_to_json(item) for item in document.objects],
        },
    }


def _enum_to_json(item: EnumDecl) -> dict[str, Any]:
    return {
        "name": item.name,
        "variants": item.variants,
        "span": _span_to_json(item.span),
    }


def _function_to_json(item: FunctionDecl) -> dict[str, Any]:
    return {
        "name": item.name,
        "signature": item.signature,
        "span": _span_to_json(item.span),
    }


def _predicate_to_json(item: PredicateDecl) -> dict[str, Any]:
    return {
        "name": item.name,
        "signature": item.signature,
        "span": _span_to_json(item.span),
        "body": item.body,
    }


def _type_to_json(item: TypeDecl) -> dict[str, Any]:
    return {
        "name": item.name,
        "header": item.header,
        "span": _span_to_json(item.span),
        "blocks": [_block_to_json(block) for block in item.blocks],
    }


def _object_to_json(item: ObjectDecl) -> dict[str, Any]:
    return {
        "name": item.name,
        "kind": item.kind,
        "span": _span_to_json(item.span),
        "initial_state": item.initial_state,
        "parent": item.parent,
        "attrs": [_block_to_json(block) for block in item.attrs],
        "references": [_block_to_json(block) for block in item.references],
        "states": [_state_to_json(state) for state in item.states],
        "other_blocks": [_block_to_json(block) for block in item.other_blocks],
        "properties": item.properties,
    }


def _state_to_json(item: StateDecl) -> dict[str, Any]:
    return {
        "name": item.name,
        "span": _span_to_json(item.span),
        "invariants": [_block_to_json(block) for block in item.invariants],
        "deferred": [_block_to_json(block) for block in item.deferred],
        "events": [_event_to_json(event) for event in item.events],
        "other_blocks": [_block_to_json(block) for block in item.other_blocks],
    }


def _event_to_json(item: EventDecl) -> dict[str, Any]:
    return {
        "name": item.name,
        "target_state": item.target_state,
        "span": _span_to_json(item.span),
        "depends_on": [_block_to_json(block) for block in item.depends_on],
        "drives": [_block_to_json(block) for block in item.drives],
        "may_change": [_block_to_json(block) for block in item.may_change],
        "deferred": [_block_to_json(block) for block in item.deferred],
        "other_blocks": [_block_to_json(block) for block in item.other_blocks],
    }


def _block_to_json(item: Block) -> dict[str, Any]:
    return {
        "kind": item.kind,
        "header": item.header,
        "body": item.body,
        "span": _span_to_json(item.span),
        "body_start_line": item.body_start_line,
        "entries": [
            {
                "text": entry,
                "span": _span_to_json(span),
            }
            for entry, span in item.entry_spans
        ],
    }


def _span_to_json(span: SourceSpan) -> dict[str, int]:
    return {
        "start_line": span.start_line,
        "end_line": span.end_line,
    }
