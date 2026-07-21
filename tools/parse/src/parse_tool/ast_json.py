"""AST JSON serialization for the parse stage."""

from __future__ import annotations

from pathlib import Path
from typing import Any

from common import AST_SCHEMA, AST_VERSION
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
    SpecDocument,
    StateDecl,
    TypeDecl,
    WithinDecl,
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
            "locks": [_lock_to_json(item) for item in document.locks],
            "exclusive_contexts": [
                _exclusive_context_to_json(item)
                for item in document.exclusive_contexts
            ],
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
        "processes": [_process_to_json(process) for process in item.processes],
        "properties": item.properties,
    }


def _lock_to_json(item: LockDecl) -> dict[str, Any]:
    return {
        "name": item.name,
        "kind": item.kind,
        "span": _span_to_json(item.span),
    }


def _exclusive_context_to_json(item: ExclusiveContextDecl) -> dict[str, Any]:
    return {
        "name": item.name,
        "span": _span_to_json(item.span),
        "kind": item.kind,
        "guard": _context_guard_to_json(item.guard),
        "lock_ref": item.lock_ref,
        "obj_refs": item.obj_refs,
        "effects": [_block_to_json(block) for block in item.effects],
        "other_blocks": [_block_to_json(block) for block in item.other_blocks],
        "properties": item.properties,
    }


def _context_guard_to_json(item: ContextGuardDecl | None) -> dict[str, Any] | None:
    if item is None:
        return None
    return {
        "span": _span_to_json(item.span),
        "lock_ref": item.lock_ref,
        "entered_by": [_block_to_json(block) for block in item.entered_by],
        "exited_by": [_block_to_json(block) for block in item.exited_by],
        "holds": [_block_to_json(block) for block in item.holds],
        "other_blocks": [_block_to_json(block) for block in item.other_blocks],
        "properties": item.properties,
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
        "processes": [_process_to_json(process) for process in item.processes],
        "other_blocks": [_block_to_json(block) for block in item.other_blocks],
        "properties": item.properties,
    }


def _state_to_json(item: StateDecl) -> dict[str, Any]:
    return {
        "name": item.name,
        "span": _span_to_json(item.span),
        "invariants": [_block_to_json(block) for block in item.invariants],
        "boundaries": [_boundary_to_json(boundary) for boundary in item.boundaries],
        "deferred": [_block_to_json(block) for block in item.deferred],
        "transitions": [_event_to_json(transition) for transition in item.transitions],
        "processes": [_process_to_json(process) for process in item.processes],
        "other_blocks": [_block_to_json(block) for block in item.other_blocks],
    }


def _event_to_json(item: TransitionDecl) -> dict[str, Any]:
    return {
        "name": item.name,
        "target_state": item.target_state,
        "span": _span_to_json(item.span),
        "depends_on": [_block_to_json(block) for block in item.depends_on],
        "drives": [_block_to_json(block) for block in item.drives],
        "emits": [_block_to_json(block) for block in item.emits],
        "within": [_within_to_json(block) for block in item.within],
        "may_change": [_block_to_json(block) for block in item.may_change],
        "ensures": [_block_to_json(block) for block in item.ensures],
        "boundaries": [_boundary_to_json(boundary) for boundary in item.boundaries],
        "deferred": [_block_to_json(block) for block in item.deferred],
        "other_blocks": [_block_to_json(block) for block in item.other_blocks],
        "body_members": [_body_member_to_json(member) for member in item.body_members],
    }


def _process_to_json(item: ProcessDecl) -> dict[str, Any]:
    return {
        "kind": item.kind,
        "name": item.name,
        "span": _span_to_json(item.span),
        "parameters": [
            {"name": name, "type": type_name}
            for name, type_name in item.parameters
        ],
        "return_type": item.return_type,
        "depends_on": [_block_to_json(block) for block in item.depends_on],
        "drives": [_block_to_json(block) for block in item.drives],
        "within": [_within_to_json(block) for block in item.within],
        "may_change": [_block_to_json(block) for block in item.may_change],
        "ensures": [_block_to_json(block) for block in item.ensures],
        "result": [_block_to_json(block) for block in item.result],
        "other_blocks": [_block_to_json(block) for block in item.other_blocks],
        "body_members": [_body_member_to_json(member) for member in item.body_members],
        "properties": item.properties,
    }


def _within_to_json(item: WithinDecl) -> dict[str, Any]:
    return {
        "context": item.context,
        "only_once": item.only_once,
        "parameters": item.parameters,
        "span": _span_to_json(item.span),
        "entered_by": [_block_to_json(block) for block in item.entered_by],
        "depends_on": [_block_to_json(block) for block in item.depends_on],
        "drives": [_block_to_json(block) for block in item.drives],
        "within": [_within_to_json(block) for block in item.within],
        "exited_by": [_block_to_json(block) for block in item.exited_by],
        "may_change": [_block_to_json(block) for block in item.may_change],
        "ensures": [_block_to_json(block) for block in item.ensures],
        "boundaries": [_boundary_to_json(boundary) for boundary in item.boundaries],
        "deferred": [_block_to_json(block) for block in item.deferred],
        "other_blocks": [_block_to_json(block) for block in item.other_blocks],
        "body_members": [_body_member_to_json(member) for member in item.body_members],
    }


def _body_member_to_json(item: BodyMember) -> dict[str, Any]:
    return {
        "kind": item.kind,
        "span": _span_to_json(item.span),
        "block": _block_to_json(item.block) if item.block is not None else None,
        "within": _within_to_json(item.within) if item.within is not None else None,
        "boundary": (
            _boundary_to_json(item.boundary) if item.boundary is not None else None
        ),
    }


def _boundary_to_json(item: BoundaryDecl) -> dict[str, Any]:
    return {
        "status": item.status,
        "id": item.id,
        "category": item.category,
        "summary": item.summary,
        "evidence": [_block_to_json(block) for block in item.evidence],
        "resolution": item.resolution,
        "span": _span_to_json(item.span),
        "property_counts": item.property_counts,
        "other_blocks": [_block_to_json(block) for block in item.other_blocks],
        "unknown_properties": item.unknown_properties,
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
        "statements": [
            {
                "kind": statement.kind,
                "text": statement.text,
                "span": _span_to_json(statement.span),
                "ordinal": statement.ordinal,
                "alias": statement.alias,
                "declared_type": statement.declared_type,
                "owner_process": statement.owner_process,
            }
            for statement in item.statements
        ],
    }


def _span_to_json(span: SourceSpan) -> dict[str, int | str | None]:
    result: dict[str, int | str | None] = {
        "start_line": span.start_line,
        "end_line": span.end_line,
    }
    if span.source_file is not None:
        result["source_file"] = span.source_file
        result["source_line"] = span.source_line
    return result
