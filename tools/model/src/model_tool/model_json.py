"""Model JSON serialization for the model stage."""

from __future__ import annotations

from typing import Any

from common import MODEL_SCHEMA, MODEL_VERSION
from common.model_types import BuildResult, Diagnostic, EventDef, ObjectDef, StateDef
from common.spec_ast import Block, EnumDecl, FunctionDecl, PredicateDecl, SourceSpan, TypeDecl


def build_result_to_model_json(
    result: BuildResult, ast_data: dict[str, Any]
) -> dict[str, Any]:
    """Serialize a model build result to the model intermediate format."""

    model = result.model
    return {
        "schema": MODEL_SCHEMA,
        "version": MODEL_VERSION,
        "source": ast_data.get("source"),
        "input": {
            "schema": ast_data.get("schema"),
            "version": ast_data.get("version"),
        },
        "summary": {
            "ok": result.ok,
            "objects": len(model.objects),
            "states": model.state_count,
            "events": model.event_count,
            "errors": len(result.errors),
            "warnings": len(result.warnings),
        },
        "diagnostics": [_diagnostic_to_json(item) for item in result.diagnostics],
        "model": {
            "enums": {
                name: _enum_to_json(item)
                for name, item in sorted(model.enums.items())
            },
            "functions": {
                name: [_function_to_json(item) for item in items]
                for name, items in sorted(model.functions.items())
            },
            "predicates": {
                name: [_predicate_to_json(item) for item in items]
                for name, items in sorted(model.predicates.items())
            },
            "types": {
                name: _type_to_json(item)
                for name, item in sorted(model.types.items())
            },
            "objects": {
                name: _object_to_json(item)
                for name, item in sorted(model.objects.items())
            },
            "children": {
                name: children
                for name, children in sorted(model.children.items())
            },
        },
    }


def _diagnostic_to_json(item: Diagnostic) -> dict[str, Any]:
    return {
        "severity": item.severity.value,
        "message": item.message,
        "span": _optional_span_to_json(item.span),
    }


def _object_to_json(item: ObjectDef) -> dict[str, Any]:
    return {
        "name": item.name,
        "kind": item.kind,
        "span": _span_to_json(item.decl.span),
        "initial_state": item.initial_state,
        "parent": item.parent,
        "children": item.children,
        "attrs": item.attrs,
        "properties": item.decl.properties,
        "states": {
            name: _state_to_json(state)
            for name, state in sorted(item.states.items())
        },
    }


def _state_to_json(item: StateDef) -> dict[str, Any]:
    return {
        "name": item.name,
        "object_name": item.object_name,
        "span": _span_to_json(item.decl.span),
        "invariants": [_block_to_json(block) for block in item.decl.invariants],
        "deferred": [_block_to_json(block) for block in item.decl.deferred],
        "events": {
            name: _event_to_json(event)
            for name, event in sorted(item.events.items())
        },
        "other_blocks": [_block_to_json(block) for block in item.decl.other_blocks],
    }


def _event_to_json(item: EventDef) -> dict[str, Any]:
    decl = item.decl
    return {
        "name": item.name,
        "object_name": item.object_name,
        "source_state": item.source_state,
        "target_state": item.target_state,
        "span": _span_to_json(decl.span),
        "depends_on": [_block_to_json(block) for block in decl.depends_on],
        "drives": [_block_to_json(block) for block in decl.drives],
        "may_change": [_block_to_json(block) for block in decl.may_change],
        "ensures": [_block_to_json(block) for block in decl.ensures],
        "deferred": [_block_to_json(block) for block in decl.deferred],
        "other_blocks": [_block_to_json(block) for block in decl.other_blocks],
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


def _optional_span_to_json(span: SourceSpan | None) -> dict[str, int] | None:
    if span is None:
        return None
    return _span_to_json(span)


def _span_to_json(span: SourceSpan) -> dict[str, int]:
    return {
        "start_line": span.start_line,
        "end_line": span.end_line,
    }
