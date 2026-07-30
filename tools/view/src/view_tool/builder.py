"""Build view models from object models."""

from __future__ import annotations

from dataclasses import replace
import re
from typing import Any

from common.defaults import DEFAULT_TARGET
from common.emits import parse_emit_expression
from common.model_types import TransitionDef, ObjectModel, StateDef
from common.spec_ast import BodyMember, Block, WithinDecl
from common.view_types import (
    TimelineItem,
    TimelineRow,
    TraceArrow,
    TraceCell,
    ViewEdge,
    ViewModel,
    ViewNode,
)


_OBJECT_TRANSITION_RE = re.compile(r"\b([A-Z][A-Za-z0-9_]*)\.Transition::([A-Za-z_][A-Za-z0-9_]*)\b")
_TARGET_RE = re.compile(r"\A([A-Z][A-Za-z0-9_]*)\.Transition::([A-Za-z_][A-Za-z0-9_]*)\Z")
_OBJECT_TRANSITION_EXPR_RE = re.compile(
    r"\A([A-Z][A-Za-z0-9_]*)\.Transition::([A-Za-z_][A-Za-z0-9_]*)(?:\s*\((.*)\))?\Z",
    re.S,
)
_OBJECT_ACTION_RE = re.compile(r"\b([A-Z][A-Za-z0-9_]*)\.Action::([A-Za-z_][A-Za-z0-9_]*)\b")
_CONTEXTUAL_ACTION_RECEIVERS = frozenset({"CurrentTask"})
_LOCAL_TRANSITION_EXPR_RE = re.compile(r"\ATransition::([A-Za-z_][A-Za-z0-9_]*)\Z")
_OBJECT_STATE_RE = re.compile(
    r"\b([A-Z][A-Za-z0-9_]*)\.state\s*==\s*State::([A-Za-z_][A-Za-z0-9_]*)\b"
)
DEFAULT_TRACE_ACTION_DEPTH = 3
_LIFECYCLE_ROOT_OBJECTS = frozenset(
    {
        "Computer",
        "Riscv64Platform",
        "OpenSBI",
        "Kernel",
    }
)
_TRACE_PHASE_MIN_BODY_ROWS = 24


def build_object_view(model: ObjectModel) -> ViewModel:
    """Build an object-level view with static objects and declaration sites."""

    nodes = {
        name: ViewNode(id=name, label=name, kind=obj.kind)
        for name, obj in model.objects.items()
    }
    for external in model.externals.values():
        nodes[external.name] = ViewNode(
            id=external.name,
            label=external.name,
            kind="External",
        )
    edges: list[ViewEdge] = []

    for obj in model.objects.values():
        if obj.parent is not None:
            edges.append(
                ViewEdge(
                    source=obj.parent,
                    target=obj.name,
                    kind="parent",
                    label="parent",
                )
            )

    declaration_inventory: list[dict[str, object]] = []
    for site in model.declaration_sites:
        node_id = _declaration_site_node_id(
            site.owner_process,
            site.ordinal,
            site.alias,
        )
        nodes[node_id] = ViewNode(
            id=node_id,
            label=(
                f"declare {site.alias} of {site.declared_type}\n"
                f"{site.owner_process} statement {site.ordinal}"
            ),
            kind="DeclarationSite",
        )
        owner_id = f"process::{site.owner_process}"
        nodes.setdefault(
            owner_id,
            ViewNode(id=owner_id, label=site.owner_process, kind="Process"),
        )
        edges.append(
            ViewEdge(
                source=owner_id,
                target=node_id,
                kind="declares",
                label="runtime template",
            )
        )
        declaration_inventory.append(
            {
                "id": node_id,
                "owner_process": site.owner_process,
                "source_ordinal": site.ordinal,
                "alias": site.alias,
                "declared_type": site.declared_type,
                "source_file": site.span.source_file,
                "source_line": site.span.source_line or site.span.start_line,
            }
        )

    return ViewModel(
        name="object",
        nodes=nodes,
        edges=edges,
        metadata={"declaration_sites": declaration_inventory},
    )


def build_boundary_view(model: ObjectModel) -> ViewModel:
    """Build the structured deferred/trimmed inventory view."""

    nodes: dict[str, ViewNode] = {}
    edges: list[ViewEdge] = []
    inventory: list[dict[str, object]] = []
    for boundary_id, boundary in sorted(model.boundaries.items()):
        owner_id = f"owner::{boundary.owner}"
        nodes.setdefault(
            owner_id,
            ViewNode(id=owner_id, label=boundary.owner, kind="boundary_owner"),
        )
        nodes[boundary_id] = ViewNode(
            id=boundary_id,
            label=f"{boundary_id} [{boundary.category}]\n{boundary.summary}",
            kind=boundary.status,
        )
        edges.append(
            ViewEdge(
                source=owner_id,
                target=boundary_id,
                kind=boundary.status,
                label="owns",
            )
        )
        span = boundary.decl.span
        inventory.append(
            {
                "id": boundary_id,
                "status": boundary.status,
                "category": boundary.category,
                "summary": boundary.summary,
                "resolution": boundary.resolution,
                "owner": boundary.owner,
                "source_file": span.source_file,
                "source_line": span.source_line or span.start_line,
            }
        )
    return ViewModel(
        name="boundaries",
        nodes=nodes,
        edges=edges,
        rankdir="LR",
        metadata={
            "inventory": inventory,
            "summary": {
                "deferred": model.deferred_count,
                "trimmed": model.trimmed_count,
                "legacy_boundaries": model.legacy_boundary_count,
            },
        },
    )


def build_drives_view(model: ObjectModel) -> ViewModel:
    """Build an transition-level view from drives blocks."""

    nodes: dict[str, ViewNode] = {}
    edges: list[ViewEdge] = []

    for external in model.externals.values():
        source = f"external::{external.name}"
        nodes[source] = ViewNode(id=source, label=external.name, kind="External")
        for delivery, blocks in (
            ("drives", external.decl.drives),
            ("emits", external.decl.emits),
        ):
            for block in blocks:
                for entry in block.entries:
                    call = parse_emit_expression(entry)
                    if (
                        call is None
                        or call.receiver not in model.objects
                        or call.kind != "Transition"
                    ):
                        continue
                    target = _transition_node_id(call.receiver, call.name)
                    _add_transition_node(nodes, target, call.receiver, call.name)
                    edges.append(
                        ViewEdge(
                            source=source,
                            target=target,
                            kind=delivery,
                        )
                    )

    for obj in model.objects.values():
        for state in obj.states.values():
            for transition in state.transitions.values():
                source = _transition_node_id(obj.name, transition.name)
                _add_transition_node(nodes, source, obj.name, transition.name)

                for target_obj, target_transition in _driven_transitions(model, transition):
                    if target_obj not in model.objects:
                        continue
                    target = _transition_node_id(target_obj, target_transition)
                    _add_transition_node(nodes, target, target_obj, target_transition)
                    edges.append(ViewEdge(source=source, target=target, kind="drives"))

                for target_obj, target_transition in _emitted_transitions(model, transition):
                    if target_obj not in model.objects:
                        continue
                    target = _transition_node_id(target_obj, target_transition)
                    _add_transition_node(nodes, target, target_obj, target_transition)
                    edges.append(ViewEdge(source=source, target=target, kind="emits"))

                for target_obj, target_action in _driven_actions(transition):
                    if (
                        target_obj not in model.objects
                        and target_obj not in _CONTEXTUAL_ACTION_RECEIVERS
                    ):
                        continue
                    target = _action_node_id(target_obj, target_action)
                    _add_action_node(nodes, target, target_obj, target_action)
                    edges.append(ViewEdge(source=source, target=target, kind="drives_action"))

    return ViewModel(name="drives", nodes=nodes, edges=edges, rankdir="LR")


def build_timeline_view(model: ObjectModel) -> ViewModel:
    """Build a timeline view from timeline and phase object transitions."""

    nodes: dict[str, ViewNode] = {}
    edges: list[ViewEdge] = []
    phase_objects = {
        name
        for name, obj in model.objects.items()
        if _is_timeline_object(name, obj.kind)
    }
    phase_parents = {
        name: model.objects[name].parent for name in phase_objects if name in model.objects
    }

    for name in phase_objects:
        obj = model.objects[name]
        nodes[name] = ViewNode(id=name, label=name, kind=obj.kind)

    for name in phase_objects:
        obj = model.objects[name]
        for state in obj.states.values():
            for transition in state.transitions.values():
                event_id = _transition_node_id(name, transition.name)
                _add_transition_node(nodes, event_id, name, transition.name)
                edges.append(
                    ViewEdge(
                        source=name,
                        target=event_id,
                        kind="has_transition",
                    )
                )

                for target_obj, target_transition in _driven_transitions(model, transition):
                    if target_obj not in phase_objects:
                        continue
                    target = _transition_node_id(target_obj, target_transition)
                    _add_transition_node(nodes, target, target_obj, target_transition)
                    edges.append(
                        ViewEdge(
                            source=event_id,
                            target=target,
                            kind="drives",
                        )
                    )
                for target_obj, target_transition in _emitted_transitions(model, transition):
                    if target_obj not in phase_objects:
                        continue
                    target = _transition_node_id(target_obj, target_transition)
                    _add_transition_node(nodes, target, target_obj, target_transition)
                    edges.append(
                        ViewEdge(
                            source=event_id,
                            target=target,
                            kind="emits",
                        )
                    )

    return ViewModel(
        name="timeline",
        nodes=nodes,
        edges=edges,
        rankdir="BT",
        graph_format="svg",
        metadata={
            "timeline_rows": _build_timeline_rows(model, phase_objects),
            "phase_parents": phase_parents,
        },
    )


def build_trace_view(
    derive_data: dict[str, Any],
    *,
    max_action_depth: int | None = DEFAULT_TRACE_ACTION_DEPTH,
) -> ViewModel:
    """Build a trace layout view from derive JSON."""

    builder = _TraceLayoutBuilder()
    roots = derive_data.get("trace", [])
    builder.build(
        roots,
        _verified_states_by_transition(derive_data, roots),
        _context_records_by_transition(derive_data),
        _ordinary_action_records_by_transition(derive_data),
        _transition_record_order_by_transition(derive_data),
        max_action_depth=max_action_depth,
    )
    return ViewModel(
        name="trace",
        graph_format="text",
        metadata={
            "trace_columns": builder.columns,
            "trace_rows": builder.rows,
            "trace_cells": tuple(builder.cells),
            "trace_arrows": tuple(builder.arrows),
            "runtime_instances": derive_data.get("runtime_instances", []),
            "runtime_instance_count": len(
                derive_data.get("runtime_instances", [])
                if isinstance(derive_data.get("runtime_instances"), list)
                else []
            ),
        },
    )


def _declaration_site_node_id(owner_process: str, ordinal: int, alias: str) -> str:
    return f"declaration::{owner_process}::s{ordinal}::{alias}"


def _ordinary_action_records_by_transition(
    derive_data: dict[str, Any]
) -> dict[tuple[str, str], list[dict[str, object]]]:
    actions: dict[tuple[str, str], list[dict[str, object]]] = {}
    records = derive_data.get("records", [])
    if not isinstance(records, list):
        return actions

    for order, record in enumerate(records):
        if not isinstance(record, dict):
            continue
        if record.get("status") != "proved":
            continue
        object_name = record.get("object")
        transition_name = record.get("transition")
        if not isinstance(object_name, str) or not isinstance(transition_name, str):
            continue
        proof_class = record.get("proof_class")
        if proof_class not in {
            "action_commit",
            "action_result_binding",
            "type_process_commit",
        }:
            continue
        if record.get("source_kind") != "drives":
            continue
        if record.get("proof_provider") == "within_context":
            continue
        expression = record.get("expression")
        if not isinstance(expression, str) or not expression:
            continue
        display_expression = record.get("display_expression")
        if not isinstance(display_expression, str) or not display_expression:
            display_expression = expression
        actions.setdefault((object_name, transition_name), []).append(
            {
                "action": display_expression,
                "expression": expression,
                "order": order,
            }
        )
    return actions


def _transition_record_order_by_transition(derive_data: dict[str, Any]) -> dict[tuple[str, str], int]:
    orders: dict[tuple[str, str], int] = {}
    records = derive_data.get("records", [])
    if not isinstance(records, list):
        return orders

    for order, record in enumerate(records):
        if not isinstance(record, dict):
            continue
        if record.get("status") != "proved":
            continue
        message = record.get("message")
        if not isinstance(message, str) or not message.startswith("transition: "):
            continue
        object_name = record.get("object")
        transition_name = record.get("transition")
        if not isinstance(object_name, str) or not isinstance(transition_name, str):
            continue
        orders.setdefault((object_name, transition_name), order)
    return orders


def _context_records_by_transition(
    derive_data: dict[str, Any]
) -> dict[tuple[str, str], list[dict[str, object]]]:
    contexts: dict[tuple[str, str], list[dict[str, object]]] = {}
    active_context: dict[tuple[str, str], list[dict[str, object]]] = {}
    context_specs = _context_specs(derive_data)
    records = derive_data.get("records", [])
    if not isinstance(records, list):
        return contexts

    def add_context_marker(
        key: tuple[str, str],
        order: int,
        entry: dict[str, object],
        *,
        marker: str,
    ) -> None:
        context_name = _active_context_name(entry)
        if context_name is None:
            return
        item: dict[str, object] = {
            "context": context_name,
            "context_stack": tuple(
                name
                for name in (
                    _active_context_name(active)
                    for active in active_context.get(key, ())
                )
                if name is not None
            ),
            "order": order,
            "marker": marker,
        }
        process_parent = entry.get("process_parent")
        if isinstance(process_parent, str) and process_parent:
            item["process_parent"] = process_parent
            item["context_process_parent"] = process_parent
        context_labels: dict[str, str] = {}
        for active_name in _context_item_stack(item):
            active_spec = context_specs.get(active_name)
            if active_spec is not None:
                context_labels[active_name] = _context_trace_label(
                    active_name, active_spec
                )
        if context_labels:
            item["context_labels"] = context_labels
        context_spec = context_specs.get(context_name)
        if context_spec is not None:
            item["context_label"] = _context_trace_label(context_name, context_spec)
        contexts.setdefault(key, []).append(item)

    for order, record in enumerate(records):
        if not isinstance(record, dict):
            continue
        object_name = record.get("object")
        transition_name = record.get("transition")
        if not isinstance(object_name, str) or not isinstance(transition_name, str):
            continue
        key = (object_name, transition_name)
        source_kind = record.get("source_kind")
        proof_class = record.get("proof_class")
        proof_provider = record.get("proof_provider")
        expression = record.get("expression")
        if (
            source_kind == "within"
            and proof_class == "exclusive_context"
            and isinstance(expression, str)
            and expression.startswith("within ")
            and not expression.endswith(" exited")
        ):
            entry = {
                "name": expression.removeprefix("within ").strip(),
                "process_parent": record.get("process_parent"),
            }
            active_context.setdefault(key, []).append(entry)
            add_context_marker(key, order, entry, marker="enter")
            continue
        if (
            source_kind == "within"
            and proof_class == "exclusive_context"
            and isinstance(expression, str)
            and expression.endswith(" exited")
        ):
            stack = active_context.get(key)
            if stack:
                entry = stack[-1]
                add_context_marker(key, order, entry, marker="exit")
                stack.pop()
            if stack == []:
                active_context.pop(key, None)
            continue
        if (
            source_kind == "within ensures"
            and proof_class == "exclusive_context_fact"
            and proof_provider == "within_ensures"
            and isinstance(expression, str)
        ):
            stack = active_context.get(key)
            context_entry = stack[-1] if stack else None
            context_name = _active_context_name(context_entry)
            if context_name is None:
                continue
            display_expression = record.get("display_expression")
            if not isinstance(display_expression, str) or not display_expression:
                display_expression = " ".join(expression.split())
            context_stack = tuple(
                name
                for name in (_active_context_name(entry) for entry in stack)
                if name is not None
            )
            item: dict[str, object] = {
                "context": context_name,
                "context_stack": context_stack,
                "expression": expression,
                "fact": display_expression,
                "order": order,
            }
            context_parent = _active_context_process_parent(context_entry)
            if isinstance(context_parent, str) and context_parent:
                item["context_process_parent"] = context_parent
            context_labels: dict[str, str] = {}
            for active_name in context_stack:
                active_spec = context_specs.get(active_name)
                if active_spec is not None:
                    context_labels[active_name] = _context_trace_label(
                        active_name, active_spec
                    )
            if context_labels:
                item["context_labels"] = context_labels
            context_spec = context_specs.get(context_name)
            if context_spec is not None:
                item["context_label"] = _context_trace_label(
                    context_name, context_spec
                )
            contexts.setdefault(key, []).append(item)
            continue
        if (
            proof_class
            not in {"action_commit", "action_result_binding", "type_process_commit"}
            or proof_provider != "within_context"
        ):
            continue
        stack = active_context.get(key)
        context_entry = stack[-1] if stack else None
        context_name = _active_context_name(context_entry)
        if context_name is None or not isinstance(expression, str):
            continue
        display_expression = record.get("display_expression")
        if not isinstance(display_expression, str) or not display_expression:
            display_expression = expression
        context_stack = tuple(
            name
            for name in (_active_context_name(entry) for entry in stack)
            if name is not None
        )
        item: dict[str, object] = {
            "context": context_name,
            "context_stack": context_stack,
            "expression": expression,
            "action": display_expression,
            "order": order,
        }
        process_parent = record.get("process_parent")
        if isinstance(process_parent, str) and process_parent:
            item["process_parent"] = process_parent
        context_parent = _active_context_process_parent(context_entry)
        if isinstance(context_parent, str) and context_parent:
            item["context_process_parent"] = context_parent
        context_labels: dict[str, str] = {}
        for active_name in context_stack:
            active_spec = context_specs.get(active_name)
            if active_spec is not None:
                context_labels[active_name] = _context_trace_label(
                    active_name, active_spec
                )
        if context_labels:
            item["context_labels"] = context_labels
        context_spec = context_specs.get(context_name)
        if context_spec is not None:
            item["context_label"] = _context_trace_label(context_name, context_spec)
        contexts.setdefault(key, []).append(item)
    return contexts


def _active_context_name(entry: object) -> str | None:
    if isinstance(entry, str):
        return entry
    if isinstance(entry, dict):
        name = entry.get("name")
        if isinstance(name, str) and name:
            return name
    return None


def _active_context_process_parent(entry: object) -> str | None:
    if not isinstance(entry, dict):
        return None
    process_parent = entry.get("process_parent")
    return process_parent if isinstance(process_parent, str) and process_parent else None


def _context_specs(derive_data: dict[str, Any]) -> dict[str, dict[str, object]]:
    model = derive_data.get("model")
    if not isinstance(model, dict):
        return {}
    contexts = model.get("exclusive_contexts")
    if not isinstance(contexts, dict):
        return {}
    return {
        name: spec
        for name, spec in contexts.items()
        if isinstance(name, str) and isinstance(spec, dict)
    }


def _context_trace_label(context_name: str, context_spec: dict[str, object]) -> str:
    labels = [context_name]
    guard = context_spec.get("guard")
    lock_ref = context_spec.get("lock_ref")
    if isinstance(guard, dict):
        guard_kind = guard.get("kind")
        if isinstance(guard_kind, str) and guard_kind:
            labels.append(f"guard={guard_kind}")
        guard_lock_ref = guard.get("lock_ref")
        if isinstance(guard_lock_ref, str) and guard_lock_ref:
            lock_ref = guard_lock_ref
        enter = _first_block_body(guard.get("entered_by"))
        exit_ = _first_block_body(guard.get("exited_by"))
        if enter:
            labels.append(f"enter={enter}")
        if exit_:
            labels.append(f"exit={exit_}")
    if isinstance(lock_ref, str) and lock_ref:
        labels.insert(1, f"lock={lock_ref}")
    return "|".join(labels)


def _first_block_body(value: object) -> str:
    if not isinstance(value, list):
        return ""
    for item in value:
        if not isinstance(item, dict):
            continue
        body = item.get("body")
        if isinstance(body, str) and body.strip():
            return body.strip().rstrip(";")
    return ""


def _context_item_stack(item: dict[str, object]) -> list[str]:
    stack = item.get("context_stack")
    if isinstance(stack, (list, tuple)):
        names = [name for name in stack if isinstance(name, str) and name]
        if names:
            return names
    context_name = item.get("context")
    return [context_name] if isinstance(context_name, str) and context_name else []


def _context_item_label(item: dict[str, object], context_name: str) -> str:
    labels = item.get("context_labels")
    if isinstance(labels, dict):
        label = labels.get(context_name)
        if isinstance(label, str) and label:
            return label
    if item.get("context") == context_name:
        label = item.get("context_label")
        if isinstance(label, str) and label:
            return label
    return context_name


def _build_context_forest(
    items: list[dict[str, object]],
    max_action_depth: int | None,
) -> list[dict[str, object]]:
    action_depths = _context_action_depths(items)
    forest: list[dict[str, object]] = []
    open_nodes: list[dict[str, object]] = []

    for item in items:
        stack = _context_item_stack(item)
        if not stack:
            continue
        marker = item.get("marker")

        common = 0
        max_common = min(len(stack), len(open_nodes))
        while (
            common < max_common
            and open_nodes[common].get("name") == stack[common]
        ):
            common += 1
        open_nodes = open_nodes[:common]

        for context_name in stack[common:]:
            node: dict[str, object] = {
                "kind": "context",
                "name": context_name,
                "label": _context_item_label(item, context_name),
                "children": [],
            }
            order = item.get("order")
            if isinstance(order, int):
                node["start_order"] = order
            context_parent = item.get("context_process_parent")
            if isinstance(context_parent, str) and context_parent:
                node["process_parent"] = context_parent
                node["parent_keys"] = _process_identity_keys(context_parent)
            if open_nodes:
                children = open_nodes[-1].setdefault("children", [])
                if isinstance(children, list):
                    children.append(node)
            else:
                forest.append(node)
            open_nodes.append(node)

        if marker == "exit":
            order = item.get("order")
            if open_nodes and isinstance(order, int):
                open_nodes[-1]["end_order"] = order
            open_nodes = open_nodes[:-1]
            continue

        children = open_nodes[-1].setdefault("children", [])
        action_text = str(item.get("action", ""))
        if isinstance(children, list) and action_text:
            children.append(
                {
                    "kind": "action",
                    "action": action_text,
                    "keys": _process_identity_keys(action_text, item.get("expression")),
                    "parent": item.get("process_parent"),
                    "parent_keys": _process_identity_keys(item.get("process_parent")),
                    "children": [],
                }
            )
            continue
        fact_text = str(item.get("fact", ""))
        if isinstance(children, list) and fact_text:
            children.append(
                {
                    "kind": "fact",
                    "fact": fact_text,
                    "keys": _process_identity_keys(fact_text, item.get("expression")),
                    "children": [],
                }
            )

    for context in forest:
        _nest_context_action_children(context)

    if max_action_depth is not None:
        pruned_forest: list[dict[str, object]] = []
        for context in forest:
            pruned = _prune_context_tree_by_action_depth(
                context,
                action_depths,
                max_action_depth,
            )
            if pruned is not None:
                pruned_forest.append(pruned)
        forest = pruned_forest

    return forest


def _context_action_depths(
    items: list[dict[str, object]]
) -> dict[str, int]:
    actions: list[dict[str, object]] = []
    action_index_by_key: dict[str, int] = {}

    for item in items:
        action = item.get("action")
        if not isinstance(action, str) or not action:
            continue
        keys = _process_identity_keys(action, item.get("expression"))
        if not keys:
            continue
        action_index = len(actions)
        actions.append(
            {
                "keys": keys,
                "parent_keys": _process_identity_keys(item.get("process_parent")),
            }
        )
        for key in keys:
            action_index_by_key.setdefault(key, action_index)

    depth_by_index: dict[int, int] = {}
    resolving: set[int] = set()

    def resolve_depth(action_index: int) -> int:
        if action_index in depth_by_index:
            return depth_by_index[action_index]
        if action_index in resolving:
            return 0
        resolving.add(action_index)
        parent_depths: list[int] = []
        parent_keys = actions[action_index].get("parent_keys")
        if isinstance(parent_keys, set):
            for parent_key in parent_keys:
                parent_index = action_index_by_key.get(parent_key)
                if parent_index is not None and parent_index != action_index:
                    parent_depths.append(resolve_depth(parent_index) + 1)
        resolving.remove(action_index)
        depth = min(parent_depths) if parent_depths else 0
        depth_by_index[action_index] = depth
        return depth

    depth_by_key: dict[str, int] = {}
    for action_index, action in enumerate(actions):
        depth = resolve_depth(action_index)
        keys = action.get("keys")
        if isinstance(keys, set):
            for key in keys:
                depth_by_key.setdefault(key, depth)
    return depth_by_key


def _prune_context_tree_by_action_depth(
    node: dict[str, object],
    action_depths: dict[str, int],
    max_action_depth: int,
) -> dict[str, object] | None:
    parent_depth = _context_action_depth_for_keys(
        node.get("parent_keys"),
        action_depths,
    )
    if parent_depth is not None and parent_depth >= max_action_depth:
        return None

    children = node.get("children")
    if not isinstance(children, list):
        return node

    pruned_children: list[dict[str, object]] = []
    for child in children:
        if not isinstance(child, dict):
            continue
        if child.get("kind") == "context":
            pruned_child = _prune_context_tree_by_action_depth(
                child,
                action_depths,
                max_action_depth,
            )
            if pruned_child is not None:
                pruned_children.append(pruned_child)
            continue
        if child.get("kind") == "action":
            action_depth = _context_action_depth_for_keys(
                child.get("keys"),
                action_depths,
            )
            if action_depth is not None and action_depth > max_action_depth:
                continue
        pruned_children.append(child)
    node["children"] = pruned_children
    return node


def _context_action_depth_for_keys(
    keys: object,
    action_depths: dict[str, int],
) -> int | None:
    if not isinstance(keys, set):
        return None
    depths = [action_depths[key] for key in keys if key in action_depths]
    return min(depths) if depths else None


def _nest_context_action_children(node: dict[str, object]) -> None:
    children = node.get("children")
    if not isinstance(children, list):
        return

    for child in children:
        if isinstance(child, dict) and child.get("kind") == "context":
            _nest_context_action_children(child)

    actions = [
        child
        for child in children
        if isinstance(child, dict) and child.get("kind") == "action"
    ]
    action_by_key: dict[str, dict[str, object]] = {}
    for action in actions:
        keys = action.get("keys")
        if isinstance(keys, set):
            for key in keys:
                action_by_key.setdefault(key, action)
        label = str(action.get("action", ""))
        if label:
            for key in _process_identity_keys(label):
                action_by_key.setdefault(key, action)
    nested_ids: set[int] = set()
    for action in actions:
        parent_keys = action.get("parent_keys")
        if not isinstance(parent_keys, set) or not parent_keys:
            continue
        parent_action = None
        for parent_key in parent_keys:
            parent_action = action_by_key.get(parent_key)
            if parent_action is not None:
                break
        if parent_action is None or parent_action is action:
            continue
        parent_children = parent_action.setdefault("children", [])
        if isinstance(parent_children, list):
            parent_children.append(action)
            nested_ids.add(id(action))

    if nested_ids:
        node["children"] = [
            child
            for child in children
            if not (
                isinstance(child, dict)
                and child.get("kind") == "action"
                and id(child) in nested_ids
            )
        ]


def _context_action_max_depth(
    node: dict[str, object], max_action_depth: int | None
) -> int:
    children = node.get("children")
    if not isinstance(children, list):
        return 0

    max_depth = _context_span_start_depth(node)

    for child in children:
        if not isinstance(child, dict):
            continue
        if child.get("kind") == "context":
            max_depth = max(max_depth, _context_action_max_depth(child, max_action_depth))
        elif child.get("kind") == "action":
            max_depth = max(
                max_depth,
                _context_action_tree_max_depth(
                    child,
                    _context_span_start_depth(node),
                    max_action_depth,
                ),
            )
    return max_depth


def _context_action_tree_max_depth(
    action_node: dict[str, object],
    action_depth: int,
    max_action_depth: int | None,
) -> int:
    max_depth = action_depth
    if max_action_depth is not None and action_depth >= max_action_depth:
        return max_depth
    children = action_node.get("children")
    if not isinstance(children, list):
        return max_depth

    for child in children:
        if isinstance(child, dict) and child.get("kind") == "action":
            max_depth = max(
                max_depth,
                _context_action_tree_max_depth(
                    child,
                    action_depth + 1,
                    max_action_depth,
                ),
            )
    return max_depth


def _context_span_start_depth(node: dict[str, object]) -> int:
    parent_keys = node.get("parent_keys")
    return 1 if isinstance(parent_keys, set) and parent_keys else 0


def _context_span_column_count(
    node: dict[str, object], max_action_depth: int | None
) -> int:
    start_depth = _context_span_start_depth(node)
    max_depth = _context_action_max_depth(node, max_action_depth)
    return 2 + max(0, max_depth - start_depth) * 2


def _context_span_row(
    start_row: int,
    end_row: int,
    event_body_start: int,
    *,
    context_parent_row: int | None,
) -> int:
    if context_parent_row is None:
        return start_row
    height = max(1, end_row - start_row)
    lower_padding = height // 2
    return max(event_body_start, context_parent_row - lower_padding)


def _process_identity_keys(*values: object) -> set[str]:
    keys: set[str] = set()
    for value in values:
        if not isinstance(value, str):
            continue
        text = " ".join(value.split()).strip()
        if not text:
            continue
        keys.add(text)
        compact_call = _process_call_identity(text)
        if compact_call:
            keys.add(compact_call)
    return keys


def _process_call_identity(value: str) -> str:
    match = re.match(
        r"\A([A-Za-z_][A-Za-z0-9_.]*\.(?:Transition|Action)::[A-Za-z_][A-Za-z0-9_]*)\((.*)\)\Z",
        value,
    )
    if match is None:
        return ""
    prefix, args = match.group(1, 2)
    normalized_args: list[str] = []
    for arg in _split_process_args(args):
        if ":" in arg:
            _name, arg = arg.split(":", 1)
        normalized_args.append(arg.strip())
    return f"{prefix}({', '.join(normalized_args)})"


def _split_process_args(args: str) -> list[str]:
    parts: list[str] = []
    start = 0
    depth = 0
    for index, char in enumerate(args):
        if char in "([{":
            depth += 1
        elif char in ")]}" and depth > 0:
            depth -= 1
        elif char == "," and depth == 0:
            parts.append(args[start:index].strip())
            start = index + 1
    tail = args[start:].strip()
    if tail:
        parts.append(tail)
    return parts


def _transition_node_id(object_name: str, transition_name: str) -> str:
    return f"{object_name}.{transition_name}"


def _action_node_id(object_name: str, action_name: str) -> str:
    return f"{object_name}.Action.{action_name}"


def _add_transition_node(
    nodes: dict[str, ViewNode], node_id: str, object_name: str, transition_name: str
) -> None:
    if node_id not in nodes:
        nodes[node_id] = ViewNode(
            id=node_id,
            label=f"{object_name}.{transition_name}",
            kind="Transition",
        )


def _add_action_node(
    nodes: dict[str, ViewNode], node_id: str, object_name: str, action_name: str
) -> None:
    if node_id not in nodes:
        nodes[node_id] = ViewNode(
            id=node_id,
            label=f"{object_name}.{action_name}",
            kind="Action",
        )


def _build_timeline_rows(
    model: ObjectModel, phase_objects: set[str]
) -> tuple[TimelineRow, ...]:
    states = {
        name: obj.initial_state
        for name, obj in model.objects.items()
        if obj.initial_state is not None
    }
    sequence: list[tuple[str, str, str, str, str]] = []
    rows_by_phase_state: dict[tuple[str, str], list[TimelineItem]] = {}
    row_order: list[tuple[str, str]] = []
    processed_events: set[tuple[str, str]] = set()

    def ensure_row(phase: str, state: str) -> None:
        row_key = (phase, state)
        if phase in _LIFECYCLE_ROOT_OBJECTS or row_key in rows_by_phase_state:
            return
        rows_by_phase_state[row_key] = []
        row_order.append(row_key)

    def process_transition(
        object_name: str,
        transition_name: str,
        current_phase: str,
        current_phase_state: str | None,
    ) -> None:
        obj = model.objects.get(object_name)
        if obj is None:
            return

        transition = _find_transition(obj, transition_name, states.get(object_name))
        if transition is None:
            return

        key = (object_name, transition_name)
        if key in processed_events:
            return
        processed_events.add(key)

        if object_name in phase_objects:
            next_phase = object_name
            next_phase_state = transition.target_state
        else:
            next_phase = current_phase
            next_phase_state = current_phase_state

        for target_obj, target_transition in _driven_transitions(model, transition):
            process_transition(target_obj, target_transition, next_phase, next_phase_state)

        states[object_name] = transition.target_state

        if object_name in phase_objects:
            ensure_row(object_name, transition.target_state)
        elif current_phase_state is not None:
            sequence.append(
                (
                    current_phase,
                    current_phase_state,
                    object_name,
                    transition.target_state,
                    transition.name,
                )
            )

        for target_obj, target_transition in _emitted_transitions(model, transition):
            process_transition(target_obj, target_transition, next_phase, next_phase_state)

    if len(model.externals) == 1:
        external = next(iter(model.externals.values()))
        for blocks in (external.decl.drives, external.decl.emits):
            for block in blocks:
                for entry in block.entries:
                    call = parse_emit_expression(entry)
                    if (
                        call is not None
                        and call.kind == "Transition"
                        and call.receiver in model.objects
                    ):
                        process_transition(
                            call.receiver,
                            call.name,
                            call.receiver,
                            None,
                        )
    else:
        root = _default_transition_target()
        if root is not None:
            process_transition(root[0], root[1], root[0], None)

    final_by_object: dict[str, tuple[str, str, str, str]] = {}
    for phase, phase_state, object_name, target_state, transition_name in sequence:
        final_by_object[object_name] = (phase, phase_state, target_state, transition_name)

    placed_objects: set[str] = set()
    for phase, phase_state, object_name, _target_state, _transition_name in sequence:
        final = final_by_object[object_name]
        if (phase, phase_state, _target_state, _transition_name) != final:
            continue
        if object_name in placed_objects:
            continue

        row_key = (phase, phase_state)
        if row_key not in rows_by_phase_state:
            rows_by_phase_state[row_key] = []
            row_order.append(row_key)
        rows_by_phase_state[row_key].append(
            TimelineItem(
                object_name=object_name,
                detail=f"State::{final[2]}",
                kind="state",
            )
        )
        placed_objects.add(object_name)

    for phase, phase_state in row_order:
        for object_name, state_name in _phase_state_object_refs(model, phase, phase_state):
            if object_name in phase_objects or object_name in placed_objects:
                continue
            rows_by_phase_state[(phase, phase_state)].append(
                TimelineItem(
                    object_name=object_name,
                    detail=f"State::{state_name}",
                    kind="state",
                )
            )
            placed_objects.add(object_name)

    return tuple(
        _make_timeline_row(index, phase, state, rows_by_phase_state[(phase, state)])
        for index, (phase, state) in enumerate(row_order)
    )


def _phase_state_object_refs(
    model: ObjectModel, phase: str, phase_state: str
) -> list[tuple[str, str]]:
    obj = model.objects.get(phase)
    if obj is None:
        return []
    state = obj.states.get(phase_state)
    if state is None:
        return []

    refs: list[tuple[str, str]] = []
    seen: set[tuple[str, str]] = set()
    for block in state.decl.invariants:
        for object_name, state_name in _OBJECT_STATE_RE.findall(block.body):
            if state_name != phase_state:
                continue
            key = (object_name, state_name)
            if key in seen:
                continue
            seen.add(key)
            refs.append(key)
    return refs


def _make_timeline_row(
    index: int, phase: str, state: str, items: list[TimelineItem]
) -> TimelineRow:
    parent_phase = _parent_timeline_phase(phase)
    subphase = phase if parent_phase != phase else None
    return TimelineRow(
        id=f"{phase}.{state}.{index}",
        phase=parent_phase,
        subphase=subphase,
        label=state.lower(),
        detail=f"State::{state}",
        items=tuple(items),
    )


def _parent_timeline_phase(phase: str) -> str:
    if phase in {
        "EntrySuccessorPhase",
        "CorePreparePhase",
        "MmCoreInitPhase",
    }:
        return "BootPhase"
    return phase


def _find_transition(obj, transition_name: str, current_state: str | None) -> TransitionDef | None:
    if current_state is not None:
        state = obj.states.get(current_state)
        if state is not None and transition_name in state.transitions:
            return state.transitions[transition_name]

    for state in obj.states.values():
        transition = state.transitions.get(transition_name)
        if transition is not None:
            return transition
    return None


def _driven_transitions(
    model: ObjectModel, transition: TransitionDef
) -> list[tuple[str, str]]:
    return _driven_transitions_from_body_members(
        model, transition, _ordered_body_members(transition.decl)
    )


def _driven_transitions_from_body_members(
    model: ObjectModel, transition: TransitionDef, members
) -> list[tuple[str, str]]:
    driven: list[tuple[str, str]] = []
    for member in members:
        if member.block is not None and member.kind == "drives":
            driven.extend(_OBJECT_TRANSITION_RE.findall(member.block.body))
            driven.extend(
                _resolved_association_transitions(model, transition, member.block.body)
            )
            continue
        if member.within is not None:
            driven.extend(
                _driven_transitions_from_body_members(
                    model, transition, _ordered_body_members(member.within)
                )
            )
    return driven


def _emitted_transitions(
    model: ObjectModel, transition: TransitionDef
) -> list[tuple[str, str]]:
    emitted: list[tuple[str, str]] = []
    for block in transition.decl.emits:
        for entry, _span in block.entry_spans:
            expression = entry.strip()
            match = _LOCAL_TRANSITION_EXPR_RE.match(expression)
            if match is not None:
                emitted.append((transition.object_name, match.group(1)))
                continue
            match = _OBJECT_TRANSITION_EXPR_RE.match(expression)
            if match is not None:
                emitted.append((match.group(1), match.group(2)))
                continue
            emitted.extend(
                _resolved_association_transitions(model, transition, expression)
            )
    return emitted


_ASSOCIATION_TRANSITION_RE = re.compile(
    r"\b(self(?:\.[a-z_][A-Za-z0-9_]*)+)\.Transition::([A-Za-z_][A-Za-z0-9_]*)\b"
)


def _resolved_association_transitions(
    model: ObjectModel, transition: TransitionDef, text: str
) -> list[tuple[str, str]]:
    resolved: list[tuple[str, str]] = []
    for receiver, transition_name in _ASSOCIATION_TRANSITION_RE.findall(text):
        target = transition.object_name
        for member in receiver.split(".")[1:]:
            obj = model.objects.get(target)
            if obj is None:
                target = ""
                break
            target = obj.associations.get(member, "")
            if not target:
                break
        if target in model.objects:
            resolved.append((target, transition_name))
    return resolved


def _driven_actions(transition: TransitionDef) -> list[tuple[str, str]]:
    return _driven_actions_from_body_members(_ordered_body_members(transition.decl))


def _driven_actions_from_body_members(members) -> list[tuple[str, str]]:
    driven: list[tuple[str, str]] = []
    for member in members:
        if member.block is not None and member.kind == "drives":
            driven.extend(_OBJECT_ACTION_RE.findall(member.block.body))
            continue
        if member.within is not None:
            driven.extend(
                _driven_actions_from_body_members(_ordered_body_members(member.within))
            )
    return driven


def _ordered_body_members(decl) -> list[BodyMember]:
    if decl.body_members:
        return list(decl.body_members)
    members: list[BodyMember] = []
    for block in getattr(decl, "depends_on", []):
        members.append(_block_body_member(block))
    for block in getattr(decl, "drives", []):
        members.append(_block_body_member(block))
    for block in getattr(decl, "emits", []):
        members.append(_block_body_member(block))
    for within in getattr(decl, "within", []):
        members.append(_within_body_member(within))
    for block in getattr(decl, "exited_by", []):
        members.append(_block_body_member(block))
    for block in getattr(decl, "may_change", []):
        members.append(_block_body_member(block))
    for block in getattr(decl, "ensures", []):
        members.append(_block_body_member(block))
    for block in getattr(decl, "deferred", []):
        members.append(_block_body_member(block))
    for block in getattr(decl, "other_blocks", []):
        members.append(_block_body_member(block))
    return members


def _block_body_member(block: Block) -> BodyMember:
    return BodyMember(kind=block.kind, span=block.span, block=block)


def _within_body_member(within: WithinDecl) -> BodyMember:
    return BodyMember(kind="within", span=within.span, within=within)


class _TraceLayoutBuilder:
    def __init__(self) -> None:
        self.columns: list[dict[str, object]] = []
        self.rows: list[dict[str, object]] = []
        self.cells: list[TraceCell] = []
        self.arrows: list[TraceArrow] = []
        self._event_index = 0
        self._max_phase_lane = 0
        self._max_object_lane = -1
        self._object_column_base = 0

    def build(
        self,
        roots: list[Any],
        verified_states_by_event: dict[tuple[str, str], list[tuple[str, str]]],
        context_records: dict[tuple[str, str], list[dict[str, object]]],
        ordinary_actions: dict[tuple[str, str], list[dict[str, object]]],
        event_orders: dict[tuple[str, str], int],
        *,
        max_action_depth: int | None,
    ) -> None:
        self._max_phase_lane = _max_trace_phase_lane(
            roots,
            verified_states_by_event,
            ordinary_actions,
            context_records,
        )
        self._object_column_base = self._max_phase_lane + 2
        pending: list[dict[str, object]] = [
            {
                "node": node,
                "phase_lane": 0,
                "object_lane": 0,
                "parent_event_id": None,
            }
            for node in roots
        ]
        while pending:
            continuation = pending.pop(0)
            node = continuation["node"]
            if _should_skip_trace_node(
                node,
                verified_states_by_event,
                ordinary_actions,
                context_records,
            ):
                continue
            emitted_continuations = self._place_node(
                node,
                phase_lane=int(continuation["phase_lane"]),
                object_lane=int(continuation["object_lane"]),
                parent_event_id=continuation.get("parent_event_id")
                if isinstance(continuation.get("parent_event_id"), str)
                else None,
                context_records=context_records,
                ordinary_actions=ordinary_actions,
                event_orders=event_orders,
                verified_states_by_event=verified_states_by_event,
                max_action_depth=max_action_depth,
            )
            pending[0:0] = emitted_continuations
        self._center_multi_target_process_sources()
        self._build_columns()

    def _place_node(
        self,
        node: Any,
        *,
        phase_lane: int,
        object_lane: int,
        parent_event_id: str | None,
        context_records: dict[tuple[str, str], list[dict[str, object]]],
        ordinary_actions: dict[tuple[str, str], list[dict[str, object]]],
        event_orders: dict[tuple[str, str], int],
        verified_states_by_event: dict[tuple[str, str], list[tuple[str, str]]],
        max_action_depth: int | None,
    ) -> list[dict[str, object]]:
        data = _trace_node_object(node)
        is_phase = _is_trace_phase_object(str(data["object"]))
        index = self._event_index
        self._event_index += 1
        label = _trace_label(data)
        event_id = f"transition-{index}"
        enter_id = f"{event_id}-source"
        exit_id = f"{event_id}-target"
        span_id = f"{event_id}-span"
        column = phase_lane if is_phase else self._object_column(object_lane)
        gap_column = None if is_phase else self._object_gap_column(object_lane)
        self._max_phase_lane = max(self._max_phase_lane, phase_lane)
        if not is_phase:
            self._max_object_lane = max(self._max_object_lane, object_lane)

        source_label = f"{data['object']}.State::{data['source_state']}"
        event_row = len(self.rows)
        adjacent_source_id = self._adjacent_state_cell_id(column, source_label)
        if adjacent_source_id is not None:
            enter_id = adjacent_source_id
            event_row = self.rows[-1]["index"]
        else:
            self._add_row(
                "state",
                event_row,
                f"{label}.source",
                group_id=event_id if not is_phase else None,
                group_role="source" if not is_phase else None,
            )
            self.cells.append(
                TraceCell(
                    id=enter_id,
                    kind="state",
                    row=event_row,
                    column=column,
                    label=source_label,
                )
            )
            self._add_gap_cell(
                TraceCell(
                    id=f"{event_id}-vertical-gap-before",
                    kind="gap",
                    row=event_row,
                    column=gap_column if gap_column is not None else column,
                )
            )

        event_body_start = len(self.rows)
        self._add_row(
            "gap",
            event_body_start,
            f"{label}.body.start",
            group_id=event_id if not is_phase else None,
            group_role="body_start" if not is_phase else None,
        )
        self.cells.append(
            TraceCell(
                id=f"{event_id}-body-gap",
                kind="gap",
                row=event_body_start,
                column=column,
            )
        )
        self._add_gap_cell(
            TraceCell(
                id=f"{event_id}-drive-gap",
                kind="gap",
                row=event_body_start,
                column=gap_column if gap_column is not None else column,
            )
        )

        verified_states = verified_states_by_event.get(
            (str(data["object"]), str(data["transition"])), []
        )
        verified_lane = object_lane if is_phase else object_lane + 1
        child_object_lane = object_lane if is_phase else object_lane + (
            2 if verified_states else 1
        )
        verified_column = self._object_column(verified_lane)
        verified_gap_column = self._object_gap_column(verified_lane)
        if verified_states:
            self._max_object_lane = max(self._max_object_lane, verified_lane)
        for verified_index, (object_name, state_name) in enumerate(verified_states):
            verified_id = f"{event_id}-verified-{verified_index}"
            verified_row = len(self.rows)
            self._add_row("state", verified_row, f"{label}.verified.{object_name}")
            self.cells.append(
                TraceCell(
                    id=verified_id,
                    kind="verified_state",
                    row=verified_row,
                    column=verified_column,
                    label=f"{object_name}.State::{state_name}",
                )
            )
            self.cells.append(
                TraceCell(
                    id=f"{verified_id}-gap",
                    kind="gap",
                    row=verified_row,
                    column=verified_gap_column,
                )
            )
            self.arrows.append(
                TraceArrow(source=span_id, target=verified_id, kind="depends_on")
            )

        body_items: list[dict[str, object]] = []
        for child_index, child in enumerate(_trace_children(data)):
            if _should_skip_trace_node(
                child,
                verified_states_by_event,
                ordinary_actions,
                context_records,
            ):
                continue
            child_data = _trace_node_object(child)
            child_key = (
                str(child_data.get("object")),
                str(child_data.get("transition")),
            )
            body_items.append(
                {
                    "kind": "child",
                    "order": event_orders.get(child_key, 1_000_000 + child_index),
                    "child": child,
                    "edge_kind": _trace_edge_kind(child_data),
                }
            )
        event_key = (str(data["object"]), str(data["transition"]))
        context_items = context_records.get(event_key, [])
        context_forest = _build_context_forest(context_items, max_action_depth)
        if context_forest:
            orders = [
                item.get("order")
                for item in context_items
                if isinstance(item.get("order"), int)
            ]
            body_items.append(
                {
                    "kind": "context",
                    "order": min(orders) if orders else 1_400_000,
                    "forest": context_forest,
                }
            )
        for action_index, action in enumerate(ordinary_actions.get(event_key, [])):
            order = action.get("order")
            body_items.append(
                {
                    "kind": "action",
                    "order": order if isinstance(order, int) else 1_500_000 + action_index,
                    "action": action,
                }
            )
        body_items.sort(key=lambda item: int(item.get("order", 0)))

        action_lane = child_object_lane
        action_column = self._object_column(action_lane)
        context_column = action_column
        max_context_action_depth = 0
        for item in body_items:
            if item.get("kind") != "context":
                continue
            forest = item.get("forest")
            if isinstance(forest, list):
                for context_node in forest:
                    if isinstance(context_node, dict):
                        max_context_action_depth = max(
                            max_context_action_depth,
                            _context_action_max_depth(context_node, max_action_depth),
                        )
        if any(item.get("kind") in {"action", "context"} for item in body_items):
            self._max_object_lane = max(
                self._max_object_lane,
                action_lane + max_context_action_depth,
            )

        context_index = 0
        previous_context_action_id: str | None = None
        process_cell_by_key: dict[str, str] = {}
        process_row_by_key: dict[str, int] = {}
        body_item_ranges: list[dict[str, int]] = []
        pending_context_spans: list[dict[str, object]] = []
        emitted_continuations: list[dict[str, object]] = []
        emit_items: list[dict[str, object]] = []
        emit_index = 0

        def register_body_item_range(
            order: object, start_row: int, end_row: int
        ) -> None:
            if not isinstance(order, int) or end_row <= start_row:
                return
            for content_range in content_ranges_for_rows(start_row, end_row):
                content_range["order"] = order
                body_item_ranges.append(content_range)

        def content_ranges_for_rows(start_row: int, end_row: int) -> list[dict[str, int]]:
            ranges: list[dict[str, int]] = []
            for cell in self.cells:
                if cell.kind == "gap":
                    continue
                cell_start = cell.row
                cell_end = cell.row + cell.row_span
                if cell_end <= start_row or cell_start >= end_row:
                    continue
                ranges.append(
                    {
                        "start_row": max(start_row, cell_start),
                        "end_row": min(end_row, cell_end),
                        "start_column": cell.column,
                        "end_column": cell.column + cell.column_span,
                    }
                )
            return ranges

        def register_process_cell(cell_id: str, row: int, *values: object) -> None:
            for key in _process_identity_keys(*values):
                process_cell_by_key.setdefault(key, cell_id)
                process_row_by_key.setdefault(key, row)

        def pad_phase_body_to_min_rows(reserved_rows: int = 0) -> None:
            if not is_phase:
                return
            final_body_rows = len(self.rows) - event_body_start + reserved_rows + 1
            padding_rows = max(0, _TRACE_PHASE_MIN_BODY_ROWS - final_body_rows)
            for padding_index in range(padding_rows):
                padding_row = len(self.rows)
                self._add_row(
                    "phase_padding",
                    padding_row,
                    f"{label}.phase_padding.{padding_index}",
                )

        def external_process_cell(keys: object) -> str | None:
            if not isinstance(keys, set):
                return None
            for key in keys:
                cell_id = process_cell_by_key.get(key)
                if cell_id is not None:
                    return cell_id
            return None

        def external_process_row(keys: object) -> int | None:
            if not isinstance(keys, set):
                return None
            for key in keys:
                row = process_row_by_key.get(key)
                if row is not None:
                    return row
            return None

        def child_lanes(
            child_data: dict[str, object], edge_kind: str
        ) -> tuple[str, bool, int, int]:
            child_object = str(child_data.get("object"))
            child_is_phase = _is_trace_phase_object(child_object)
            same_object_emit = (
                edge_kind == "emits" and child_object == str(data.get("object"))
            )
            next_phase_lane = (
                phase_lane
                if same_object_emit or not child_is_phase
                else phase_lane + 1
            )
            next_object_lane = (
                object_lane
                if same_object_emit or child_is_phase
                else child_object_lane
            )
            return child_object, child_is_phase, next_phase_lane, next_object_lane

        def place_emit_item(item: dict[str, object]) -> None:
            nonlocal emit_index
            child = item.get("child")
            if child is None:
                return
            child_data = _trace_node_object(child)
            _child_object, child_is_phase, next_phase_lane, next_object_lane = child_lanes(
                child_data, "emits"
            )
            emit_row = len(self.rows)
            self._add_row(
                "emit",
                emit_row,
                f"{label}.emits.{emit_index}",
                group_id=event_id if not is_phase else None,
                group_role="emit" if not is_phase else None,
            )
            emit_id = f"{event_id}-emit-{emit_index}"
            emit_column = (
                next_phase_lane
                if child_is_phase
                else self._object_column(next_object_lane)
            )
            if child_is_phase:
                self._max_phase_lane = max(self._max_phase_lane, next_phase_lane)
            else:
                self._max_object_lane = max(self._max_object_lane, next_object_lane)
            self.cells.append(
                TraceCell(
                    id=emit_id,
                    kind="emit_event",
                    row=emit_row,
                    column=emit_column,
                    label=_trace_label(child_data),
                )
            )
            if not child_is_phase:
                self._add_gap_cell(
                    TraceCell(
                        id=f"{emit_id}-gap",
                        kind="gap",
                        row=emit_row,
                        column=self._object_gap_column(next_object_lane),
                    )
                )
            self.arrows.append(TraceArrow(source=span_id, target=emit_id, kind="emits"))
            emitted_continuations.append(
                {
                    "node": child,
                    "phase_lane": next_phase_lane,
                    "object_lane": next_object_lane,
                    "parent_event_id": None,
                }
            )
            emit_index += 1

        def place_context_node(
            node: dict[str, object], depth: int
        ) -> list[dict[str, int]]:
            nonlocal context_index, previous_context_action_id
            current_context_index = context_index
            context_index += 1
            context_id = f"{event_id}-context-{current_context_index}"
            context_parent_row = external_process_row(node.get("parent_keys"))
            action_index = 0
            context_label = str(node.get("label", node.get("name", "")))
            context_start_depth = _context_span_start_depth(node)
            context_span_columns = _context_span_column_count(node, max_action_depth)
            children = node.get("children")
            if not isinstance(children, list):
                children = []
            context_ranges: list[dict[str, int]] = []
            context_column_start = max(0, context_column + context_start_depth * 2)
            context_column_end = context_column_start + context_span_columns

            def add_context_range(row: int) -> None:
                context_ranges.append(
                    {
                        "start_row": row,
                        "end_row": row + 1,
                        "start_column": context_column_start,
                        "end_column": context_column_end,
                    }
                )

            if context_parent_row is not None:
                add_context_range(context_parent_row)

            if children:
                padding_row = len(self.rows)
                self._add_row(
                    "context_padding",
                    padding_row,
                    f"{label}.within.{current_context_index}.padding.bottom",
                    group_id=event_id,
                    group_role="context_padding",
                )
                add_context_range(padding_row)

            def place_context_action(
                action_node: dict[str, object],
                *,
                action_depth: int,
                parent_action_id: str | None,
            ) -> str:
                nonlocal action_index, previous_context_action_id
                external_parent_id = (
                    external_process_cell(action_node.get("parent_keys"))
                    if parent_action_id is None
                    else None
                )
                effective_action_depth = (
                    1 if external_parent_id is not None else action_depth
                )
                external_parent_row = (
                    external_process_row(action_node.get("parent_keys"))
                    if external_parent_id is not None
                    else None
                )
                if external_parent_row is not None and action_index == 0:
                    action_row = external_parent_row
                else:
                    action_row = len(self.rows)
                    self._add_row(
                        "context_action",
                        action_row,
                        f"{label}.within.{current_context_index}.{action_index}",
                        group_id=event_id,
                        group_role="context_action",
                    )
                action_id = f"{context_id}-action-{action_index}"
                self.cells.append(
                    TraceCell(
                        id=action_id,
                        kind="context_action",
                        row=action_row,
                        column=context_column + effective_action_depth * 2,
                        label=str(action_node.get("action", "")),
                        column_span=2,
                    )
                )
                context_ranges.append(
                    {
                        "start_row": action_row,
                        "end_row": action_row + 1,
                        "start_column": context_column + effective_action_depth * 2,
                        "end_column": context_column + effective_action_depth * 2 + 2,
                    }
                )
                register_process_cell(
                    action_id,
                    action_row,
                    action_node.get("action"),
                    *tuple(action_node.get("keys") or ()),
                )
                if parent_action_id is not None:
                    self.arrows.append(
                        TraceArrow(
                            source=parent_action_id,
                            target=action_id,
                            kind="drives",
                        )
                    )
                elif external_parent_id is not None:
                    self.arrows.append(
                        TraceArrow(
                            source=external_parent_id,
                            target=action_id,
                            kind="drives",
                        )
                    )
                elif previous_context_action_id is not None:
                    self.arrows.append(
                        TraceArrow(
                            source=previous_context_action_id,
                            target=action_id,
                            kind="context_order",
                        )
                    )
                if parent_action_id is None:
                    previous_context_action_id = action_id
                action_index += 1

                nested_children = action_node.get("children")
                if (
                    isinstance(nested_children, list)
                    and (
                        max_action_depth is None
                        or effective_action_depth < max_action_depth
                    )
                ):
                    for nested in nested_children:
                        if isinstance(nested, dict) and nested.get("kind") == "action":
                            place_context_action(
                                nested,
                                action_depth=effective_action_depth + 1,
                                parent_action_id=action_id,
                            )
                return action_id

            def place_context_fact(fact_node: dict[str, object]) -> None:
                nonlocal action_index
                fact_row = len(self.rows)
                self._add_row(
                    "context_fact",
                    fact_row,
                    f"{label}.within.{current_context_index}.fact.{action_index}",
                    group_id=event_id,
                    group_role="context_action",
                )
                fact_id = f"{context_id}-fact-{action_index}"
                self.cells.append(
                    TraceCell(
                        id=fact_id,
                        kind="context_fact",
                        row=fact_row,
                        column=context_column,
                        label=str(fact_node.get("fact", "")),
                        column_span=2,
                    )
                )
                context_ranges.append(
                    {
                        "start_row": fact_row,
                        "end_row": fact_row + 1,
                        "start_column": context_column,
                        "end_column": context_column + 2,
                    }
                )
                action_index += 1

            for child in children:
                if not isinstance(child, dict):
                    continue
                if child.get("kind") == "context":
                    context_ranges.extend(place_context_node(child, depth + 1))
                    continue
                if child.get("kind") == "action":
                    place_context_action(child, action_depth=0, parent_action_id=None)
                    continue
                if child.get("kind") == "fact":
                    place_context_fact(child)

            if "|" in context_label:
                guard_row = len(self.rows)
                self._add_row(
                    "context_guard",
                    guard_row,
                    f"{label}.within.{current_context_index}.guard",
                    group_id=event_id,
                    group_role="context_guard",
                )
                add_context_range(guard_row)
            if children:
                padding_row = len(self.rows)
                self._add_row(
                    "context_padding",
                    padding_row,
                    f"{label}.within.{current_context_index}.padding.top",
                    group_id=event_id,
                    group_role="context_padding",
                )
                add_context_range(padding_row)

            pending_context_spans.append(
                {
                    "id": context_id,
                    "label": context_label,
                    "ranges": context_ranges,
                    "start_order": node.get("start_order"),
                    "end_order": node.get("end_order"),
                    "source_id": external_process_cell(node.get("parent_keys"))
                    or span_id,
                }
            )
            return context_ranges

        def finalize_context_spans() -> None:
            for pending in pending_context_spans:
                ranges = list(pending.get("ranges") or ())
                start_order = pending.get("start_order")
                end_order = pending.get("end_order")
                if isinstance(start_order, int) and isinstance(end_order, int):
                    for item_range in body_item_ranges:
                        order = item_range["order"]
                        if start_order < order < end_order:
                            ranges.append(item_range)
                if not ranges:
                    continue
                span_row = max(
                    event_body_start,
                    min(item["start_row"] for item in ranges),
                )
                span_end = max(item["end_row"] for item in ranges)
                if span_end <= span_row:
                    continue
                span_column = min(item["start_column"] for item in ranges)
                span_column_end = max(item["end_column"] for item in ranges)
                context_id = str(pending["id"])
                self.cells.append(
                    TraceCell(
                        id=context_id,
                        kind="context_span",
                        row=span_row,
                        column=span_column,
                        label=str(pending["label"]),
                        row_span=span_end - span_row,
                        column_span=max(1, span_column_end - span_column),
                    )
                )
                self.arrows.append(
                    TraceArrow(
                        source=str(pending["source_id"]),
                        target=context_id,
                        kind="within",
                    )
                )

        for item in body_items:
            if item.get("kind") == "context":
                forest = item.get("forest")
                if not isinstance(forest, list):
                    continue
                for context_node in forest:
                    if isinstance(context_node, dict) and context_node.get("kind") == "context":
                        place_context_node(context_node, 0)
                continue

            if item.get("kind") == "action":
                action = item.get("action")
                if not isinstance(action, dict):
                    continue
                action_row = len(self.rows)
                self._add_row(
                    "action",
                    action_row,
                    f"{label}.action.{action_row}",
                    group_id=event_id if not is_phase else None,
                    group_role="action" if not is_phase else None,
                )
                action_id = f"{event_id}-action-{action_row}"
                self.cells.append(
                    TraceCell(
                        id=action_id,
                        kind="action",
                        row=action_row,
                        column=action_column,
                        label=str(action.get("action", "")),
                    )
                )
                register_process_cell(
                    action_id,
                    action_row,
                    action.get("action"),
                    action.get("expression"),
                )
                self.arrows.append(
                    TraceArrow(source=span_id, target=action_id, kind="action")
                )
                register_body_item_range(item.get("order"), action_row, action_row + 1)
                continue

            child = item.get("child")
            if child is None:
                continue
            child_data = _trace_node_object(child)
            edge_kind = str(item.get("edge_kind") or "drives")
            if edge_kind == "emits":
                emit_items.append(item)
                continue
            _child_object, _child_is_phase, next_phase_lane, next_object_lane = child_lanes(
                child_data, edge_kind
            )

            child_event_id = f"transition-{self._event_index}"
            self.arrows.append(
                TraceArrow(
                    source=span_id,
                    target=f"{child_event_id}-span",
                    kind=edge_kind,
                )
            )
            child_start_row = len(self.rows)
            child_continuations = self._place_node(
                child,
                phase_lane=next_phase_lane,
                object_lane=next_object_lane,
                parent_event_id=event_id,
                context_records=context_records,
                ordinary_actions=ordinary_actions,
                event_orders=event_orders,
                verified_states_by_event=verified_states_by_event,
                max_action_depth=max_action_depth,
            )
            register_body_item_range(item.get("order"), child_start_row, len(self.rows))
            emitted_continuations.extend(child_continuations)

        finalize_context_spans()
        pad_phase_body_to_min_rows(reserved_rows=len(emit_items))
        for emit_item in emit_items:
            place_emit_item(emit_item)

        event_exit_gap_row = len(self.rows)
        self._add_row(
            "gap",
            event_exit_gap_row,
            f"{label}.body.end",
            group_id=event_id if not is_phase else None,
            group_role="body_end" if not is_phase else None,
        )
        self.cells.append(
            TraceCell(
                id=f"{event_id}-body-end-gap",
                kind="gap",
                row=event_exit_gap_row,
                column=column,
            )
        )
        self._add_gap_cell(
            TraceCell(
                id=f"{event_id}-drive-end-gap",
                kind="gap",
                row=event_exit_gap_row,
                column=gap_column if gap_column is not None else column,
            )
        )

        exit_row = len(self.rows)
        self._add_row(
            "state",
            exit_row,
            f"{label}.target",
            group_id=event_id if not is_phase else None,
            group_role="target" if not is_phase else None,
        )
        self.cells.append(
            TraceCell(
                id=exit_id,
                kind="state",
                row=exit_row,
                column=column,
                label=f"{data['object']}.State::{data['target_state']}",
            )
        )
        self._add_gap_cell(
            TraceCell(
                id=f"{event_id}-vertical-gap-after",
                kind="gap",
                row=exit_row,
                column=gap_column if gap_column is not None else column,
            )
        )

        self.cells.append(
            TraceCell(
                id=span_id,
                kind="transition_span",
                row=event_body_start,
                column=column,
                label=label,
                row_span=event_exit_gap_row - event_body_start + 1,
            )
        )
        self.arrows.append(TraceArrow(source=enter_id, target=exit_id, kind="state"))
        if parent_event_id is not None:
            self._add_gap_cell(
                TraceCell(
                    id=f"{parent_event_id}-to-{event_id}-gap",
                    kind="gap",
                    row=event_row,
                    column=gap_column if gap_column is not None else column,
                )
            )
        return emitted_continuations

    def _add_row(
        self,
        kind: str,
        index: int,
        label: str,
        *,
        group_id: str | None = None,
        group_role: str | None = None,
    ) -> None:
        row: dict[str, object] = {"index": index, "kind": kind, "label": label}
        if group_id is not None:
            row["group_id"] = group_id
        if group_role is not None:
            row["group_role"] = group_role
        self.rows.append(row)

    def _add_gap_cell(self, cell: TraceCell) -> None:
        if cell.column < self._object_column_base:
            return
        self.cells.append(cell)

    def _adjacent_state_cell_id(self, column: int, label: str) -> str | None:
        if not self.rows or self.rows[-1]["kind"] != "state":
            return None
        row = self.rows[-1]["index"]
        for cell in reversed(self.cells):
            if (
                cell.kind == "state"
                and cell.row == row
                and cell.column == column
                and cell.label == label
            ):
                return cell.id
        return None

    def _build_columns(self) -> None:
        for phase_lane in range(self._max_phase_lane + 1):
            self.columns.append(
                {
                    "index": phase_lane,
                    "kind": "phase",
                    "depth": phase_lane,
                }
            )
        self.columns.append(
            {
                "index": self._object_column_base - 1,
                "kind": "phase_object_gap",
                "depth": 0,
            }
        )
        for object_lane in range(self._max_object_lane + 1):
            self.columns.append(
                {
                    "index": self._object_column(object_lane),
                    "kind": "object",
                    "depth": object_lane,
                }
            )
            self.columns.append(
                {
                    "index": self._object_gap_column(object_lane),
                    "kind": "gap",
                    "depth": object_lane,
                }
            )

    def _object_column(self, object_lane: int) -> int:
        return self._object_column_base + object_lane * 2

    def _object_gap_column(self, object_lane: int) -> int:
        return self._object_column(object_lane) + 1

    def _center_multi_target_process_sources(self) -> None:
        while True:
            cell_by_id = {cell.id: cell for cell in self.cells}
            target_ranges_by_source: dict[str, list[tuple[int, int]]] = {}
            for arrow in self.arrows:
                if arrow.kind != "drives":
                    continue
                source = cell_by_id.get(arrow.source)
                target = cell_by_id.get(arrow.target)
                if source is None or target is None:
                    continue
                if source.kind not in {"action", "context_action"}:
                    continue
                if target.kind != "context_action" or target.column <= source.column:
                    continue
                target_ranges_by_source.setdefault(source.id, []).append(
                    (target.row, target.row + target.row_span)
                )

            replacements: dict[str, TraceCell] = {}
            for source_id, target_ranges in target_ranges_by_source.items():
                if len(target_ranges) < 2:
                    continue
                source = cell_by_id[source_id]
                start_row = min(start for start, _end in target_ranges)
                end_row = max(end for _start, end in target_ranges)
                row_span = max(1, end_row - start_row)
                if source.row == start_row and source.row_span == row_span:
                    continue
                replacements[source_id] = replace(
                    source,
                    row=start_row,
                    row_span=row_span,
                )

            if not replacements:
                return
            self.cells = [replacements.get(cell.id, cell) for cell in self.cells]


def _trace_node_object(node: Any) -> dict[str, Any]:
    if not isinstance(node, dict):
        raise ValueError("trace node must be an object")
    return node


def _trace_children(node: dict[str, Any]) -> list[Any]:
    children = node.get("children", [])
    return children if isinstance(children, list) else []


def _trace_label(node: dict[str, Any]) -> str:
    label = node.get("label")
    if isinstance(label, str):
        return label
    return f"{node.get('object')}.Transition::{node.get('transition')}"


def _trace_edge_kind(node: dict[str, Any]) -> str:
    edge_kind = node.get("edge_kind")
    if edge_kind in {"drives", "emits"}:
        return str(edge_kind)
    return "drives"


def _should_skip_trace_node(
    node: Any,
    verified_states_by_event: dict[tuple[str, str], list[tuple[str, str]]],
    ordinary_actions: dict[tuple[str, str], list[dict[str, object]]] | None = None,
    context_records: dict[tuple[str, str], list[dict[str, object]]] | None = None,
) -> bool:
    data = _trace_node_object(node)
    object_name = data.get("object")
    transition_name = data.get("transition")
    if not isinstance(object_name, str) or not isinstance(transition_name, str):
        return False
    if not _is_trace_phase_object(object_name):
        return False
    if _trace_children(data):
        return False
    key = (object_name, transition_name)
    if verified_states_by_event.get(key):
        return False
    if ordinary_actions is not None and ordinary_actions.get(key):
        return False
    if context_records is not None and context_records.get(key):
        return False
    return True


def _is_trace_phase_object(object_name: str) -> bool:
    return object_name in _LIFECYCLE_ROOT_OBJECTS or object_name.endswith("Phase")


def _is_timeline_object(object_name: str, kind: str) -> bool:
    return object_name in _LIFECYCLE_ROOT_OBJECTS or kind == "PhaseObject"


def _default_transition_target() -> tuple[str, str] | None:
    match = _TARGET_RE.match(DEFAULT_TARGET)
    if match is None:
        return None
    return match.group(1), match.group(2)


def _max_trace_phase_lane(
    roots: list[Any],
    verified_states_by_event: dict[tuple[str, str], list[tuple[str, str]]],
    ordinary_actions: dict[tuple[str, str], list[dict[str, object]]],
    context_records: dict[tuple[str, str], list[dict[str, object]]],
) -> int:
    max_lane = 0

    def visit(node: Any, phase_lane: int) -> None:
        nonlocal max_lane
        if _should_skip_trace_node(
            node,
            verified_states_by_event,
            ordinary_actions,
            context_records,
        ):
            return
        data = _trace_node_object(node)
        is_phase = _is_trace_phase_object(str(data.get("object")))
        current_phase_lane = phase_lane
        if is_phase:
            max_lane = max(max_lane, current_phase_lane)
        for child in _trace_children(data):
            child_data = _trace_node_object(child)
            child_phase_lane = (
                current_phase_lane + 1
                if _is_trace_phase_object(str(child_data.get("object")))
                else current_phase_lane
            )
            visit(child, child_phase_lane)

    for root in roots:
        visit(root, 0)
    return max_lane


def _verified_states_by_transition(
    derive_data: dict[str, Any], roots: list[Any]
) -> dict[tuple[str, str], list[tuple[str, str]]]:
    transitioned_objects = _trace_transitioned_objects(roots)
    seen_states: set[tuple[str, str]] = set()
    verified: dict[tuple[str, str], list[tuple[str, str]]] = {}
    for record in derive_data.get("records", []):
        if not isinstance(record, dict):
            continue
        if record.get("status") != "proved":
            continue
        message = record.get("message")
        if not isinstance(message, str) or not message.startswith("depends_on: "):
            continue
        expression = record.get("expression")
        if not isinstance(expression, str):
            continue
        match = _OBJECT_STATE_RE.search(expression)
        if match is None:
            continue

        object_name, state_name = match.group(1), match.group(2)
        if object_name in transitioned_objects:
            continue
        state_key = (object_name, state_name)
        if state_key in seen_states:
            continue
        event_object = record.get("object")
        transition_name = record.get("transition")
        if not isinstance(event_object, str) or not isinstance(transition_name, str):
            continue

        seen_states.add(state_key)
        verified.setdefault((event_object, transition_name), []).append(state_key)
    return verified


def _trace_transitioned_objects(roots: list[Any]) -> set[str]:
    objects: set[str] = set()

    def visit(node: Any) -> None:
        if not isinstance(node, dict):
            return
        object_name = node.get("object")
        if isinstance(object_name, str):
            objects.add(object_name)
        for child in _trace_children(node):
            visit(child)

    for root in roots:
        visit(root)
    return objects
