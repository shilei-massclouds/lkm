"""Model JSON serialization for the model stage."""

from __future__ import annotations

from typing import Any

from common import MODEL_SCHEMA, MODEL_VERSION
from common.model_types import (
    BoundaryDef,
    BuildResult,
    Diagnostic,
    DeclarationSiteDef,
    TransitionDef,
    ExclusiveContextDef,
    ObjectDef,
    StateDef,
)
from common.spec_ast import (
    BoundaryDecl,
    BodyMember,
    Block,
    ContextGuardDecl,
    EnumDecl,
    FunctionDecl,
    LockDecl,
    PredicateDecl,
    ProcessDecl,
    SourceSpan,
    TypeDecl,
    WithinDecl,
)


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
            "transitions": model.transition_count,
            "declaration_sites": len(model.declaration_sites),
            "deferred": model.deferred_count,
            "trimmed": model.trimmed_count,
            "legacy_boundaries": model.legacy_boundary_count,
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
            "locks": {
                name: _lock_to_json(item)
                for name, item in sorted(model.locks.items())
            },
            "exclusive_contexts": {
                name: _exclusive_context_to_json(item)
                for name, item in sorted(model.exclusive_contexts.items())
            },
            "objects": {
                name: _object_to_json(item)
                for name, item in sorted(model.objects.items())
            },
            "boundaries": {
                boundary_id: _boundary_def_to_json(boundary)
                for boundary_id, boundary in sorted(model.boundaries.items())
            },
            "declaration_sites": [
                _declaration_site_to_json(site)
                for site in model.declaration_sites
            ],
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
        "processes": [_process_to_json(process) for process in item.decl.processes],
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
        "boundaries": [
            _boundary_decl_to_json(boundary) for boundary in item.decl.boundaries
        ],
        "deferred": [_block_to_json(block) for block in item.decl.deferred],
        "transitions": {
            name: _event_to_json(transition)
            for name, transition in sorted(item.transitions.items())
        },
        "processes": [_process_to_json(process) for process in item.decl.processes],
        "other_blocks": [_block_to_json(block) for block in item.decl.other_blocks],
    }


def _event_to_json(item: TransitionDef) -> dict[str, Any]:
    decl = item.decl
    return {
        "name": item.name,
        "object_name": item.object_name,
        "source_state": item.source_state,
        "target_state": item.target_state,
        "span": _span_to_json(decl.span),
        "depends_on": [_block_to_json(block) for block in decl.depends_on],
        "drives": [_block_to_json(block) for block in decl.drives],
        "emits": [_block_to_json(block) for block in decl.emits],
        "within": [_within_to_json(block) for block in decl.within],
        "may_change": [_block_to_json(block) for block in decl.may_change],
        "ensures": [_block_to_json(block) for block in decl.ensures],
        "boundaries": [
            _boundary_decl_to_json(boundary) for boundary in decl.boundaries
        ],
        "deferred": [_block_to_json(block) for block in decl.deferred],
        "other_blocks": [_block_to_json(block) for block in decl.other_blocks],
        "body_members": [_body_member_to_json(member) for member in decl.body_members],
    }


def _lock_to_json(item: LockDecl) -> dict[str, Any]:
    return {
        "name": item.name,
        "kind": item.kind,
        "span": _span_to_json(item.span),
    }


def _exclusive_context_to_json(item: ExclusiveContextDef) -> dict[str, Any]:
    return {
        "name": item.name,
        "span": _span_to_json(item.decl.span),
        "kind": item.kind,
        "guard": _context_guard_to_json(item.guard),
        "lock_ref": item.lock_ref,
        "obj_refs": list(item.obj_refs),
        "effects": [_block_to_json(block) for block in item.decl.effects],
        "other_blocks": [_block_to_json(block) for block in item.decl.other_blocks],
        "properties": item.decl.properties,
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


def _declaration_site_to_json(item: DeclarationSiteDef) -> dict[str, Any]:
    return {
        "kind": "declare",
        "owner_process": item.owner_process,
        "source_ordinal": item.ordinal,
        "alias": item.alias,
        "declared_type": item.declared_type,
        "span": _span_to_json(item.span),
        "static_object": None,
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
        "boundaries": [
            _boundary_decl_to_json(boundary) for boundary in item.boundaries
        ],
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
            _boundary_decl_to_json(item.boundary)
            if item.boundary is not None
            else None
        ),
    }


def _boundary_def_to_json(item: BoundaryDef) -> dict[str, Any]:
    data = _boundary_decl_to_json(item.decl)
    data.update(
        {
            "category": item.category,
            "source_category": item.decl.category,
            "summary": item.summary,
            "resolution": item.resolution,
            "owner": item.owner,
            "object_name": item.object_name,
            "state_name": item.state_name,
            "transition_name": item.transition_name,
            "context_path": list(item.context_path),
        }
    )
    return data


def _boundary_decl_to_json(item: BoundaryDecl) -> dict[str, Any]:
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


def _optional_span_to_json(
    span: SourceSpan | None,
) -> dict[str, int | str | None] | None:
    if span is None:
        return None
    return _span_to_json(span)


def _span_to_json(span: SourceSpan) -> dict[str, int | str | None]:
    result: dict[str, int | str | None] = {
        "start_line": span.start_line,
        "end_line": span.end_line,
    }
    if span.source_file is not None:
        result["source_file"] = span.source_file
        result["source_line"] = span.source_line
    return result
