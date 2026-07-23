"""Normalize the complete tools2 AST into a stable executable Signal model."""

from __future__ import annotations

from copy import deepcopy
import json
import re
from typing import Any

from tools2_common import fingerprint


_CALL_RE = re.compile(
    r"^(?:(?P<receiver>[A-Za-z_][A-Za-z0-9_.-]*)\.)?"
    r"(?P<kind>Transition|Action)::(?P<name>[A-Za-z_][A-Za-z0-9_-]*)"
    r"(?:\((?P<args>.*)\))?$",
    re.S,
)
_STATE_RE = re.compile(
    r"^(?P<target>[A-Za-z_][A-Za-z0-9_.-]*)\.state\s*==\s*State::(?P<state>[A-Za-z_][A-Za-z0-9_-]*)$"
)
_ASSIGN_STATE_RE = re.compile(
    r"^(?P<target>[A-Za-z_][A-Za-z0-9_.-]*)\.state\s*=\s*State::(?P<state>[A-Za-z_][A-Za-z0-9_-]*)$"
)
_FACT_RE = re.compile(r"^(?P<name>[A-Za-z_][A-Za-z0-9_:.-]*)\((?P<args>.*)\)$", re.S)
_PATH_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_.-]*$")


def _diagnostic(
    diagnostics: list[dict[str, Any]], category: str, message: str, span: dict[str, Any]
) -> None:
    diagnostics.append({"category": category, "message": message, "span": span})


def _split_top(text: str, separator: str = ",") -> list[str]:
    result: list[str] = []
    depth = 0
    start = 0
    in_string = False
    for index, char in enumerate(text):
        if in_string:
            if char == '"' and (index == 0 or text[index - 1] != "\\"):
                in_string = False
            continue
        if char == '"':
            in_string = True
        elif char in "(<{":
            depth += 1
        elif char in ")>}":
            depth = max(0, depth - 1)
        elif char == separator and depth == 0:
            result.append(text[start:index].strip())
            start = index + 1
    tail = text[start:].strip()
    if tail:
        result.append(tail)
    return result


def _split_top_operator(text: str, operator: str) -> list[str]:
    result: list[str] = []
    depth = 0
    start = 0
    index = 0
    in_string = False
    while index < len(text):
        char = text[index]
        if in_string:
            if char == '"' and (index == 0 or text[index - 1] != "\\"):
                in_string = False
            index += 1
            continue
        if char == '"':
            in_string = True
        elif char in "(<{":
            depth += 1
        elif char in ")>}":
            depth = max(0, depth - 1)
        elif depth == 0 and text.startswith(operator, index):
            result.append(text[start:index].strip())
            index += len(operator)
            start = index
            continue
        index += 1
    tail = text[start:].strip()
    if tail:
        result.append(tail)
    return result


def _named_separator(text: str) -> int | None:
    depth = 0
    for index, char in enumerate(text):
        if char in "(<{":
            depth += 1
        elif char in ")>}":
            depth = max(0, depth - 1)
        elif char == ":" and depth == 0:
            before = text[index - 1] if index else ""
            after = text[index + 1] if index + 1 < len(text) else ""
            if before != ":" and after != ":":
                return index
    return None


def _value(text: str) -> dict[str, Any]:
    value = text.strip()
    if value.startswith('"') and value.endswith('"'):
        try:
            return {"kind": "string", "value": json.loads(value)}
        except json.JSONDecodeError:
            return {"kind": "expression", "value": value}
    if re.fullmatch(r"-?[0-9]+", value):
        return {"kind": "integer", "value": int(value)}
    if value in {"true", "false"}:
        return {"kind": "boolean", "value": value == "true"}
    enum = re.fullmatch(r"([A-Za-z_][A-Za-z0-9_]*)::([A-Za-z_][A-Za-z0-9_]*)", value)
    if enum:
        return {"kind": "enum", "type": enum.group(1), "value": enum.group(2)}
    if _PATH_RE.fullmatch(value):
        return {"kind": "path", "value": value}
    return {"kind": "expression", "value": value}


def _expression(entry: dict[str, Any], *, assignment: bool = False) -> dict[str, Any]:
    text = entry["text"].strip()
    alternatives = _split_top_operator(text, "||") if not assignment else [text]
    if len(alternatives) > 1:
        return {
            "kind": "any_of",
            "alternatives": [
                _expression({"text": alternative, "span": entry["span"]})
                for alternative in alternatives
            ],
            "text": text,
            "span": entry["span"],
        }
    state = (_ASSIGN_STATE_RE if assignment else _STATE_RE).fullmatch(text)
    if state:
        result = {
            "kind": "state_assignment" if assignment else "state_condition",
            "target": state.group("target"),
            "state": state.group("state"),
        }
    else:
        operator = "=" if assignment else "=="
        positions: list[int] = []
        depth = 0
        for index, char in enumerate(text):
            if char in "(<{":
                depth += 1
            elif char in ")>}":
                depth = max(0, depth - 1)
            elif char == "=" and depth == 0:
                if assignment and (index == 0 or text[index - 1] not in "=!<>") and (
                    index + 1 >= len(text) or text[index + 1] != "="
                ):
                    positions.append(index)
                elif not assignment and index and text[index - 1] == "=":
                    positions.append(index - 1)
        if positions:
            position = positions[0]
            width = len(operator)
            result = {
                "kind": "reference_assignment" if assignment else "reference_condition",
                "reference": text[:position].strip(),
                "value": _value(text[position + width :]),
            }
        else:
            fact = _FACT_RE.fullmatch(text)
            if fact:
                raw_args = fact.group("args").strip()
                result = {
                    "kind": "fact",
                    "name": fact.group("name"),
                    "arguments": [_value(item) for item in _split_top(raw_args)] if raw_args else [],
                }
            elif re.fullmatch(r"[A-Za-z_][A-Za-z0-9_.:-]*", text):
                result = {"kind": "fact", "name": text, "arguments": []}
            else:
                result = {"kind": "assertion", "expression": text}
    result["text"] = text
    result["span"] = entry["span"]
    return result


def _call(entry: dict[str, Any]) -> dict[str, Any]:
    text = entry["text"].strip()
    alternatives = _split_top_operator(text, "||")
    if len(alternatives) > 1:
        choices = [
            _call({"text": alternative, "span": entry["span"]})
            for alternative in alternatives
        ]
        if all(choice.get("kind") == "call" for choice in choices):
            return {
                "kind": "choice",
                "choices": choices,
                "span": entry["span"],
                "text": entry["text"],
            }
        return {
            "kind": "invalid_call",
            "text": entry["text"],
            "span": entry["span"],
        }
    if text.startswith("lossy "):
        return {
            "kind": "invalid_call",
            "text": entry["text"],
            "span": entry["span"],
        }
    declaration = re.fullmatch(r"declare\s+([a-z][A-Za-z0-9_]*)\s+of\s+(.+)", text, re.S)
    if declaration:
        return {
            "kind": "declare",
            "alias": declaration.group(1),
            "declared_type": declaration.group(2).strip(),
            "span": entry["span"],
            "text": entry["text"],
        }
    binding = re.fullmatch(
        r"let\s+([a-z][A-Za-z0-9_]*)\s*:\s*(.+?)\s*<-\s*(.+)", text, re.S
    )
    alias = None
    result_type = None
    if binding:
        alias, result_type, text = binding.group(1), binding.group(2).strip(), binding.group(3).strip()
    match = _CALL_RE.fullmatch(text)
    if match is None:
        return {
            "kind": "invalid_call",
            "text": entry["text"],
            "span": entry["span"],
        }
    arguments: list[dict[str, Any]] = []
    for raw in _split_top(match.group("args") or ""):
        separator = _named_separator(raw)
        if separator is None:
            arguments.append({"name": None, "value": _value(raw)})
        else:
            arguments.append(
                {"name": raw[:separator].strip(), "value": _value(raw[separator + 1 :])}
            )
    return {
        "kind": "call",
        "receiver": match.group("receiver") or "self",
        "process_kind": match.group("kind"),
        "name": match.group("name"),
        "arguments": arguments,
        "result_alias": alias,
        "result_type": result_type,
        "span": entry["span"],
        "text": entry["text"],
    }


def _normalize_members(members: list[dict[str, Any]]) -> list[dict[str, Any]]:
    result: list[dict[str, Any]] = []
    for member in members:
        kind = member["kind"]
        if kind in {"depends_on", "ensures", "may_change", "transitions"}:
            result.append(
                {
                    **member,
                    "entries": [_expression(entry) for entry in member.get("entries", [])],
                }
            )
        elif kind == "updates":
            result.append(
                {**member, "entries": [_expression(entry, assignment=True) for entry in member["entries"]]}
            )
        elif kind in {"drives", "emits"}:
            calls = [_call(entry) for entry in member["entries"]]
            for call in calls:
                if call.get("kind") == "call":
                    call["delivery"] = kind
            result.append({**member, "entries": calls})
        elif kind == "within":
            result.append({**member, "members": _normalize_members(member["members"])})
        elif kind in {"deferred", "trimmed"}:
            result.append(
                {**member, "evidence": [_expression(entry) for entry in member.get("evidence", [])]}
            )
        elif kind == "result":
            variants = []
            for variant in member.get("variants", []):
                variants.append(
                    {
                        **variant,
                        "members": _normalize_members(variant.get("members", [])),
                    }
                )
            result.append({**member, "variants": variants})
        else:
            result.append(deepcopy(member))
    return result


def _handler(handler: dict[str, Any], *, owner: str) -> dict[str, Any]:
    result = deepcopy(handler)
    result["owner"] = owner
    result["id"] = (
        f"{owner}.{handler['kind']}::{handler['name']}"
        + (f"@{handler['source_state']}" if handler.get("source_state") else "@process")
    )
    if "body" in handler and "members" not in handler:
        result["body"] = deepcopy(handler["body"])
    else:
        result["body"] = _normalize_members(handler.get("members", []))
    result.pop("members", None)
    return result


def _compose_type_lifecycle_process(
    handler: dict[str, Any], type_process: dict[str, Any] | None
) -> dict[str, Any]:
    """Attach reusable Type lifecycle behavior to an instance wrapper."""

    if type_process is None or handler.get("kind") != "Transition":
        return handler
    result = deepcopy(handler)
    result["body"] = [*deepcopy(type_process.get("body", [])), *result.get("body", [])]
    result["composed_type_processes"] = [
        {
            "id": type_process["id"],
            "owner": type_process["owner"],
            "span": deepcopy(type_process["span"]),
        }
    ]
    return result


def _index_types(
    raw_types: list[dict[str, Any]], diagnostics: list[dict[str, Any]]
) -> dict[str, dict[str, Any]]:
    types: dict[str, dict[str, Any]] = {}
    for declaration in raw_types:
        name = declaration["name"]
        if name in types:
            _diagnostic(diagnostics, "error", f"duplicate type {name}", declaration["span"])
            continue
        normalized = deepcopy(declaration)
        normalized["processes"] = [_handler(item, owner=name) for item in declaration["processes"]]
        normalized["states"] = {
            state["name"]: {
                **deepcopy(state),
                "invariant": [_expression(item) for item in state["invariant"]],
                "handlers": [_handler(item, owner=name) for item in state["handlers"]],
            }
            for state in declaration["states"]
        }
        normalized["invariant"] = [_expression(item) for item in declaration["invariant"]]
        types[name] = normalized
    return types


def _type_chain(types: dict[str, dict[str, Any]], name: str | None) -> list[dict[str, Any]]:
    result: list[dict[str, Any]] = []
    seen: set[str] = set()
    current = name
    while current and current not in seen and current in types:
        seen.add(current)
        declaration = types[current]
        result.append(declaration)
        current = declaration.get("base_type")
    return result


def _is_subtype(types: dict[str, dict[str, Any]], actual: str | None, expected: str) -> bool:
    return actual == expected or any(item["name"] == expected for item in _type_chain(types, actual))


def _check_task_only_lifecycle(
    *,
    owner: str,
    initial_state: str | None,
    states: list[dict[str, Any]],
    task_lifecycle: bool,
    diagnostics: list[dict[str, Any]],
    owner_span: dict[str, Any],
) -> None:
    if initial_state == "OnCpu" and not task_lifecycle:
        _diagnostic(
            diagnostics,
            "error",
            f"Task-only lifecycle state on non-Task: {owner}.initial_state State::OnCpu",
            owner_span,
        )
    for state in states:
        if state["name"] == "OnCpu" and not task_lifecycle:
            _diagnostic(
                diagnostics,
                "error",
                f"Task-only lifecycle state on non-Task: {owner}.State::OnCpu",
                state["span"],
            )
        for handler in state.get("handlers", []):
            if (
                handler.get("kind") == "Transition"
                and handler.get("name") in {"Continue", "Suspend"}
                and not task_lifecycle
            ):
                _diagnostic(
                    diagnostics,
                    "error",
                    "Task-only lifecycle transition on non-Task: "
                    f"{owner}.Transition::{handler['name']}",
                    handler["span"],
                )


def _type_fields(
    types: dict[str, dict[str, Any]], declared_type: str | None, field_kind: str
) -> list[dict[str, Any]]:
    fields: dict[str, dict[str, Any]] = {}
    for declaration in reversed(_type_chain(types, declared_type)):
        for field in declaration.get("fields", {}).get(field_kind, []):
            fields[field["name"]] = deepcopy(field)
    return list(fields.values())


def _system_fields(
    types: dict[str, dict[str, Any]], declaration: dict[str, Any]
) -> dict[str, list[dict[str, Any]]]:
    """Return inherited and object-local fields with stable override semantics."""

    result: dict[str, list[dict[str, Any]]] = {}
    declarative_kinds = {"attrs", "owned", "associations", "references", "slots"}
    field_kinds = ({
        kind
        for type_declaration in _type_chain(types, declaration.get("declared_type"))
        for kind in type_declaration.get("fields", {})
    } | set(declaration.get("fields", {}))) & declarative_kinds
    for kind in sorted(field_kinds):
        indexed = {
            item["name"]: item
            for item in _type_fields(types, declaration.get("declared_type"), kind)
        }
        for item in declaration.get("fields", {}).get(kind, []):
            inherited = deepcopy(indexed.get(item["name"], {}))
            for key, value in item.items():
                if value is not None:
                    inherited[key] = deepcopy(value)
            if item.get("type") is None and "mutable" in indexed.get(item["name"], {}):
                inherited["mutable"] = indexed[item["name"]]["mutable"]
            indexed[item["name"]] = inherited
        result[kind] = list(indexed.values())
    return result


def _nearest_lifecycle(
    types: dict[str, dict[str, Any]], declared_type: str | None
) -> dict[str, Any] | None:
    for declaration in _type_chain(types, declared_type):
        if declaration.get("initial_state") is not None or declaration.get("states"):
            return declaration
    return None


def _type_processes(
    types: dict[str, dict[str, Any]], declared_type: str | None
) -> list[dict[str, Any]]:
    indexed: dict[tuple[str, str], dict[str, Any]] = {}
    for declaration in reversed(_type_chain(types, declared_type)):
        for process in declaration.get("processes", []):
            indexed[(process["kind"], process["name"])] = deepcopy(process)
    return list(indexed.values())


def _parent_cycles(systems: dict[str, dict[str, Any]]) -> list[list[str]]:
    result: set[tuple[str, ...]] = set()
    for name in systems:
        path: list[str] = []
        positions: dict[str, int] = {}
        current: str | None = name
        while current is not None and current in systems:
            if current in positions:
                cycle = path[positions[current] :]
                rotations = [tuple(cycle[index:] + cycle[:index]) for index in range(len(cycle))]
                result.add(min(rotations))
                break
            positions[current] = len(path)
            path.append(current)
            current = systems[current].get("parent")
    return [list(item) for item in sorted(result)]


def build_model(document: dict[str, Any]) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    diagnostics = deepcopy(document.get("diagnostics", []))
    enums: dict[str, list[str]] = {}
    enum_spans: dict[str, dict[str, Any]] = {}
    for declaration in document.get("enums", []):
        name = declaration["name"]
        if name in enums:
            _diagnostic(diagnostics, "error", f"duplicate enum {name}", declaration["span"])
            continue
        enums[name] = declaration["values"]
        enum_spans[name] = declaration["span"]

    types = _index_types(document.get("types", []), diagnostics)
    for declaration in types.values():
        _check_task_only_lifecycle(
            owner=declaration["name"],
            initial_state=declaration.get("initial_state"),
            states=list(declaration.get("states", {}).values()),
            task_lifecycle=_is_subtype(types, declaration["name"], "Task"),
            diagnostics=diagnostics,
            owner_span=declaration["span"],
        )
    predicates: dict[str, list[dict[str, Any]]] = {}
    for declaration in document.get("predicates", []):
        predicates.setdefault(declaration["name"], []).append(deepcopy(declaration))

    contexts: dict[str, dict[str, Any]] = {}
    for declaration in document.get("contexts", []):
        if declaration["name"] in contexts:
            _diagnostic(
                diagnostics, "error", f"duplicate context {declaration['name']}", declaration["span"]
            )
        contexts[declaration["name"]] = deepcopy(declaration)

    raw_systems = document.get("systems", [])
    has_kernel = any(item["name"] == "Kernel" for item in raw_systems)
    systems: dict[str, dict[str, Any]] = {}
    for declaration in raw_systems:
        name = declaration["name"]
        if name in systems:
            _diagnostic(diagnostics, "error", f"duplicate system {name}", declaration["span"])
            continue
        declared_type = declaration.get("declared_type")
        lifecycle = _nearest_lifecycle(types, declared_type)
        declares_lifecycle = declaration.get("initial_state") is not None or bool(declaration.get("states"))
        override = declaration.get("properties", {}).get("lifecycle_override") == "true"
        _check_task_only_lifecycle(
            owner=name,
            initial_state=declaration.get("initial_state"),
            states=declaration.get("states", []),
            task_lifecycle=_is_subtype(types, declared_type, "Task"),
            diagnostics=diagnostics,
            owner_span=declaration["span"],
        )
        if lifecycle is not None and not declares_lifecycle:
            initial_state = lifecycle.get("initial_state")
            raw_states = list(lifecycle.get("states", {}).values())
            lifecycle_owner = lifecycle["name"]
        else:
            initial_state = declaration.get("initial_state")
            raw_states = declaration.get("states", [])
            lifecycle_owner = None
        if lifecycle is not None and declares_lifecycle and not override:
            # Existing standalone tools2 fixtures predate lifecycle_override and
            # may use ad-hoc system kinds; enforce this only for declared types.
            if declared_type in types:
                _diagnostic(
                    diagnostics,
                    "error",
                    f"object lifecycle shadows inherited type lifecycle: {name}",
                    declaration["span"],
                )
        states: dict[str, dict[str, Any]] = {}
        handlers_by_name: dict[str, list[dict[str, Any]]] = {}
        type_process_by_key = {
            (item["kind"], item["name"]): item
            for item in _type_processes(types, declared_type)
        }
        for raw_state in raw_states:
            state_name = raw_state["name"]
            if state_name in states:
                _diagnostic(
                    diagnostics, "error", f"duplicate state {name}.State::{state_name}", raw_state["span"]
                )
                continue
            handlers = []
            for raw_handler in raw_state.get("handlers", []):
                handler = _handler(raw_handler, owner=name)
                if not override:
                    handler = _compose_type_lifecycle_process(
                        handler,
                        type_process_by_key.get((handler["kind"], handler["name"])),
                    )
                handlers.append(handler)
            for item in handlers:
                item["lifecycle_owner"] = lifecycle_owner
                handlers_by_name.setdefault(item["name"], []).append(item)
            states[state_name] = {
                "name": state_name,
                "invariant": [
                    item if "kind" in item else _expression(item)
                    for item in raw_state.get("invariant", [])
                ],
                "handlers": handlers,
                "boundaries": deepcopy(raw_state.get("boundaries", [])),
                "span": raw_state["span"],
                "lifecycle_owner": lifecycle_owner,
            }

        object_processes = [_handler(item, owner=name) for item in declaration.get("processes", [])]
        object_keys = {
            (item["kind"], item["name"])
            for state in states.values()
            for item in state["handlers"]
        } | {(item["kind"], item["name"]) for item in object_processes}
        inherited_processes = [
            item for item in _type_processes(types, declared_type) if (item["kind"], item["name"]) not in object_keys
        ]
        for process in [*inherited_processes, *object_processes]:
            normalized = deepcopy(process)
            normalized["owner"] = name
            normalized["id"] = f"{name}.{normalized['kind']}::{normalized['name']}@process"
            handlers_by_name.setdefault(normalized["name"], []).append(normalized)

        if initial_state is None:
            _diagnostic(diagnostics, "error", f"system {name} has no initial_state", declaration["span"])
        elif initial_state not in states:
            _diagnostic(
                diagnostics, "error", f"unknown initial state {name}.State::{initial_state}", declaration["span"]
            )
        for state in states.values():
            for handler in state["handlers"]:
                target_state = handler.get("target_state")
                if handler["kind"] == "Transition" and target_state not in states:
                    _diagnostic(
                        diagnostics,
                        "error",
                        f"unknown target state {name}.State::{target_state}",
                        handler["span"],
                    )

        parent = declaration.get("parent")
        if (
            parent is None
            and has_kernel
            and name not in {"Computer", "Kernel"}
            and not _is_subtype(types, declared_type, "ProjectObject")
        ):
            parent = "Kernel"

        fields = _system_fields(types, declaration)
        association_fields = fields.get("associations", [])
        reference_types = {item["name"]: item["type"] for item in association_fields if item.get("type")}
        references = {item["name"]: item["value"] for item in association_fields if item.get("value")}
        for field in declaration.get("fields", {}).get("associations", []):
            if field.get("type"):
                reference_types[field["name"]] = field["type"]
            if field.get("value"):
                references[field["name"]] = field["value"]
        reference_types.update(declaration.get("reference_types", {}))
        references.update(declaration.get("references", {}))

        initial_invariants: list[dict[str, Any]] = []
        for type_decl in reversed(_type_chain(types, declared_type)):
            initial_invariants.extend(deepcopy(type_decl.get("invariant", [])))
        if initial_state in states:
            initial_invariants.extend(deepcopy(states[initial_state]["invariant"]))
        initial_invariants.extend(_expression(item) for item in declaration.get("initial_facts", []))

        systems[name] = {
            "name": name,
            "declaration_kind": declaration["declaration_kind"],
            "declared_type": declared_type,
            "parent": parent,
            "initial_state": initial_state,
            "properties": deepcopy(declaration.get("properties", {})),
            "fields": fields,
            "reference_types": reference_types,
            "references": references,
            "initial_facts": initial_invariants,
            "states": states,
            "processes": [*inherited_processes, *object_processes],
            "handlers_by_name": handlers_by_name,
            "boundaries": deepcopy(declaration.get("boundaries", [])),
            "span": declaration["span"],
        }

    # A Type-owned field is an instance, not an external reference.  Give each
    # statically declared owner a stable private child identity and bind the
    # field path to it so Type processes can address ``self.field``.
    materialize_queue = list(sorted(systems))
    materialize_index = 0
    while materialize_index < len(materialize_queue):
        owner_name = materialize_queue[materialize_index]
        materialize_index += 1
        owner = systems[owner_name]
        owned_fields = _type_fields(types, owner.get("declared_type"), "owned")
        for field in owned_fields:
            field_name = field["name"]
            child_type = field.get("type")
            if not child_type:
                continue
            child_name = f"{owner_name}.{field_name}"
            owner["reference_types"][field_name] = child_type
            owner["references"][field_name] = child_name
            if child_name in systems:
                continue

            lifecycle = _nearest_lifecycle(types, child_type)
            initial_state = lifecycle.get("initial_state") if lifecycle is not None else None
            raw_states = list(lifecycle.get("states", {}).values()) if lifecycle is not None else []
            states: dict[str, dict[str, Any]] = {}
            handlers_by_name: dict[str, list[dict[str, Any]]] = {}
            for raw_state in raw_states:
                state_handlers = [_handler(item, owner=child_name) for item in raw_state.get("handlers", [])]
                for handler in state_handlers:
                    handler["lifecycle_owner"] = lifecycle["name"]
                    handlers_by_name.setdefault(handler["name"], []).append(handler)
                states[raw_state["name"]] = {
                    "name": raw_state["name"],
                    "invariant": deepcopy(raw_state.get("invariant", [])),
                    "handlers": state_handlers,
                    "boundaries": deepcopy(raw_state.get("boundaries", [])),
                    "span": raw_state["span"],
                    "lifecycle_owner": lifecycle["name"],
                }

            inherited_processes = _type_processes(types, child_type)
            for process in inherited_processes:
                normalized = deepcopy(process)
                normalized["owner"] = child_name
                normalized["id"] = (
                    f"{child_name}.{normalized['kind']}::{normalized['name']}@process"
                )
                handlers_by_name.setdefault(normalized["name"], []).append(normalized)

            child_reference_fields = _type_fields(types, child_type, "associations")
            child_initial_facts: list[dict[str, Any]] = []
            for type_decl in reversed(_type_chain(types, child_type)):
                child_initial_facts.extend(deepcopy(type_decl.get("invariant", [])))
            if initial_state in states:
                child_initial_facts.extend(deepcopy(states[initial_state]["invariant"]))
            systems[child_name] = {
                "name": child_name,
                "declaration_kind": "owned",
                "declared_type": child_type,
                "parent": owner_name,
                "initial_state": initial_state,
                "properties": {},
                "fields": {
                    kind: _type_fields(types, child_type, kind)
                    for kind in sorted(
                        {
                            field_kind
                            for type_declaration in _type_chain(types, child_type)
                            for field_kind in type_declaration.get("fields", {})
                        }
                        & {"attrs", "owned", "associations", "references", "slots"}
                    )
                },
                "reference_types": {
                    item["name"]: item["type"]
                    for item in child_reference_fields
                    if item.get("type")
                },
                "references": {
                    item["name"]: item["value"]
                    for item in child_reference_fields
                    if item.get("value")
                },
                "initial_facts": child_initial_facts,
                "states": states,
                "processes": inherited_processes,
                "handlers_by_name": handlers_by_name,
                "boundaries": [],
                "span": field["span"],
                "owned_by": owner_name,
                "owned_field": field_name,
            }
            materialize_queue.append(child_name)

    for name, system in systems.items():
        parent = system.get("parent")
        if parent == name:
            _diagnostic(diagnostics, "error", f"self parent {name}", system["span"])
        elif parent is not None and parent not in systems:
            _diagnostic(diagnostics, "error", f"unknown parent {parent} for system {name}", system["span"])
    for cycle in _parent_cycles(systems):
        _diagnostic(
            diagnostics,
            "error",
            "parent cycle contains systems: " + " -> ".join([*cycle, cycle[0]]),
            systems[cycle[0]]["span"],
        )

    def report_invalid_calls(members: list[dict[str, Any]]) -> None:
        for member in members:
            for entry in member.get("entries", []):
                if entry.get("kind") == "invalid_call":
                    _diagnostic(
                        diagnostics,
                        "unsupported",
                        f"invalid process call: {entry.get('text')}",
                        entry["span"],
                    )
                elif entry.get("kind") == "choice":
                    for choice in entry.get("choices", []):
                        if choice.get("kind") == "invalid_call":
                            _diagnostic(
                                diagnostics,
                                "unsupported",
                                f"invalid process call: {choice.get('text')}",
                                choice["span"],
                            )
            report_invalid_calls(member.get("members", []))
            for variant in member.get("variants", []):
                report_invalid_calls(variant.get("members", []))

    seen_handlers: set[str] = set()
    for system in systems.values():
        for handlers in system["handlers_by_name"].values():
            for handler in handlers:
                if handler["id"] in seen_handlers:
                    continue
                seen_handlers.add(handler["id"])
                report_invalid_calls(handler.get("body", []))

    core = {
        "enums": enums,
        "enum_spans": enum_spans,
        "types": types,
        "predicates": predicates,
        "functions": deepcopy(document.get("functions", [])),
        "contexts": contexts,
        "locks": {item["name"]: deepcopy(item) for item in document.get("locks", [])},
        "systems": systems,
    }
    return core, diagnostics


def model_fingerprint(model: dict[str, Any]) -> str:
    return fingerprint(model)


__all__ = ["build_model", "model_fingerprint"]
