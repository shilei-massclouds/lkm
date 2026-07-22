"""Validate and index the tools2 AST without importing the legacy model."""

from __future__ import annotations

from copy import deepcopy
from typing import Any

from tools2_common import fingerprint


BUILTIN_TYPES = frozenset({"String", "Int", "Bool", "System"})


def _diagnostic(
    diagnostics: list[dict[str, Any]], category: str, message: str, span: dict[str, Any]
) -> None:
    diagnostics.append({"category": category, "message": message, "span": span})


def build_model(document: dict[str, Any]) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    diagnostics = deepcopy(document.get("diagnostics", []))
    enums: dict[str, list[str]] = {}
    enum_spans: dict[str, dict[str, Any]] = {}
    for declaration in document.get("enums", []):
        name = declaration["name"]
        if name in enums:
            _diagnostic(diagnostics, "error", f"duplicate enum {name}", declaration["span"])
            continue
        values = declaration.get("values", [])
        if len(values) != len(set(values)):
            _diagnostic(diagnostics, "error", f"duplicate value in enum {name}", declaration["span"])
        enums[name] = values
        enum_spans[name] = declaration["span"]

    raw_systems = document.get("systems", [])
    known_systems = {item["name"] for item in raw_systems}
    declared_types = {
        item.get("declared_type") for item in raw_systems if item.get("declared_type")
    }
    systems: dict[str, dict[str, Any]] = {}
    for declaration in raw_systems:
        name = declaration["name"]
        if name in systems:
            _diagnostic(diagnostics, "error", f"duplicate system {name}", declaration["span"])
            continue
        states: dict[str, dict[str, Any]] = {}
        handlers_by_name: dict[str, list[dict[str, Any]]] = {}
        for raw_state in declaration.get("states", []):
            state_name = raw_state["name"]
            if state_name in states:
                _diagnostic(
                    diagnostics,
                    "error",
                    f"duplicate state {name}.State::{state_name}",
                    raw_state["span"],
                )
                continue
            handlers: list[dict[str, Any]] = []
            local_identities: set[tuple[str, str]] = set()
            for raw_handler in raw_state.get("handlers", []):
                handler = deepcopy(raw_handler)
                identity = (handler["kind"], handler["name"])
                if identity in local_identities:
                    _diagnostic(
                        diagnostics,
                        "error",
                        f"duplicate handler {name}.{identity[0]}::{identity[1]} in State::{state_name}",
                        handler["span"],
                    )
                local_identities.add(identity)
                handler["id"] = f"{name}.{handler['kind']}::{handler['name']}@{state_name}"
                seen_parameters: set[str] = set()
                for parameter in handler.get("parameters", []):
                    parameter_name = parameter["name"]
                    parameter_type = parameter["type"]
                    if parameter_name in seen_parameters:
                        _diagnostic(
                            diagnostics,
                            "error",
                            f"duplicate parameter {parameter_name} in {handler['id']}",
                            parameter["span"],
                        )
                    seen_parameters.add(parameter_name)
                    if (
                        parameter_type not in BUILTIN_TYPES
                        and parameter_type not in enums
                        and parameter_type not in known_systems
                        and parameter_type not in declared_types
                    ):
                        _diagnostic(
                            diagnostics,
                            "error",
                            f"unknown parameter type {parameter_type} in {handler['id']}",
                            parameter["span"],
                        )
                if handler["kind"] == "Action" and handler.get("target_state") is not None:
                    _diagnostic(
                        diagnostics,
                        "error",
                        f"Action handler {handler['id']} cannot change lifecycle state",
                        handler["span"],
                    )
                handlers.append(handler)
                handlers_by_name.setdefault(handler["name"], []).append(handler)
            states[state_name] = {
                "name": state_name,
                "invariant": deepcopy(raw_state.get("invariant", [])),
                "handlers": handlers,
                "span": raw_state["span"],
            }
        initial_state = declaration.get("initial_state")
        if initial_state is None:
            _diagnostic(diagnostics, "error", f"system {name} has no initial_state", declaration["span"])
        elif initial_state not in states:
            _diagnostic(
                diagnostics,
                "error",
                f"unknown initial state {name}.State::{initial_state}",
                declaration["span"],
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
        if parent is not None and parent not in known_systems:
            _diagnostic(
                diagnostics, "error", f"unknown parent {parent} for system {name}", declaration["span"]
            )
        references = deepcopy(declaration.get("references", {}))
        reference_types = deepcopy(declaration.get("reference_types", {}))
        for reference, reference_type in reference_types.items():
            if (
                reference_type != "System"
                and reference_type not in known_systems
                and reference_type not in declared_types
            ):
                _diagnostic(
                    diagnostics,
                    "error",
                    f"unknown reference type {reference_type} for {name}.{reference}",
                    declaration["span"],
                )
        for reference, target in references.items():
            if target not in known_systems:
                _diagnostic(
                    diagnostics,
                    "error",
                    f"unknown initial reference target {target} for {name}.{reference}",
                    declaration["span"],
                )
            else:
                expected = reference_types.get(reference, "System")
                target_declaration = next(item for item in raw_systems if item["name"] == target)
                if (
                    expected != "System"
                    and target != expected
                    and target_declaration.get("declared_type") != expected
                ):
                    _diagnostic(
                        diagnostics,
                        "error",
                        f"reference {name}.{reference} expects {expected}, got {target}",
                        declaration["span"],
                    )
        systems[name] = {
            "name": name,
            "declaration_kind": declaration["declaration_kind"],
            "declared_type": declaration.get("declared_type"),
            "parent": parent,
            "initial_state": initial_state,
            "reference_types": reference_types,
            "references": references,
            "initial_facts": deepcopy(declaration.get("initial_facts", [])),
            "states": states,
            "handlers_by_name": handlers_by_name,
            "span": declaration["span"],
        }

    for name in systems:
        seen: set[str] = set()
        current: str | None = name
        while current is not None:
            if current in seen:
                _diagnostic(
                    diagnostics,
                    "error",
                    f"parent cycle contains system {current}",
                    systems[name]["span"],
                )
                break
            seen.add(current)
            current = systems.get(current, {}).get("parent")

    core = {
        "enums": enums,
        "enum_spans": enum_spans,
        "systems": systems,
    }
    return core, diagnostics


def model_fingerprint(model: dict[str, Any]) -> str:
    return fingerprint(model)
