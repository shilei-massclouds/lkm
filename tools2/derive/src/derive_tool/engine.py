"""Deterministic bounded Signal derivation for the tools2 compatibility model."""

from __future__ import annotations

from copy import deepcopy
import json
from pathlib import Path
import re
from typing import Any

from tools2_common import (
    PRODUCER,
    SNAPSHOT_SCHEMA,
    SNAPSHOT_VERSION,
    ProtocolError,
    normalize_signal_request,
    read_json,
    require_protocol,
)


class DerivationProblem(ValueError):
    pass


class UntilReached(Exception):
    """Internal non-failure control flow for a matched pre-send boundary."""


def parse_budget(value: str) -> int | None:
    if value == "all":
        return None
    try:
        number = int(value)
    except ValueError as exc:
        raise ValueError("expected a non-negative integer or 'all'") from exc
    if number < 0:
        raise ValueError("expected a non-negative integer or 'all'")
    return number


def _snapshot(value: dict[str, Any]) -> dict[str, Any]:
    return {
        "states": dict(sorted(value["states"].items())),
        "facts": sorted(set(value["facts"])),
        "references": dict(sorted(value["references"].items())),
        "instances": {
            key: deepcopy(item)
            for key, item in sorted(value.get("instances", {}).items())
        },
    }


def _fact_argument(value: Any) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, str):
        symbolic = re.fullmatch(
            r"(?:dynamic:[^\s,()]+|[A-Za-z_][A-Za-z0-9_:@-]*(?:\[[^\[\]]+\])?(?:\.[A-Za-z_][A-Za-z0-9_:@-]*(?:\[[^\[\]]+\])?)*)",
            value,
        )
        if symbolic is None:
            return json.dumps(value, ensure_ascii=False, separators=(",", ":"))
    return str(value)


def _fact(name: str, arguments: list[Any]) -> str:
    return name if not arguments else f"{name}({','.join(_fact_argument(item) for item in arguments)})"


def _assertion(text: str) -> str:
    return "assert:" + " ".join(text.split())


def _split_fact_arguments(text: str) -> list[str]:
    result: list[str] = []
    depth = 0
    start = 0
    for index, char in enumerate(text):
        if char in "(<":
            depth += 1
        elif char in ")>":
            depth = max(0, depth - 1)
        elif char == "," and depth == 0:
            result.append(text[start:index].strip())
            start = index + 1
    tail = text[start:].strip()
    if tail:
        result.append(tail)
    return result


def _matches_system_type(model: dict[str, Any], target: str, expected: str) -> bool:
    if target not in model["systems"]:
        return False
    if expected == "System" or target == expected:
        return True
    actual = model["systems"][target].get("declared_type")
    seen: set[str] = set()
    while actual and actual not in seen:
        if actual == expected:
            return True
        seen.add(actual)
        declaration = model.get("types", {}).get(actual)
        actual = declaration.get("base_type") if declaration is not None else None
    return False


def _nominal_type(type_expression: str | None) -> str | None:
    if type_expression is None:
        return None
    match = re.match(r"[A-Za-z_][A-Za-z0-9_]*", type_expression)
    return match.group(0) if match is not None else None


def _slot_field_name(member: str) -> str:
    return member[:1].lower() + member[1:]


def load_scenario(
    path: str | Path | None,
    model: dict[str, Any],
    initial: dict[str, Any],
    *,
    model_fingerprint: str,
) -> dict[str, Any]:
    if path is None:
        return _snapshot(initial)
    value = read_json(path)
    trusted_snapshot = False
    if "schema" in value:
        require_protocol(
            value, schema=SNAPSHOT_SCHEMA, version=SNAPSHOT_VERSION, label="snapshot"
        )
        if value.get("model_fingerprint") != model_fingerprint:
            raise ProtocolError("snapshot model fingerprint does not match the loaded model")
        trusted_snapshot = True
        value = value.get("snapshot", {})
    elif "snapshot" in value:
        value = value["snapshot"]
    result = deepcopy(initial)
    states = value.get("states", value.get("state", {}))
    if not isinstance(states, dict):
        raise ProtocolError("scenario states must be an object")
    instances = value.get("instances", {}) if trusted_snapshot else {}
    if not isinstance(instances, dict):
        raise ProtocolError("snapshot instances must be an object")
    for target, state in states.items():
        if target not in model["systems"] and target not in instances:
            raise ProtocolError(f"scenario has unknown system state target {target}")
        if target in model["systems"]:
            system = model["systems"][target]
            allowed_states = set(system["states"]) | {system.get("initial_state")}
        else:
            instance_type = instances[target].get("declared_type")
            type_decl = model.get("types", {}).get(instance_type, {})
            lifecycle = model.get("types", {}).get(
                type_decl.get("effective_lifecycle_type", instance_type), {}
            )
            allowed_states = set(lifecycle.get("states", {})) | {
                lifecycle.get("initial_state")
            }
            if lifecycle.get("initial_state") is None:
                allowed_states.add("Base")
        if state not in allowed_states:
            raise ProtocolError(f"scenario has unknown state {target}.State::{state}")
        result["states"][target] = state
    facts = value.get("facts", [])
    if isinstance(facts, dict):
        for fact, present in facts.items():
            if present:
                result["facts"].append(fact)
            else:
                result["facts"] = [item for item in result["facts"] if item != fact]
    elif isinstance(facts, list) and all(isinstance(item, str) for item in facts):
        result["facts"].extend(facts)
    else:
        raise ProtocolError("scenario facts must be a string list or boolean object")
    references = value.get("references", {})
    if not isinstance(references, dict):
        raise ProtocolError("scenario references must be an object")
    for reference, target in references.items():
        if "." not in reference:
            raise ProtocolError(f"scenario reference must be System.field: {reference}")
        owners = [
            name
            for name in (*model["systems"], *instances)
            if reference.startswith(f"{name}.")
        ]
        if not owners:
            if trusted_snapshot:
                result["references"][reference] = target
                continue
            owner, field = reference.split(".", 1)
            raise ProtocolError(f"scenario has unknown reference owner {owner}")
        owner = max(owners, key=len)
        field = reference[len(owner) + 1 :]
        if owner in instances:
            if target not in model["systems"] and target not in instances:
                raise ProtocolError(f"scenario has unknown reference target {target}")
            result["references"][reference] = target
            continue
        if field not in model["systems"][owner]["reference_types"]:
            if trusted_snapshot and target in instances:
                result["references"][reference] = target
                continue
            raise ProtocolError(f"scenario has unknown reference {reference}")
        if target not in model["systems"] and not trusted_snapshot:
            raise ProtocolError(f"scenario has unknown reference target {target}")
        expected = model["systems"][owner]["reference_types"][field]
        if target in model["systems"] and not _matches_system_type(model, target, expected):
            raise ProtocolError(
                f"scenario reference {reference} expects {expected}, got {target}"
            )
        result["references"][reference] = target
    if trusted_snapshot:
        result["instances"] = deepcopy(instances)
    return _snapshot(result)


class Engine:
    def __init__(
        self,
        model_document: dict[str, Any],
        *,
        source: str,
        root_target: str,
        root_name: str,
        orchestration: dict[str, Any] | None,
        until_target: str | None,
        until_name: str | None,
        initial_snapshot: dict[str, Any],
        max_depth: int | None,
        max_breadth: int | None,
    ) -> None:
        self.document = model_document
        self.model = deepcopy(model_document["model"])
        self.systems = deepcopy(self.model["systems"])
        self.model["systems"] = self.systems
        self.source = source
        self.root_target = root_target
        self.root_name = root_name
        self.orchestration = orchestration
        self.until_target = until_target
        self.until_name = until_name
        self.initial_snapshot = _snapshot(initial_snapshot)
        self.current = _snapshot(initial_snapshot)
        for identity, metadata in self.current.get("instances", {}).items():
            self._install_runtime_system(identity, metadata)
        self.max_depth = max_depth
        self.max_breadth = max_breadth
        self.signals: list[dict[str, Any]] = []
        self.signal_index: dict[str, dict[str, Any]] = {}
        self.events: list[dict[str, Any]] = []
        self.queue: list[dict[str, Any]] = []
        self.frontier: list[dict[str, Any]] = []
        self.boundary: dict[str, Any] | None = None
        self.failed = False
        self.was_bounded = False
        self.failure_signal_id: str | None = None
        self.failure_reason: str | None = None
        self.next_signal = 1
        self.next_event = 1
        self.next_instance = len(self.current.get("instances", {})) + 1
        self.next_generation = 1 + max(
            (
                int(item.get("generation", 0))
                for item in self.current.get("instances", {}).values()
                if isinstance(item, dict)
            ),
            default=0,
        )
        self.active_requests: set[tuple[str, str, bytes]] = set()
        self.active_predicates: set[tuple[str, tuple[str, ...]]] = set()
        for fact in self.initial_snapshot["facts"]:
            self.event("initial_fact_established", fact=fact, source="initial_state_invariant")

    def _runtime_handler(self, handler: dict[str, Any], identity: str) -> dict[str, Any]:
        result = deepcopy(handler)
        result["owner"] = identity
        location = result.get("source_state") or "process"
        result["id"] = f"{identity}.{result['kind']}::{result['name']}@{location}"
        return result

    def _runtime_fields(self, declared_type: str) -> dict[str, list[dict[str, Any]]]:
        fields: dict[str, dict[str, dict[str, Any]]] = {}
        for declaration in reversed(self._type_chain(declared_type)):
            for kind, items in declaration.get("fields", {}).items():
                indexed = fields.setdefault(kind, {})
                for item in items:
                    indexed[item["name"]] = deepcopy(item)
        return {
            kind: list(indexed.values()) for kind, indexed in sorted(fields.items())
        }

    def _install_runtime_system(
        self, identity: str, metadata: dict[str, Any]
    ) -> dict[str, Any]:
        if identity in self.systems:
            return self.systems[identity]
        declared_type = metadata.get("declared_type")
        type_decl = self.model.get("types", {}).get(declared_type)
        if type_decl is None:
            raise DerivationProblem(f"unknown runtime declared type {declared_type}")
        lifecycle_name = type_decl.get("effective_lifecycle_type")
        lifecycle = self.model.get("types", {}).get(lifecycle_name)
        structural_only = lifecycle is None or lifecycle.get("initial_state") is None
        if structural_only:
            lifecycle_name = declared_type
            lifecycle = {
                "initial_state": "Base",
                "states": {"Base": {"handlers": [], "invariant": []}},
            }

        states: dict[str, dict[str, Any]] = {}
        handlers_by_name: dict[str, list[dict[str, Any]]] = {}
        for state_name, raw_state in lifecycle.get("states", {}).items():
            handlers = [
                self._runtime_handler(item, identity)
                for item in raw_state.get("handlers", [])
            ]
            for handler in handlers:
                handlers_by_name.setdefault(handler["name"], []).append(handler)
            states[state_name] = {
                **deepcopy(raw_state),
                "handlers": handlers,
                "lifecycle_owner": lifecycle_name,
            }

        processes = [
            self._runtime_handler(item, identity)
            for item in type_decl.get("effective_processes", [])
            if item.get("source_state") is None
        ]
        for process in processes:
            handlers_by_name.setdefault(process["name"], []).append(process)
        fields = self._runtime_fields(declared_type)
        associations = fields.get("associations", [])
        references = {
            item["name"]: str(item["value"]).replace("self", identity, 1)
            for item in associations
            if item.get("value") is not None
        }
        system = {
            "name": identity,
            "declaration_kind": "runtime_indexed"
            if metadata.get("indexed")
            else "runtime",
            "declared_type": declared_type,
            "parent": metadata.get("parent"),
            "initial_state": lifecycle["initial_state"],
            "properties": {},
            "fields": fields,
            "reference_types": {
                item["name"]: item["type"]
                for item in associations
                if item.get("type")
            },
            "references": references,
            "initial_facts": [
                *deepcopy(type_decl.get("invariant", [])),
                *deepcopy(states[lifecycle["initial_state"]].get("invariant", [])),
            ],
            "states": states,
            "processes": processes,
            "handlers_by_name": handlers_by_name,
            "boundaries": [],
            "span": deepcopy(metadata.get("span", type_decl.get("span", {}))),
            "runtime_instance": deepcopy(metadata),
        }
        self.systems[identity] = system
        for field in fields.get("owned", []):
            if field.get("indexed") or not field.get("type"):
                continue
            field_name = field["name"]
            child_identity = f"{identity}.{field_name}"
            child_metadata = {
                "declared_type": field["type"],
                "parent": identity,
                "indexed": False,
                "owned_field": field_name,
                "resident": True,
                "span": deepcopy(field.get("span", metadata.get("span", {}))),
            }
            self.current["instances"].setdefault(child_identity, child_metadata)
            self.current["references"][f"{identity}.{field_name}"] = child_identity
            system["references"][field_name] = child_identity
            system["reference_types"][field_name] = field["type"]
            self._install_runtime_system(child_identity, child_metadata)
        return system

    def _activate_static_system(self, name: str, *, signal: dict[str, Any]) -> None:
        if name in self.current["states"]:
            return
        system = self.systems[name]
        self.current["states"][name] = system["initial_state"]
        activation_signal = {**signal, "target": name, "_self_value": name}
        for field, target in system.get("references", {}).items():
            self.current["references"][f"{name}.{field}"] = target
        for expression in system.get("initial_facts", []):
            if expression.get("kind") == "fact":
                values = [
                    self.value(item, signal=activation_signal, bindings={})
                    for item in expression.get("arguments", [])
                ]
                self.current["facts"].append(_fact(expression["name"], values))
            elif expression.get("kind") in {"assertion", "reference_condition"}:
                rendered = re.sub(
                    r"\bself\b",
                    name,
                    expression.get("expression", expression.get("text", "")),
                )
                self.current["facts"].append(_assertion(rendered))
        self.current = _snapshot(self.current)
        for child_name, child in sorted(self.systems.items()):
            if child.get("parent") == name:
                field_name = child_name.rsplit(".", 1)[-1]
                self.current["references"][f"{name}.{field_name}"] = child_name
                system["reference_types"][field_name] = (
                    child.get("declared_type") or "System"
                )
                self._activate_static_system(child_name, signal=signal)

    def event(self, kind: str, **fields: Any) -> None:
        self.events.append({"sequence": self.next_event, "kind": kind, **fields})
        self.next_event += 1

    @staticmethod
    def self_value(signal: dict[str, Any]) -> Any:
        return signal.get("_self_value", signal["target"])

    def new_signal(
        self,
        *,
        source: str,
        target: str,
        name: str,
        raw_arguments: list[dict[str, Any]],
        delivery: str,
        cause_id: str | None,
        coordinate: dict[str, Any],
        compat_process_kind: str | None,
        call_span: dict[str, Any] | None = None,
        fifo_position: int | None = None,
    ) -> dict[str, Any]:
        if target == self.until_target and name == self.until_name:
            send_position = {
                "delivery": delivery,
                "cause_id": cause_id,
                "coordinate": {
                    key: value for key, value in coordinate.items() if not key.startswith("_")
                },
            }
            if fifo_position is not None:
                send_position["fifo_position"] = fifo_position
            self.boundary = {
                "kind": "before_signal_send",
                "normalized_signal": f"{target}.{name}",
                "target": target,
                "signal": name,
                "source": source,
                "send_position": send_position,
                "call_span": deepcopy(call_span),
                "snapshot": _snapshot(self.current),
            }
            self.event(
                "until_signal_reached",
                normalized_signal=f"{target}.{name}",
                source=source,
                send_position=deepcopy(send_position),
                call_span=deepcopy(call_span),
            )
            for queued in self.queue:
                if queued["outcome"] != "sent":
                    continue
                queued["outcome"] = "stopped"
                queued["reason"] = "until_signal_reached"
                queued["before_snapshot"] = _snapshot(self.current)
                queued["after_snapshot"] = _snapshot(self.current)
                self.event(
                    "signal_stopped",
                    signal_id=queued["id"],
                    reason="until_signal_reached",
                    location="fifo",
                )
            self.queue.clear()
            raise UntilReached
        signal_id = f"sig-{self.next_signal:04d}"
        self.next_signal += 1
        signal = {
            "id": signal_id,
            "source": source,
            "target": target,
            "name": name,
            "delivery": delivery,
            "cause_id": cause_id,
            "response_parent_id": cause_id,
            "compat_process_kind": compat_process_kind,
            "coordinate": {key: value for key, value in coordinate.items() if not key.startswith("_")},
            "payload": [],
            "handler": None,
            "before_snapshot": None,
            "after_snapshot": None,
            "outcome": "sent",
            "reason": None,
            "_raw_arguments": raw_arguments,
            "_budget_depth": coordinate.get("_budget_depth", abs(coordinate["depth"])),
            "_budget_breadth": coordinate.get("_budget_breadth", coordinate["breadth"]),
        }
        self.signals.append(signal)
        self.signal_index[signal_id] = signal
        self.event(
            "signal_sent",
            signal_id=signal_id,
            source=source,
            target=target,
            signal=name,
            delivery=delivery,
            cause_id=cause_id,
            coordinate=signal["coordinate"],
        )
        return signal

    def coordinate(self, source: str, target: str, current: dict[str, Any]) -> dict[str, Any]:
        if source == target:
            return {
                "depth": current["depth"],
                "breadth": current["breadth"],
                "movement": {"up": 0, "across": 0, "down": 0},
                "_budget_depth": abs(current["depth"]),
                "_budget_breadth": current["breadth"],
            }
        if source not in self.systems or target not in self.systems:
            return {
                "depth": current["depth"],
                "breadth": current["breadth"],
                "movement": {"up": 0, "across": 1, "down": 0},
                "_budget_depth": abs(current["depth"]),
                "_budget_breadth": current["breadth"] + 1,
            }

        def lineage(name: str) -> list[str]:
            result = [name]
            while self.systems[result[-1]]["parent"] is not None:
                result.append(self.systems[result[-1]]["parent"])
            if result[-1] != "Environment":
                result.append("Environment")
            return result

        source_line = lineage(source)
        target_line = lineage(target)
        if source in target_line:
            down = target_line.index(source)
            depth = current["depth"] + down
            return {
                "depth": depth,
                "breadth": 0,
                "movement": {"up": 0, "across": 0, "down": down},
                "_budget_depth": max(abs(current["depth"]), abs(depth)),
                "_budget_breadth": 0,
            }
        if target in source_line:
            up = source_line.index(target)
            depth = current["depth"] - up
            return {
                "depth": depth,
                "breadth": 0,
                "movement": {"up": up, "across": 0, "down": 0},
                "_budget_depth": max(abs(current["depth"]), abs(depth)),
                "_budget_breadth": 0,
            }
        shared = next((item for item in source_line if item in target_line), None)
        if shared is None:
            raise DerivationProblem(f"systems {source} and {target} have no common parent tree")
        up = source_line.index(shared)
        down = target_line.index(shared)
        depth_at_lca = current["depth"] - up
        depth = depth_at_lca + down
        if down == 1 and up == 1:
            breadth = current["breadth"] + 1
            breadth_peak = breadth
        else:
            breadth = 0
            breadth_peak = 1
        return {
            "depth": depth,
            "breadth": breadth,
            "movement": {"up": up, "across": 1, "down": down},
            "_budget_depth": max(abs(current["depth"]), abs(depth_at_lca), abs(depth)),
            "_budget_breadth": breadth_peak,
        }

    def over_budget(self, signal: dict[str, Any]) -> bool:
        return (
            self.max_depth is not None and signal["_budget_depth"] > self.max_depth
        ) or (
            self.max_breadth is not None and signal["_budget_breadth"] > self.max_breadth
        )

    def fail(self, signal: dict[str, Any], reason: str) -> None:
        signal["outcome"] = "failed"
        signal["reason"] = reason
        signal["after_snapshot"] = _snapshot(self.current)
        self.failed = True
        if self.failure_signal_id is None:
            self.failure_signal_id = signal["id"]
            self.failure_reason = reason
        self.event("signal_failed", signal_id=signal["id"], reason=reason)

    def reject(self, signal: dict[str, Any], reason: str) -> None:
        signal["reason"] = reason
        signal["after_snapshot"] = _snapshot(self.current)
        self.event("signal_rejected", signal_id=signal["id"], reason=reason)
        signal["outcome"] = "rejected"
        self.failed = True
        if self.failure_signal_id is None:
            self.failure_signal_id = signal["id"]
            self.failure_reason = reason

    def resolve_path(
        self, path: str, *, signal: dict[str, Any], bindings: dict[str, Any]
    ) -> Any:
        parts = path.split(".")
        first = parts[0]
        if first == "CurrentCPU":
            value = self._current_cpu_target(signal)
        elif first == "CurrentTask":
            value = self._current_task_target(signal)
        elif first == "CurrentTaskRef":
            value = self._current_task_ref(signal)
        elif first == "self":
            value: Any = self.self_value(signal)
        elif first in bindings:
            value = bindings[first]
        elif first in self.systems:
            value = first
        else:
            raise DerivationProblem(f"unresolved system/reference path {path}")
        for field in parts[1:]:
            if field == "parent" and value in self.systems:
                parent = self.systems[value].get("parent")
                if parent is None:
                    raise DerivationProblem(f"unbound system parent {value}.parent")
                value = parent
                continue
            key = f"{value}.{field}"
            if key not in self.current["references"]:
                raise DerivationProblem(f"unbound system reference {key}")
            value = self.current["references"][key]
        return value

    def _effective_flow(self, signal: dict[str, Any]) -> str | None:
        target = signal.get("target")
        current = target
        seen: set[str] = set()
        while current in self.systems and current not in seen:
            seen.add(str(current))
            if _matches_system_type(self.model, str(current), "TaskFlow"):
                return str(current)
            current = self.systems[str(current)].get("parent")
        inherited = signal.get("_effective_flow")
        return str(inherited) if inherited in self.systems else None

    def _current_cpu_target(self, signal: dict[str, Any]) -> str:
        flow = self._effective_flow(signal)
        if flow is None:
            raise DerivationProblem("CurrentCPU requires an effective TaskFlow")
        reference = self.current["references"].get(f"{flow}.cpu_ref")
        if reference is None:
            raise DerivationProblem(f"CurrentCPU source Flow {flow} has no CpuRef")
        target = self._deref(reference)
        if target not in self.systems or target not in self.current["states"]:
            raise DerivationProblem(
                f"CurrentCPU CpuRef {reference} targets no published CPU element"
            )
        if not _matches_system_type(self.model, str(target), "CPU"):
            raise DerivationProblem(f"CurrentCPU CpuRef {reference} targets non-CPU {target}")
        self._record_selector_resolution(signal, {
            "selector": "CurrentCPU",
            "target": str(target),
            "source_flow": flow,
            "source_cpu_ref": reference,
        })
        return str(target)

    @staticmethod
    def _record_selector_resolution(
        signal: dict[str, Any], resolution: dict[str, Any]
    ) -> None:
        resolutions = signal.setdefault("_selector_resolutions", [])
        if resolution not in resolutions:
            resolutions.append(resolution)

    def _fact_targets(self, predicate: str, source: str) -> list[str]:
        pattern = re.compile(
            rf"{re.escape(predicate)}\({re.escape(source)},([^,)]+)\)"
        )
        return sorted(
            {
                match.group(1)
                for fact in self.current["facts"]
                if (match := pattern.fullmatch(fact)) is not None
            }
        )

    def _current_task_context(self, signal: dict[str, Any]) -> tuple[str, str, str]:
        flow = self._effective_flow(signal)
        if flow is None:
            raise DerivationProblem("CurrentTask requires an effective TaskFlow")

        parents = self._fact_targets("task_flow_parent_is", flow)
        owners = self._fact_targets("task_flow_owner_is", flow)
        if len(parents) != 1:
            raise DerivationProblem(
                f"CurrentTask source Flow {flow} must have exactly one parent Task"
            )
        if len(owners) != 1:
            raise DerivationProblem(
                f"CurrentTask source Flow {flow} must have exactly one owner Task"
            )
        task = parents[0]
        if owners[0] != task:
            raise DerivationProblem(
                f"CurrentTask source Flow {flow} parent {task} disagrees with owner {owners[0]}"
            )
        if task not in self.systems or not _matches_system_type(self.model, task, "Task"):
            raise DerivationProblem(
                f"CurrentTask source Flow {flow} parent {task} is not a published Task"
            )
        if self.current["states"].get(task) != "OnCpu":
            raise DerivationProblem(f"CurrentTask target Task {task} is not OnCpu")
        live_fact = (
            f"task_execution_authority_is({task},TaskExecutionAuthority::Live)"
        )
        if live_fact not in self.current["facts"]:
            raise DerivationProblem(f"CurrentTask target Task {task} is not Live")
        active_flow = self.current["references"].get(f"{task}.active_flow")
        if active_flow != flow:
            raise DerivationProblem(
                f"CurrentTask target Task {task} active Flow {active_flow} does not match effective Flow {flow}"
            )

        live_tasks = sorted(
            candidate
            for candidate in self.systems
            if _matches_system_type(self.model, candidate, "Task")
            and self.current["states"].get(candidate) == "OnCpu"
            and (
                f"task_execution_authority_is({candidate},TaskExecutionAuthority::Live)"
                in self.current["facts"]
            )
        )
        if live_tasks != [task]:
            raise DerivationProblem(
                f"CurrentTask requires one OnCpu/Live Task, found {live_tasks}"
            )

        references = []
        for reference in self._fact_sources("task_ref_targets", task):
            if f"task_ref_ready({reference})" in self.current["facts"]:
                references.append(reference)
        references = sorted(set(references))
        if len(references) != 1:
            raise DerivationProblem(
                f"CurrentTask target Task {task} must have exactly one live TaskRef, found {references}"
            )
        reference = references[0]
        if self._deref(reference) != task:
            raise DerivationProblem(
                f"CurrentTask TaskRef {reference} does not dereference to {task}"
            )
        self._record_selector_resolution(signal, {
            "selector": "CurrentTask",
            "source_flow": flow,
            "source_task_ref": reference,
            "target": task,
        })
        return task, reference, flow

    def _fact_sources(self, predicate: str, target: str) -> list[str]:
        pattern = re.compile(
            rf"{re.escape(predicate)}\(([^,)]+),{re.escape(target)}\)"
        )
        return sorted(
            {
                match.group(1)
                for fact in self.current["facts"]
                if (match := pattern.fullmatch(fact)) is not None
            }
        )

    def _current_task_target(self, signal: dict[str, Any]) -> str:
        task, _, _ = self._current_task_context(signal)
        return task

    def _current_task_ref(self, signal: dict[str, Any]) -> str:
        _, reference, _ = self._current_task_context(signal)
        return reference

    def _deref(self, value: Any) -> Any:
        if value in self.systems:
            return value
        escaped = re.escape(str(value))
        pattern = re.compile(rf"[A-Za-z_][A-Za-z0-9_]*_ref_targets\({escaped},([^,)]+)\)")
        for fact in self.current["facts"]:
            match = pattern.fullmatch(fact)
            if match and match.group(1) in self.systems:
                return match.group(1)
        return value

    def _reference_type(self, value: Any) -> str | None:
        escaped = re.escape(str(value))
        pattern = re.compile(
            rf"(?P<prefix>[A-Za-z_][A-Za-z0-9_]*)_ref_targets\({escaped},[^,)]+\)"
        )
        normalized_types = {
            re.sub(r"[^a-z0-9]", "", name.lower()): name
            for name in self.model.get("types", {})
        }
        for fact in self.current["facts"]:
            match = pattern.fullmatch(fact)
            if match is None:
                continue
            normalized = re.sub(
                r"[^a-z0-9]", "", (match.group("prefix") + "_ref").lower()
            )
            if normalized in normalized_types:
                return normalized_types[normalized]
        return None

    @staticmethod
    def _field_in_groups(
        fields: dict[str, list[dict[str, Any]]], field_name: str
    ) -> dict[str, Any] | None:
        for group in fields.values():
            for field in group:
                if field["name"] == field_name:
                    return field
        return None

    def _type_chain(self, type_name: str | None) -> list[dict[str, Any]]:
        result: list[dict[str, Any]] = []
        seen: set[str] = set()
        current = _nominal_type(type_name)
        while current is not None and current not in seen:
            declaration = self.model.get("types", {}).get(current)
            if declaration is None:
                break
            seen.add(current)
            result.append(declaration)
            current = _nominal_type(declaration.get("base_type"))
        return result

    def _type_field(self, type_name: str, field_name: str) -> dict[str, Any] | None:
        for declaration in self._type_chain(type_name):
            field = self._field_in_groups(declaration.get("fields", {}), field_name)
            if field is not None:
                return field
        return None

    def _structural_value_owner(self, value: Any) -> tuple[str, str] | None:
        """Resolve a symbolic instance path to its final system or declared type."""

        text = str(value)
        if text in self.systems:
            return ("system", text)
        if text in self.model.get("types", {}):
            return ("type", text)

        owners = [name for name in self.systems if text.startswith(f"{name}.")]
        if owners:
            owner = max(owners, key=len)
            remaining = text[len(owner) + 1 :].split(".")
            owner_kind = "system"
            owner_name = owner
        else:
            type_names = [
                name
                for name in self.model.get("types", {})
                if text.startswith(f"{name}.")
            ]
            if not type_names:
                return None
            owner_name = max(type_names, key=len)
            remaining = text[len(owner_name) + 1 :].split(".")
            owner_kind = "type"

        for field_name in remaining:
            if owner_kind == "system":
                field = self._field_in_groups(
                    self.systems[owner_name].get("fields", {}), field_name
                )
            else:
                field = self._type_field(owner_name, field_name)
            if field is None:
                return None
            field_type = _nominal_type(field.get("type"))
            if field_type is None or field_type not in self.model.get("types", {}):
                return None
            owner_kind = "type"
            owner_name = field_type
        return (owner_kind, owner_name)

    def _declares_slot(self, value: Any, slot: Any) -> bool:
        slot_match = re.fullmatch(
            r"[A-Za-z_][A-Za-z0-9_]*::([A-Za-z_][A-Za-z0-9_]*)", str(slot)
        )
        if slot_match is None:
            return False
        expected = _slot_field_name(slot_match.group(1))
        owner = self._structural_value_owner(value)
        if owner is None:
            return False
        owner_kind, owner_name = owner
        if owner_kind == "system":
            if any(
                field["name"] == expected
                for field in self.systems[owner_name].get("fields", {}).get("slots", [])
            ):
                return True
            type_name = self.systems[owner_name].get("declared_type")
        else:
            type_name = owner_name
        return any(
            field["name"] == expected
            for declaration in self._type_chain(type_name)
            for field in declaration.get("fields", {}).get("slots", [])
        )

    def resolve_receiver(
        self, path: str, *, signal: dict[str, Any], bindings: dict[str, Any]
    ) -> str:
        parts = path.split(".")
        first = parts[0]
        if first == "CurrentCPU":
            value = self._current_cpu_target(signal)
        elif first == "CurrentTask":
            value = self._current_task_target(signal)
        elif first == "CurrentTaskRef":
            value = self._current_task_ref(signal)
        elif first == "self":
            value: Any = self.self_value(signal)
        elif first in bindings:
            value = bindings[first]
        elif first in self.systems:
            value = first
        else:
            value = first
        for field in parts[1:]:
            value = self._deref(value)
            key = f"{value}.{field}"
            if key not in self.current["references"]:
                raise DerivationProblem(f"unbound system reference {key}")
            value = self.current["references"][key]
        value = self._deref(value)
        if value not in self.systems:
            raise DerivationProblem(f"receiver {path} does not resolve to a static system")
        return str(value)

    def value(
        self, expression: dict[str, Any], *, signal: dict[str, Any], bindings: dict[str, Any]
    ) -> Any:
        kind = expression["kind"]
        if kind in {"string", "integer", "boolean"}:
            return expression["value"]
        if kind == "enum":
            return f"{expression['type']}::{expression['value']}"
        if kind == "path":
            path = expression["value"]
            first = path.split(".", 1)[0]
            if first in {"CurrentCPU", "CurrentTask", "CurrentTaskRef"} or first == "self" or first in bindings:
                try:
                    return self.resolve_path(path, signal=signal, bindings=bindings)
                except DerivationProblem:
                    if first in {"CurrentCPU", "CurrentTask", "CurrentTaskRef"}:
                        raise
                    base = self.self_value(signal) if first == "self" else bindings[first]
                    suffix = path.split(".", 1)[1] if "." in path else ""
                    return str(base) + (f".{suffix}" if suffix else "")
            if first in self.systems:
                try:
                    return self.resolve_path(path, signal=signal, bindings=bindings)
                except DerivationProblem:
                    return path
            if "." not in path and path and path[0].islower():
                return f"{signal['target']}.{path}"
            return path
        if kind == "expression":
            return self.substitute(expression["value"], signal=signal, bindings=bindings)
        raise DerivationProblem(f"unsupported runtime value kind {kind}")

    def substitute(
        self, text: str, *, signal: dict[str, Any], bindings: dict[str, Any]
    ) -> str:
        values = {"self": self.self_value(signal), **{key: str(value) for key, value in bindings.items()}}
        result = text
        for name in sorted(values, key=len, reverse=True):
            result = re.sub(rf"\b{re.escape(name)}\b", values[name], result)
        return " ".join(result.split())

    def materialize_arguments(
        self,
        arguments: list[dict[str, Any]],
        *,
        signal: dict[str, Any],
        bindings: dict[str, Any],
    ) -> list[dict[str, Any]]:
        materialized: list[dict[str, Any]] = []
        for argument in arguments:
            value = self.value(argument["value"], signal=signal, bindings=bindings)
            if isinstance(value, bool):
                expression = {"kind": "boolean", "value": value}
            elif isinstance(value, int):
                expression = {"kind": "integer", "value": value}
            elif isinstance(value, str) and "::" in value:
                enum_type, member = value.split("::", 1)
                expression = {"kind": "enum", "type": enum_type, "value": member}
            elif value in self.systems:
                expression = {"kind": "path", "value": value}
            else:
                expression = {"kind": "string", "value": value}
            materialized.append({"name": argument.get("name"), "value": expression})
        return materialized

    def bind_payload(self, signal: dict[str, Any], handler: dict[str, Any]) -> dict[str, Any]:
        parameters = handler.get("parameters", [])
        raw = signal["_raw_arguments"]
        named = [item for item in raw if item.get("name") is not None]
        positional = [item for item in raw if item.get("name") is None]
        if named and positional:
            raise DerivationProblem("cannot mix named and positional Signal payload arguments")
        if positional:
            if len(parameters) != len(positional):
                raise DerivationProblem(
                    f"positional payload count mismatch: expected {len(parameters)}, got {len(positional)}"
                )
            assignments = {
                parameter["name"]: argument["value"]
                for parameter, argument in zip(parameters, positional)
            }
        else:
            assignments: dict[str, dict[str, Any]] = {}
            for item in named:
                if item["name"] in assignments:
                    raise DerivationProblem(f"duplicate payload argument {item['name']}")
                assignments[item["name"]] = item["value"]
        expected = {item["name"] for item in parameters}
        supplied = set(assignments)
        if expected != supplied:
            missing = sorted(expected - supplied)
            unknown = sorted(supplied - expected)
            raise DerivationProblem(f"payload mismatch: missing={missing}, unknown={unknown}")
        bindings: dict[str, Any] = {}
        payload: list[dict[str, Any]] = []
        generic_bounds = {
            item["name"]: item["bound"] for item in handler.get("type_parameters", [])
        }
        for parameter in parameters:
            name = parameter["name"]
            value = self.value(assignments[name], signal=signal, bindings=bindings)
            expected_type = parameter["type"]
            concrete_type = generic_bounds.get(expected_type, expected_type)
            if value not in self.systems:
                dereferenced = self._deref(value)
                if dereferenced in self.systems and _matches_system_type(
                    self.model, dereferenced, concrete_type
                ):
                    value = dereferenced
            self.validate_type(concrete_type, value)
            bindings[name] = value
            payload.append({"name": name, "type": expected_type, "value": value})
        signal["payload"] = payload
        for parameter in parameters:
            if parameter["type"] == "TaskFlow" and parameter["name"] == "current_flow":
                candidate_flow = bindings.get(parameter["name"])
                if candidate_flow in self.systems and _matches_system_type(
                    self.model, candidate_flow, "TaskFlow"
                ):
                    signal["_effective_flow"] = candidate_flow
        if handler.get("return_type"):
            result_value = f"{handler['return_type']}Result@{signal['id']}"
            bindings["result"] = result_value
            signal["result"] = {
                "type": handler["return_type"],
                "value": result_value,
                "evidence": None,
            }
        return bindings

    def infer_result(
        self,
        *,
        signal: dict[str, Any],
        handler: dict[str, Any],
        bindings: dict[str, Any],
        effects: list[dict[str, Any]],
    ) -> None:
        if not handler.get("return_type"):
            return
        candidates: list[tuple[Any, dict[str, Any]]] = []
        for expression in effects:
            if expression["kind"] != "fact" or "return" not in expression["name"]:
                continue
            values = [
                self.value(item, signal=signal, bindings=bindings)
                for item in expression["arguments"]
            ]
            if values:
                candidates.append((values[-1], expression))
        unique = {str(value): (value, evidence) for value, evidence in candidates}
        if len(unique) != 1:
            return
        value, evidence = next(iter(unique.values()))
        self.validate_type(handler["return_type"], value)
        signal["result"] = {
            "type": handler["return_type"],
            "value": value,
            "evidence": evidence["text"],
        }
        bindings["result"] = value
        self.event(
            "action_result_resolved",
            signal_id=signal["id"],
            result_type=handler["return_type"],
            value=value,
            evidence=evidence["text"],
            span=evidence["span"],
        )

    def validate_type(self, expected: str, value: Any) -> None:
        if expected == "String" and not isinstance(value, str):
            raise DerivationProblem(f"expected String payload, got {value!r}")
        if expected == "Int" and (isinstance(value, bool) or not isinstance(value, int)):
            raise DerivationProblem(f"expected Int payload, got {value!r}")
        if expected == "Bool" and not isinstance(value, bool):
            raise DerivationProblem(f"expected Bool payload, got {value!r}")
        if expected in self.model["enums"]:
            prefix = f"{expected}::"
            if not isinstance(value, str) or not value.startswith(prefix):
                raise DerivationProblem(f"expected {expected} enum payload, got {value!r}")
            member = value[len(prefix) :]
            if member not in self.model["enums"][expected]:
                raise DerivationProblem(f"unknown {expected} enum value {member}")
        if expected == "System":
            if value not in self.systems:
                raise DerivationProblem(f"expected System reference payload, got {value!r}")
        elif (
            expected not in {"String", "Int", "Bool"}
            and expected not in self.model["enums"]
            and value in self.systems
        ):
            if not _matches_system_type(self.model, value, expected):
                raise DerivationProblem(f"system {value} is not of expected type {expected}")

    def expression_value(
        self,
        expression: dict[str, Any],
        *,
        signal: dict[str, Any],
        bindings: dict[str, Any],
        snapshot: dict[str, Any] | None = None,
    ) -> bool:
        saved = self.current
        if snapshot is not None:
            self.current = snapshot
        try:
            kind = expression["kind"]
            if kind == "any_of":
                return any(
                    self.expression_value(
                        alternative,
                        signal=signal,
                        bindings=bindings,
                    )
                    for alternative in expression["alternatives"]
                )
            if kind == "state_condition":
                target = self.resolve_path(expression["target"], signal=signal, bindings=bindings)
                return self.current["states"].get(target) == expression["state"]
            if kind == "reference_condition":
                reference = expression["reference"]
                parts = reference.split(".")
                if parts[0] == "self":
                    key = ".".join([str(self.self_value(signal)), *parts[1:]])
                elif parts[0] in bindings:
                    key = ".".join([str(bindings[parts[0]]), *parts[1:]])
                else:
                    key = reference
                expected = self.value(expression["value"], signal=signal, bindings=bindings)
                if key in self.current["references"]:
                    return self.current["references"].get(key) == expected
                rendered = self.substitute(expression["text"], signal=signal, bindings=bindings)
                comparison = self._symbolic_reference_comparison(rendered)
                if comparison is not None:
                    return comparison
                return _assertion(rendered) in self.current["facts"]
            if kind == "fact":
                values = [self.value(item, signal=signal, bindings=bindings) for item in expression["arguments"]]
                if expression["name"].endswith(("_generation_nonzero", "_occurrence_fresh")):
                    return self._builtin_fact(expression["name"], values)
                if (
                    expression["name"].endswith("_ref_generation_valid")
                    and len(values) == 1
                    and self._is_dynamic_reference(values[0])
                ):
                    return self._builtin_fact(expression["name"], values)
                return _fact(expression["name"], values) in self.current["facts"] or self._builtin_fact(
                    expression["name"], values
                ) or self._predicate_body_value(expression["name"], values, signal=signal)
            if kind == "assertion":
                rendered = self.substitute(expression["expression"], signal=signal, bindings=bindings)
                comparison = self._symbolic_reference_comparison(rendered)
                if comparison is not None:
                    return comparison
                return _assertion(rendered) in self.current["facts"]
            raise DerivationProblem(f"unsupported condition kind {kind}")
        finally:
            self.current = saved

    def _is_dynamic_reference(self, value: Any) -> bool:
        metadata = self.current.get("instances", {}).get(str(value))
        if metadata is None:
            return False
        declared_type = str(metadata.get("declared_type", ""))
        return (
            declared_type.endswith("Ref")
            or "target" in metadata
            or "target_generation" in metadata
        )

    def _symbolic_reference_comparison(self, expression: str) -> bool | None:
        comparison = re.fullmatch(
            r"([A-Za-z_][A-Za-z0-9_]*)\s*(==|!=)\s*([A-Za-z_][A-Za-z0-9_]*)",
            expression,
        )
        if comparison is None:
            return None
        left, operator, right = comparison.groups()
        left_type = self._reference_type(left)
        right_type = self._reference_type(right)
        if left_type is None or left_type != right_type:
            return None
        return left == right if operator == "==" else left != right

    def _builtin_fact(self, name: str, values: list[Any]) -> bool:
        if name.endswith("_generation_nonzero") and len(values) == 1:
            metadata = self.current.get("instances", {}).get(str(values[0]), {})
            return int(metadata.get("generation", 0)) > 0 and metadata.get("alive", True)
        if name.endswith("_occurrence_fresh") and len(values) == 1:
            identity = str(values[0])
            metadata = self.current.get("instances", {}).get(identity, {})
            return (
                int(metadata.get("generation", 0)) > 0
                and metadata.get("alive", True)
                and self.current["states"].get(identity) == "Base"
            )
        if name.endswith("_ref_generation_valid") and len(values) == 1:
            metadata = self.current.get("instances", {}).get(str(values[0]), {})
            target = metadata.get("target")
            target_metadata = self.current.get("instances", {}).get(str(target), {})
            return (
                target is not None
                and metadata.get("target_generation") == target_metadata.get("generation")
                and target_metadata.get("alive", True)
            )
        if name.endswith("_ref_targets") and len(values) == 2:
            metadata = self.current.get("instances", {}).get(str(values[0]), {})
            return metadata.get("target") == values[1]
        if name == "task_flow_cpu_ref_is" and len(values) == 2:
            return self.current["references"].get(f"{values[0]}.cpu_ref") == values[1]
        if name == "task_flow_cpu_ref_targets" and len(values) == 2:
            reference = self.current["references"].get(f"{values[0]}.cpu_ref")
            return reference is not None and self._deref(reference) == values[1]
        if name == "task_flow_cpu_ref_read_only_while_executing" and len(values) == 1:
            return self.current["references"].get(f"{values[0]}.cpu_ref") is not None
        if name == "cpu_ref_dereference_requires_published_element" and len(values) == 1:
            target = self._deref(values[0])
            return target in self.systems and target in self.current["states"]
        if name == "has_slot" and len(values) == 2:
            return self._declares_slot(values[0], values[1])
        if name == "initcall_entry_declared" and len(values) == 1:
            prefix = "InitcallEntry::"
            return (
                isinstance(values[0], str)
                and values[0].startswith(prefix)
                and values[0][len(prefix) :] in self.model.get("enums", {}).get("InitcallEntry", [])
            )
        if name == "device_ref_ready" and len(values) == 1:
            escaped = re.escape(str(values[0]))
            return any(
                re.fullmatch(rf"platform_bus_device_discovered\([^,]+,{escaped}\)", fact)
                for fact in self.current["facts"]
            )
        constant_predicates = {
            "vmap_flags_vm_ioremap": "VmapAreaFlags::VmIoremap",
            "page_protection_io_memory": "PageProtectionRef::IoMemory",
            "page_protection_kind_io_memory": "VmapPageProtectionKind::IoMemory",
        }
        if name in constant_predicates and len(values) == 1:
            return values[0] == constant_predicates[name]
        if name in {"task_state_new", "task_state_running", "task_not_enqueued"} and len(values) == 1:
            target = self._deref(values[0])
            return _fact(name, [target]) in self.current["facts"]
        if name == "task_runtime_state_transition_allowed" and len(values) == 2:
            task = self._deref(values[0])
            requested = values[1]
            if requested != "TaskRuntimeState::Running":
                return False
            return (
                _fact("task_state_new", [task]) in self.current["facts"]
                or _fact("task_state_running", [task]) in self.current["facts"]
                or _fact("task_runtime_state_is", [task, "TaskRuntimeState::New"])
                in self.current["facts"]
                or _fact("task_runtime_state_is", [task, "TaskRuntimeState::Running"])
                in self.current["facts"]
            )
        if name == "task_initial_flow_is" and len(values) == 2:
            return self.current["references"].get(f"{values[0]}.initial_flow") == values[1]
        if name in {"task_owns_flow", "task_flow_owner_is", "task_flow_parent_is"} and len(values) == 2:
            task, flow = (values[0], values[1]) if name == "task_owns_flow" else (values[1], values[0])
            return (
                flow in self.systems
                and self.systems[flow].get("parent") == task
                and self.current["references"].get(f"{task}.initial_flow") == flow
            )
        if name == "task_flow_initial_binding_consistent" and len(values) == 1:
            flow = values[0]
            if flow not in self.systems:
                return False
            task = self.systems[flow].get("parent")
            return task in self.systems and self.current["references"].get(f"{task}.initial_flow") == flow
        if name == "task_flow_start_binding_consistent" and len(values) == 1:
            flow = values[0]
            if flow not in self.systems:
                return False
            task = self.systems[flow].get("parent")
            if task not in self.systems:
                return False
            if self.current["references"].get(f"{task}.initial_flow") == flow:
                return True
            return (
                _fact("task_owns_flow", [task, flow]) in self.current["facts"]
                and _fact("task_flow_owner_is", [flow, task]) in self.current["facts"]
                and _fact("task_flow_parent_is", [flow, task]) in self.current["facts"]
            )
        return False

    def _predicate_path(self, path: str, bindings: dict[str, Any]) -> Any:
        parts = path.strip().split(".")
        value: Any = bindings.get(parts[0], parts[0])
        for field in parts[1:]:
            if field == "parent" and value in self.systems:
                value = self.systems[value].get("parent")
                continue
            if value in self.systems and field in self.systems[value].get("properties", {}):
                value = self.systems[value]["properties"][field]
                continue
            key = f"{value}.{field}"
            if key in self.current["references"]:
                value = self.current["references"][key]
                continue
            return key
        return value

    @staticmethod
    def _predicate_render(statement: str, bindings: dict[str, Any]) -> str:
        rendered = statement
        for parameter, value in sorted(bindings.items(), key=lambda item: -len(item[0])):
            rendered = re.sub(rf"\b{re.escape(parameter)}\b", str(value), rendered)
        return " ".join(rendered.split())

    def _predicate_body_value(
        self, name: str, values: list[Any], *, signal: dict[str, Any]
    ) -> bool:
        declarations = [
            declaration
            for declaration in self.model.get("predicates", {}).get(name, [])
            if declaration.get("body") is not None
            and len(declaration.get("parameters", [])) == len(values)
        ]
        if not declarations:
            return False
        key = (name, tuple(str(value) for value in values))
        if key in self.active_predicates:
            raise DerivationProblem(f"recursive predicate body without progress: {name}")
        self.active_predicates.add(key)
        try:
            for declaration in declarations:
                local = {
                    parameter["name"]: value
                    for parameter, value in zip(declaration["parameters"], values)
                }
                body = " ".join(declaration["body"].split())
                # Quantified collection predicates in the current model are
                # proof obligations over symbolic fields/ranges.  They are
                # true only when the exact predicate fact was already present
                # (checked by expression_value before entering this method).
                # Returning false lets a target-state invariant establish its
                # own declared fact without pretending to enumerate values.
                if re.fullmatch(r"forall\s+.+?\s+in\s+.+?\s*\{.*\}", body, re.S):
                    continue
                okay = True
                for raw in declaration["body"].split(";"):
                    statement = " ".join(raw.split())
                    if not statement:
                        continue
                    state = re.fullmatch(
                        r"([A-Za-z_][A-Za-z0-9_.]*)\.state == State::([A-Za-z_][A-Za-z0-9_]*)",
                        statement,
                    )
                    if state is not None:
                        target = self._predicate_path(state.group(1), local)
                        if self.current["states"].get(target) != state.group(2):
                            okay = False
                            break
                        continue
                    comparison = re.fullmatch(
                        r"([A-Za-z_][A-Za-z0-9_.]*)\s*(==|!=|<=|>=|<|>)\s*(.+)",
                        statement,
                        re.S,
                    )
                    if comparison is not None:
                        left = self._predicate_path(comparison.group(1), local)
                        right_text = comparison.group(3).strip()
                        right = self._predicate_path(right_text, local)
                        operator = comparison.group(2)
                        if operator == "==" and left == right:
                            continue
                        rendered = self._predicate_render(statement, local)
                        if _assertion(rendered) in self.current["facts"]:
                            continue
                        okay = False
                        break
                    fact = re.fullmatch(
                        r"([A-Za-z_][A-Za-z0-9_]*)\((.*)\)", statement, re.S
                    )
                    if fact is not None:
                        arguments = [
                            self._predicate_path(argument, local)
                            for argument in _split_fact_arguments(fact.group(2))
                        ]
                        if not (
                            _fact(fact.group(1), arguments) in self.current["facts"]
                            or self._builtin_fact(fact.group(1), arguments)
                            or self._predicate_body_value(
                                fact.group(1), arguments, signal=signal
                            )
                        ):
                            okay = False
                            break
                        continue
                    raise DerivationProblem(
                        f"unsupported predicate body expression in {name}: {statement}"
                    )
                if okay:
                    return True
            return False
        finally:
            self.active_predicates.discard(key)

    def _intrinsic_invariant(self, expression: dict[str, Any]) -> bool:
        if expression["kind"] == "fact":
            name = expression["name"]
            return bool(self.model.get("predicates", {}).get(name, [])) or name.startswith(
                ("valid_", "inside", "aligned", "page_aligned", "exists")
            )
        if expression["kind"] in {"assertion", "reference_condition"}:
            text = expression.get("text", expression.get("expression", ""))
            return ".state" not in text
        return False

    def apply_effect(
        self,
        expression: dict[str, Any],
        *,
        signal: dict[str, Any],
        bindings: dict[str, Any],
        candidate: dict[str, Any],
        handler: dict[str, Any],
    ) -> None:
        kind = expression["kind"]
        if kind == "any_of":
            if any(
                self.expression_value(
                    alternative,
                    signal=signal,
                    bindings=bindings,
                    snapshot=candidate,
                )
                for alternative in expression["alternatives"]
            ):
                return
            self.apply_effect(
                expression["alternatives"][0],
                signal=signal,
                bindings=bindings,
                candidate=candidate,
                handler=handler,
            )
            return
        if kind == "fact":
            values = [self.value(item, signal=signal, bindings=bindings) for item in expression["arguments"]]
            if expression["name"].endswith("_ref_targets") and len(values) == 2:
                prefix = f"{expression['name']}({_fact_argument(values[0])},"
                candidate["facts"] = [
                    fact for fact in candidate["facts"] if not fact.startswith(prefix)
                ]
            if expression["name"] == "task_runtime_state_is" and len(values) == 2:
                task = self._deref(values[0])
                candidate["facts"] = [
                    fact
                    for fact in candidate["facts"]
                    if not fact.startswith(f"task_runtime_state_is({task},")
                    and fact not in {
                        _fact("task_state_new", [task]),
                        _fact("task_state_running", [task]),
                    }
                ]
                values[0] = task
            candidate["facts"].append(_fact(expression["name"], values))
            if expression["name"] == "task_runtime_state_is" and len(values) == 2:
                aliases = {
                    "TaskRuntimeState::New": "task_state_new",
                    "TaskRuntimeState::Running": "task_state_running",
                }
                alias = aliases.get(values[1])
                if alias is not None:
                    candidate["facts"].append(_fact(alias, [values[0]]))
            candidate["facts"] = sorted(set(candidate["facts"]))
            return
        if kind == "assertion":
            rendered = self.substitute(expression["expression"], signal=signal, bindings=bindings)
            candidate["facts"].append(_assertion(rendered))
            candidate["facts"] = sorted(set(candidate["facts"]))
            return
        if kind == "reference_condition":
            rendered = self.substitute(expression["text"], signal=signal, bindings=bindings)
            candidate["facts"].append(_assertion(rendered))
            candidate["facts"] = sorted(set(candidate["facts"]))
            return
        if kind == "reference_assignment":
            parts = expression["reference"].split(".")
            if parts[0] == "self":
                key = ".".join([str(self.self_value(signal)), *parts[1:]])
            elif parts[0] in bindings:
                key = ".".join([str(bindings[parts[0]]), *parts[1:]])
            else:
                key = expression["reference"]
            owner, field = key.split(".", 1)
            if owner not in self.systems or field not in self.systems[owner]["reference_types"]:
                raise DerivationProblem(f"unknown reference update {key}")
            value = self.value(expression["value"], signal=signal, bindings=bindings)
            expected = self.systems[owner]["reference_types"][field]
            if value in self.systems and not _matches_system_type(self.model, value, expected):
                raise DerivationProblem(f"reference update {key} expects {expected}, got {value}")
            if value not in self.systems and self._reference_type(value) != expected:
                raise DerivationProblem(f"reference update {key} expects {expected}, got {value}")
            candidate["references"][key] = value
            return
        if kind == "state_assignment":
            target = self.resolve_path(expression["target"], signal=signal, bindings=bindings)
            if handler["kind"] == "Action":
                raise DerivationProblem("Action handler cannot assign lifecycle state")
            if target != signal["target"] or expression["state"] != handler["target_state"]:
                raise DerivationProblem("state update must match the owner Transition target state")
            return
        if kind == "state_condition":
            target = self.resolve_path(expression["target"], signal=signal, bindings=bindings)
            if candidate["states"].get(target) != expression["state"]:
                raise DerivationProblem(f"postcondition_not_satisfied: {expression['text']}")
            return
        raise DerivationProblem(f"unsupported effect kind {kind}")

    def _handler_conditions(self, handler: dict[str, Any]) -> list[dict[str, Any]]:
        return [
            entry
            for block in handler["body"]
            if block["kind"] == "depends_on"
            for entry in block["entries"]
        ]

    def _condition_proof_source(
        self,
        condition: dict[str, Any],
        *,
        signal: dict[str, Any],
        bindings: dict[str, Any],
    ) -> str:
        if condition["kind"] != "fact" or condition["name"] != "has_slot":
            return "snapshot"
        values = [
            self.value(item, signal=signal, bindings=bindings)
            for item in condition["arguments"]
        ]
        if _fact(condition["name"], values) in self.current["facts"]:
            return "snapshot"
        if len(values) == 2 and self._declares_slot(values[0], values[1]):
            return "model_structure"
        return "snapshot"

    def _check_conditions(
        self,
        entries: list[dict[str, Any]],
        *,
        signal: dict[str, Any],
        bindings: dict[str, Any],
        context: str | None = None,
    ) -> tuple[bool, str | None]:
        for condition in entries:
            result = self.expression_value(condition, signal=signal, bindings=bindings)
            self.event(
                "condition_checked",
                signal_id=signal["id"],
                expression=condition["text"],
                result=result,
                context=context,
                proof_source=self._condition_proof_source(
                    condition, signal=signal, bindings=bindings
                ),
            )
            if not result:
                return False, condition["text"]
        return True, None

    def _execute_call(
        self,
        call: dict[str, Any],
        *,
        signal: dict[str, Any],
        bindings: dict[str, Any],
        context_stack: list[str],
    ) -> None:
        if call.get("kind") == "choice":
            rejected: list[str] = []
            selected: dict[str, Any] | None = None
            for index, candidate in enumerate(call["choices"]):
                okay, reason = self._call_acceptable(
                    candidate,
                    signal=signal,
                    bindings=bindings,
                )
                self.event(
                    "drives_choice_checked",
                    signal_id=signal["id"],
                    candidate=index,
                    expression=candidate["text"],
                    result=okay,
                    reason=reason,
                    span=candidate["span"],
                )
                if okay:
                    selected = candidate
                    self.event(
                        "drives_choice_selected",
                        signal_id=signal["id"],
                        candidate=index,
                        expression=candidate["text"],
                    )
                    break
                rejected.append(f"{candidate['text']}: {reason}")
            if selected is None:
                raise DerivationProblem(
                    "no acceptable drives alternative: " + "; ".join(rejected)
                )
            self._execute_call(
                selected,
                signal=signal,
                bindings=bindings,
                context_stack=context_stack,
            )
            return
        if call.get("kind") == "declare_indexed":
            signal["_indexed_transaction"] = True
            owner = self.resolve_receiver(
                call["owner_receiver"], signal=signal, bindings=bindings
            )
            if owner != signal["target"]:
                raise DerivationProblem(
                    "indexed-owned elements may be declared only by their owner handler: "
                    f"{signal['target']} cannot publish {owner}.{call['field']}"
                )
            field = self._field_in_groups(
                self.systems[owner].get("fields", {}), call["field"]
            )
            if field is None or not field.get("indexed"):
                raise DerivationProblem(
                    f"{owner}.{call['field']} is not an indexed-owned collection"
                )
            if field.get("type") != call["declared_type"]:
                raise DerivationProblem(
                    f"indexed declaration {owner}.{call['field']} expects "
                    f"{field.get('type')}, got {call['declared_type']}"
                )
            key = self.value(call["key"], signal=signal, bindings=bindings)
            key_type = field.get("key_type")
            if key_type in {"LogicId", "Int", "usize", "u32", "u16", "u8"} and (
                isinstance(key, bool) or not isinstance(key, int) or key < 0
            ):
                raise DerivationProblem(
                    f"indexed declaration key for {owner}.{call['field']} expects {key_type}"
                )
            identity = f"{owner}.{call['field']}[{key}]"
            reference_key = identity
            if reference_key in self.current["references"] or identity in self.current["instances"]:
                raise DerivationProblem(f"duplicate indexed key {identity}")
            metadata = {
                "declared_type": call["declared_type"],
                "parent": owner,
                "indexed": True,
                "owned_field": call["field"],
                "key": key,
                "span": deepcopy(call["span"]),
            }
            self.current["instances"][identity] = metadata
            self.current["references"][reference_key] = identity
            self._install_runtime_system(identity, metadata)
            self._activate_static_system(identity, signal=signal)
            self.event(
                "indexed_instance_declared",
                signal_id=signal["id"],
                identity=identity,
                owner=owner,
                field=call["field"],
                key=key,
                declared_type=call["declared_type"],
                context=list(context_stack),
                span=call["span"],
                snapshot=_snapshot(self.current),
            )
            return
        if call.get("kind") == "declare":
            signal["_dynamic_transaction"] = True
            identity = (
                f"dynamic:{signal['id']}:{self.next_instance}:{call['alias']}"
            )
            self.next_instance += 1
            metadata = {
                "declared_type": call["declared_type"],
                "parent": signal["target"],
                "indexed": False,
                "alias": call["alias"],
                "generation": self.next_generation,
                "alive": True,
                "span": deepcopy(call["span"]),
            }
            self.next_generation += 1
            self.current["instances"][identity] = metadata
            self._install_runtime_system(identity, metadata)
            self._activate_static_system(identity, signal=signal)
            bindings[call["alias"]] = identity
            self.event(
                "dynamic_declared",
                signal_id=signal["id"],
                alias=call["alias"],
                declared_type=call["declared_type"],
                identity=identity,
                context=list(context_stack),
                span=call["span"],
                snapshot=_snapshot(self.current),
            )
            return
        if call.get("kind") != "call":
            raise DerivationProblem(
                f"invalid process call at {call.get('span', {}).get('source_file')}:{call.get('span', {}).get('start_line')}: "
                f"{call.get('text')}"
            )

        signal.pop("_selector_resolutions", None)
        target = self.resolve_receiver(call["receiver"], signal=signal, bindings=bindings)
        receiver_value = self.value(
            {"kind": "path", "value": call["receiver"]},
            signal=signal,
            bindings=bindings,
        )
        raw_arguments = self.materialize_arguments(
            call["arguments"], signal=signal, bindings=bindings
        )
        selector_resolutions = signal.pop("_selector_resolutions", [])
        coordinate = self.coordinate(signal["target"], target, signal["coordinate"])
        child = self.new_signal(
            source=signal["target"],
            target=target,
            name=call["name"],
            raw_arguments=raw_arguments,
            delivery="drives",
            cause_id=signal["id"],
            coordinate=coordinate,
            compat_process_kind=call["process_kind"],
            call_span=call.get("span"),
        )
        effective_flow = self._effective_flow(signal)
        if effective_flow is not None:
            child["_effective_flow"] = effective_flow
        if selector_resolutions:
            child["selector_resolutions"] = deepcopy(selector_resolutions)
        child["contexts"] = list(context_stack)
        if receiver_value != target:
            child["_receiver_value"] = receiver_value
        self.event("drives_wait_started", signal_id=signal["id"], child_id=child["id"])
        self.deliver(child)
        # The callee does not wait for its emits.  Once its response boundary is
        # committed, the global scheduler may service the FIFO before waking
        # the synchronous caller; this preserves post-commit delivery while
        # making completion facts visible at the caller's next statement.
        self._drain_queue()
        self.event(
            "drives_wait_finished",
            signal_id=signal["id"],
            child_id=child["id"],
            child_outcome=child["outcome"],
        )
        if child["outcome"] == "truncated":
            return
        if child["outcome"] != "completed":
            raise DerivationProblem(f"strict child {child['id']} did not complete")
        alias = call.get("result_alias")
        if alias:
            result = child.get("result")
            if result is None or result.get("evidence") is None:
                raise DerivationProblem(
                    f"action {child['handler']['id']} has no unique result evidence"
                )
            bindings[alias] = result["value"]
            self.event(
                "action_result_bound",
                signal_id=signal["id"],
                child_id=child["id"],
                alias=alias,
                type=call.get("result_type"),
                value=result["value"],
            )

    @staticmethod
    def _handler_property(handler: dict[str, Any], name: str) -> str | None:
        for member in handler.get("body", []):
            if member.get("kind") == "property" and member.get("name") == name:
                return str(member.get("value"))
        return None

    def _bind_generation_reference(
        self,
        *,
        reference: str,
        target: str,
        candidate: dict[str, Any],
    ) -> None:
        reference_metadata = candidate.get("instances", {}).get(reference)
        target_metadata = candidate.get("instances", {}).get(target)
        if reference_metadata is None or target_metadata is None:
            raise DerivationProblem("generation binding requires two dynamic instances")
        existing = reference_metadata.get("target")
        if existing is not None and existing != target:
            raise DerivationProblem(f"generation reference {reference} is already bound")
        generation = int(target_metadata.get("generation", 0))
        if generation == 0 or not target_metadata.get("alive", True):
            raise DerivationProblem(f"generation reference target {target} is not live")
        reference_metadata["target"] = target
        reference_metadata["target_generation"] = generation
        candidate["references"][f"{reference}.target"] = target

    def _apply_occurrence_metadata(
        self,
        *,
        handler: dict[str, Any],
        signal: dict[str, Any],
        bindings: dict[str, Any],
        candidate: dict[str, Any],
    ) -> None:
        identity = signal["target"]
        metadata = candidate.get("instances", {}).get(identity)
        if metadata is None:
            return

        declared_type = str(metadata.get("declared_type", ""))
        if handler.get("name") == "Bind" and declared_type.endswith("Ref"):
            target = next(
                (
                    bindings.get(parameter["name"])
                    for parameter in handler.get("parameters", [])
                    if str(bindings.get(parameter["name"])) in candidate.get("instances", {})
                    and not parameter["type"].endswith("Ref")
                ),
                None,
            )
            if target is not None:
                self._bind_generation_reference(
                    reference=identity,
                    target=str(target),
                    candidate=candidate,
                )

        if self._handler_property(handler, "structural_binding") == "true":
            parent = next(
                (
                    bindings.get(parameter["name"])
                    for parameter in handler.get("parameters", [])
                    if not parameter["type"].endswith("Ref")
                    and str(bindings.get(parameter["name"])) in self.systems
                ),
                None,
            )
            if parent is None:
                raise DerivationProblem(f"structural Bind for {identity} has no parent")
            bound_parent = metadata.get("bound_parent")
            if bound_parent is not None and bound_parent != parent:
                raise DerivationProblem(f"structural parent for {identity} is immutable")
            metadata["parent"] = str(parent)
            metadata["bound_parent"] = str(parent)
            self.systems[identity]["parent"] = str(parent)
            for parameter in handler.get("parameters", []):
                if not parameter["type"].endswith("Ref"):
                    continue
                reference = bindings.get(parameter["name"])
                if str(reference) in candidate.get("instances", {}):
                    self._bind_generation_reference(
                        reference=str(reference),
                        target=identity,
                        candidate=candidate,
                    )

        if (
            handler.get("kind") == "Transition"
            and handler.get("name") == "Cleanup"
            and handler.get("target_state") == "Destroyed"
        ):
            metadata["alive"] = False

    def _call_acceptable(
        self,
        call: dict[str, Any],
        *,
        signal: dict[str, Any],
        bindings: dict[str, Any],
    ) -> tuple[bool, str | None]:
        try:
            target = self.resolve_receiver(call["receiver"], signal=signal, bindings=bindings)
            receiver_value = self.value(
                {"kind": "path", "value": call["receiver"]},
                signal=signal,
                bindings=bindings,
            )
            raw_arguments = self.materialize_arguments(
                call["arguments"], signal=signal, bindings=bindings
            )
            probe: dict[str, Any] = {
                "id": f"choice@{signal['id']}",
                "target": target,
                "_raw_arguments": raw_arguments,
                "payload": [],
            }
            system = self.systems[target]
            candidates = system["handlers_by_name"].get(call["name"], [])
            if not candidates and receiver_value != target:
                reference_type = self._reference_type(receiver_value)
                if reference_type is not None:
                    candidates = [
                        item
                        for item in self.model["types"][reference_type].get("processes", [])
                        if item["name"] == call["name"]
                    ]
                    if candidates:
                        probe["_self_value"] = receiver_value
            if not candidates:
                return False, "no_handler"
            if len(candidates) != 1:
                return False, "ambiguous_handler"
            handler = candidates[0]
            local = self.bind_payload(probe, handler)
            actual_state = self.current["states"].get(target)
            if handler.get("source_state") is not None and actual_state != handler["source_state"]:
                return (
                    False,
                    f"state_not_accepted: expected State::{handler['source_state']}, got State::{actual_state}",
                )
            for condition in self._handler_conditions(handler):
                if not self.expression_value(condition, signal=probe, bindings=local):
                    return False, f"condition_not_satisfied: {condition['text']}"
            return True, None
        except (DerivationProblem, KeyError) as exc:
            return False, str(exc)

    def _drain_queue(self) -> None:
        while self.queue:
            queued = self.queue.pop(0)
            self.event("emits_dequeued", signal_id=queued["id"], remaining=len(self.queue))
            self.deliver(queued)

    def _execute_members(
        self,
        members: list[dict[str, Any]],
        *,
        signal: dict[str, Any],
        handler: dict[str, Any],
        bindings: dict[str, Any],
        pending_emits: list[dict[str, Any]],
        pending_effects: list[dict[str, Any]],
        context_stack: list[str],
        skip_outer_depends: bool = False,
    ) -> None:
        for member in members:
            kind = member["kind"]
            if kind == "depends_on":
                if skip_outer_depends:
                    continue
                okay, failed = self._check_conditions(
                    member["entries"],
                    signal=signal,
                    bindings=bindings,
                    context=context_stack[-1] if context_stack else None,
                )
                if not okay:
                    raise DerivationProblem(f"condition_not_satisfied: {failed}")
            elif kind == "drives":
                for call in member["entries"]:
                    self._execute_call(
                        call,
                        signal=signal,
                        bindings=bindings,
                        context_stack=context_stack,
                    )
            elif kind == "emits":
                pending_emits.extend(member["entries"])
            elif kind in {"ensures", "updates"}:
                pending_effects.extend(member["entries"])
            elif kind == "within":
                context = member["context"]
                if context not in self.model.get("contexts", {}):
                    raise DerivationProblem(f"unknown context {context}")
                context_stack.append(context)
                self.event(
                    "context_entered",
                    signal_id=signal["id"],
                    context=context,
                    stack=list(context_stack),
                    span=member["span"],
                )
                context_effects: list[dict[str, Any]] = []
                self._execute_members(
                    member["members"],
                    signal=signal,
                    handler=handler,
                    bindings=bindings,
                    pending_emits=pending_emits,
                    pending_effects=context_effects,
                    context_stack=context_stack,
                )
                if context_effects:
                    candidate = _snapshot(self.current)
                    for expression in context_effects:
                        self.apply_effect(
                            expression,
                            signal=signal,
                            bindings=bindings,
                            candidate=candidate,
                            handler=handler,
                        )
                    self.current = _snapshot(candidate)
                    self.event(
                        "context_effects_committed",
                        signal_id=signal["id"],
                        context=context,
                        expressions=[item["text"] for item in context_effects],
                    )
                self.event(
                    "context_exited",
                    signal_id=signal["id"],
                    context=context,
                    stack=list(context_stack),
                    span=member["span"],
                )
                context_stack.pop()
            elif kind in {"deferred", "trimmed"}:
                evidence = member.get("evidence", [])
                for expression in evidence:
                    known = expression["kind"] in {"fact", "assertion", "state_condition", "reference_condition"}
                    self.event(
                        "boundary_evidence_checked",
                        signal_id=signal["id"],
                        boundary_id=member.get("id"),
                        status=kind,
                        expression=expression["text"],
                        result=known,
                        span=expression["span"],
                    )
                    if not known:
                        raise DerivationProblem(
                            f"boundary_evidence_unverifiable: {member.get('id')}: {expression['text']}"
                        )
            elif kind == "result":
                success = next(
                    (item for item in member.get("variants", []) if item.get("name") == "Success"),
                    None,
                )
                if success is not None:
                    self._execute_members(
                        success.get("members", []),
                        signal=signal,
                        handler=handler,
                        bindings=bindings,
                        pending_emits=pending_emits,
                        pending_effects=pending_effects,
                        context_stack=context_stack,
                    )
            elif kind in {"may_change", "transitions", "property"}:
                self.event(
                    "model_member_observed",
                    signal_id=signal["id"],
                    member_kind=kind,
                    context=list(context_stack),
                )

    def _failure_chain(self) -> list[str]:
        chain: list[str] = []
        current = self.failure_signal_id
        while current is not None:
            chain.append(current)
            current = self.signal_index[current]["cause_id"]
        return list(reversed(chain))

    def deliver(self, signal: dict[str, Any]) -> None:
        signal["before_snapshot"] = _snapshot(self.current)
        if self.over_budget(signal):
            signal["outcome"] = "truncated"
            signal["reason"] = "propagation_budget_exceeded"
            signal["after_snapshot"] = _snapshot(self.current)
            self.was_bounded = True
            frontier = {
                "signal_id": signal["id"],
                "target": signal["target"],
                "signal": signal["name"],
                "coordinate": signal["coordinate"],
                "max_depth": self.max_depth,
                "max_breadth": self.max_breadth,
            }
            self.frontier.append(frontier)
            self.event("signal_truncated", **frontier)
            return
        self.event("signal_received", signal_id=signal["id"], target=signal["target"])
        if signal["target"] not in self.systems:
            self.reject(signal, f"unknown target system {signal['target']}")
            return
        request_key = (
            signal["target"],
            signal["name"],
            json.dumps(signal["before_snapshot"], sort_keys=True, separators=(",", ":")).encode("utf-8"),
        )
        if request_key in self.active_requests:
            self.fail(signal, "causal_cycle_without_snapshot_progress")
            return
        self.active_requests.add(request_key)
        system = self.systems[signal["target"]]
        candidates = system["handlers_by_name"].get(signal["name"], [])
        if not candidates and signal.get("_receiver_value") is not None:
            reference_type = self._reference_type(signal["_receiver_value"])
            if reference_type is not None:
                candidates = [
                    item
                    for item in self.model["types"][reference_type].get("processes", [])
                    if item["name"] == signal["name"]
                ]
                if candidates:
                    signal["_self_value"] = signal["_receiver_value"]
        if not candidates:
            self.reject(signal, "no_handler")
            self.active_requests.discard(request_key)
            return
        if len(candidates) != 1:
            self.reject(signal, "ambiguous_handler")
            self.active_requests.discard(request_key)
            return
        handler = candidates[0]
        signal["handler"] = {
            "id": handler["id"],
            "kind": handler["kind"],
            "source_state": handler["source_state"],
            "target_state": handler["target_state"],
            "span": handler["span"],
            "composed_type_processes": deepcopy(
                handler.get("composed_type_processes", [])
            ),
        }
        if handler.get("description") is not None:
            signal["handler"]["description"] = handler["description"]
        try:
            bindings = self.bind_payload(signal, handler)
        except DerivationProblem as exc:
            self.fail(signal, f"payload_error: {exc}")
            self.active_requests.discard(request_key)
            return
        actual_state = self.current["states"].get(signal["target"])
        if handler.get("source_state") is not None and actual_state != handler["source_state"]:
            self.reject(
                signal,
                f"state_not_accepted: expected State::{handler['source_state']}, got State::{actual_state}",
            )
            self.active_requests.discard(request_key)
            return
        try:
            okay, failed_condition = self._check_conditions(
                self._handler_conditions(handler), signal=signal, bindings=bindings
            )
        except DerivationProblem as exc:
            self.fail(signal, f"condition_error: {exc}")
            self.active_requests.discard(request_key)
            return
        if not okay:
            self.reject(signal, f"condition_not_satisfied: {failed_condition}")
            self.active_requests.discard(request_key)
            return
        signal["outcome"] = "accepted"
        self.event("response_started", signal_id=signal["id"], handler=handler["id"])
        pending_emits: list[dict[str, Any]] = []
        pending_effects: list[dict[str, Any]] = []
        transaction_snapshot = _snapshot(self.current)
        transaction_system_names = set(self.systems)
        transaction_next_instance = self.next_instance
        transaction_next_generation = self.next_generation
        transaction_parents = {
            name: system.get("parent") for name, system in self.systems.items()
        }
        try:
            self._execute_members(
                handler["body"],
                signal=signal,
                handler=handler,
                bindings=bindings,
                pending_emits=pending_emits,
                pending_effects=pending_effects,
                context_stack=[],
                skip_outer_depends=True,
            )
            candidate = _snapshot(self.current)
            if handler["kind"] == "Transition" and handler.get("target_state") is not None:
                candidate["states"][signal["target"]] = handler["target_state"]
            for expression in pending_effects:
                self.apply_effect(
                    expression,
                    signal=signal,
                    bindings=bindings,
                    candidate=candidate,
                    handler=handler,
                )
            self._apply_occurrence_metadata(
                handler=handler,
                signal=signal,
                bindings=bindings,
                candidate=candidate,
            )
            self.infer_result(
                signal=signal,
                handler=handler,
                bindings=bindings,
                effects=pending_effects,
            )
            if handler["kind"] == "Transition" and handler.get("target_state") is not None:
                target_state = system["states"][handler["target_state"]]
                for invariant in target_state["invariant"]:
                    result = self.expression_value(
                        invariant, signal=signal, bindings=bindings, snapshot=candidate
                    )
                    proof_source = "snapshot"
                    if not result and self._intrinsic_invariant(invariant):
                        result = True
                        proof_source = "predicate_body_or_static_attribute"
                        self.apply_effect(
                            invariant,
                            signal=signal,
                            bindings=bindings,
                            candidate=candidate,
                            handler=handler,
                        )
                    self.event(
                        "invariant_checked",
                        signal_id=signal["id"],
                        expression=invariant["text"],
                        result=result,
                        proof_source=proof_source,
                    )
                    if not result:
                        raise DerivationProblem(f"invariant_not_satisfied: {invariant['text']}")
            self.current = _snapshot(candidate)
            signal["after_snapshot"] = _snapshot(self.current)
            signal["outcome"] = "completed"
            self.event(
                "response_completed",
                signal_id=signal["id"],
                before=signal["before_snapshot"],
                after=signal["after_snapshot"],
            )
            for call in pending_emits:
                if call.get("kind") != "call":
                    raise DerivationProblem(f"invalid emits call: {call.get('text')}")
                target = self.resolve_receiver(call["receiver"], signal=signal, bindings=bindings)
                coordinate = self.coordinate(signal["target"], target, signal["coordinate"])
                child = self.new_signal(
                    source=signal["target"],
                    target=target,
                    name=call["name"],
                    raw_arguments=self.materialize_arguments(
                        call["arguments"], signal=signal, bindings=bindings
                    ),
                    delivery="emits",
                    cause_id=signal["id"],
                    coordinate=coordinate,
                    compat_process_kind=call["process_kind"],
                    call_span=call.get("span"),
                    fifo_position=len(self.queue) + 1,
                )
                self.queue.append(child)
                self.event(
                    "emits_enqueued",
                    signal_id=signal["id"],
                    child_id=child["id"],
                    fifo_position=len(self.queue),
                )
        except UntilReached:
            if signal["outcome"] != "completed":
                signal["outcome"] = "stopped"
                signal["reason"] = "until_signal_reached"
                signal["after_snapshot"] = _snapshot(self.current)
                self.event(
                    "response_stopped",
                    signal_id=signal["id"],
                    reason="until_signal_reached",
                )
            raise
        except DerivationProblem as exc:
            if signal.get("_indexed_transaction") or signal.get("_dynamic_transaction"):
                self.current = transaction_snapshot
                for name in set(self.systems) - transaction_system_names:
                    del self.systems[name]
                for name, parent in transaction_parents.items():
                    if name in self.systems:
                        self.systems[name]["parent"] = parent
                self.next_instance = transaction_next_instance
                self.next_generation = transaction_next_generation
                self.event(
                    "indexed_transaction_rolled_back"
                    if signal.get("_indexed_transaction")
                    else "dynamic_transaction_rolled_back",
                    signal_id=signal["id"],
                    reason=str(exc),
                    snapshot=_snapshot(self.current),
                )
            if signal["outcome"] not in {"failed", "rejected"}:
                self.fail(signal, str(exc))
        finally:
            self.active_requests.discard(request_key)

    def run(self) -> dict[str, Any]:
        root: dict[str, Any] | None = None
        try:
            diagnostics = self.document.get("diagnostics", [])
            coordinate = {
                "depth": 0,
                "breadth": 0,
                "movement": {"up": 0, "across": 0, "down": 0},
                "_budget_depth": 0,
                "_budget_breadth": 0,
            }
            if self.orchestration is None:
                root = self.new_signal(
                    source=self.source,
                    target=self.root_target,
                    name=self.root_name,
                    raw_arguments=[],
                    delivery="root",
                    cause_id=None,
                    coordinate=coordinate,
                    compat_process_kind=None,
                    call_span=None,
                )
                if diagnostics:
                    root["before_snapshot"] = _snapshot(self.current)
                    self.fail(root, "model_has_errors_or_unsupported_syntax")
                else:
                    self.deliver(root)
                self._drain_queue()
            else:
                calls = [
                    *(('drives', call) for call in self.orchestration["drives"]),
                    *(('emits', call) for call in self.orchestration["emits"]),
                ]
                for delivery, call in calls:
                    if delivery == "emits" and (
                        self.failed or self.was_bounded or root is None
                    ):
                        break
                    signal = self.new_signal(
                        source=self.orchestration["name"],
                        target=call["receiver"],
                        name=call["name"],
                        raw_arguments=call["arguments"],
                        delivery=delivery,
                        cause_id=None,
                        coordinate=coordinate,
                        compat_process_kind=call["process_kind"],
                        call_span=call.get("span"),
                        fifo_position=(len(self.queue) + 1) if delivery == "emits" else None,
                    )
                    if root is None:
                        root = signal
                    if diagnostics:
                        signal["before_snapshot"] = _snapshot(self.current)
                        self.fail(signal, "model_has_errors_or_unsupported_syntax")
                        break
                    if delivery == "drives":
                        self.deliver(signal)
                        if signal["outcome"] != "completed":
                            break
                    else:
                        self.queue.append(signal)
                        self.event(
                            "emits_enqueued",
                            signal_id=None,
                            child_id=signal["id"],
                            fifo_position=len(self.queue),
                        )
                if not self.failed and not self.was_bounded:
                    self._drain_queue()
        except UntilReached:
            pass
        if self.failed:
            verdict = "failed"
        elif self.was_bounded:
            verdict = "bounded"
        elif self.boundary is not None:
            verdict = "reached"
        elif self.until_target is not None:
            verdict = "until_signal_not_reached"
        else:
            verdict = "complete"
        for signal in self.signals:
            signal.pop("_raw_arguments", None)
            signal.pop("_budget_depth", None)
            signal.pop("_budget_breadth", None)
            signal.pop("_receiver_value", None)
            signal.pop("_self_value", None)
            signal.pop("_effective_flow", None)
            signal.pop("_selector_resolutions", None)
            signal.pop("_indexed_transaction", None)
        return {
            "root_request": {
                "source": self.source,
                "target": self.root_target,
                "signal": self.root_name,
                "signal_id": None if root is None else root["id"],
            },
            "until_request": None
            if self.until_target is None
            else {
                "target": self.until_target,
                "signal": self.until_name,
                "normalized_signal": f"{self.until_target}.{self.until_name}",
            },
            "boundary": deepcopy(self.boundary),
            "model_fingerprint": self.document["model_fingerprint"],
            "budget": {"max_depth": self.max_depth, "max_breadth": self.max_breadth},
            "verdict": verdict,
            "initial_snapshot": self.initial_snapshot,
            "last_stable_snapshot": _snapshot(self.current),
            "signals": self.signals,
            "events": self.events,
            "truncated_frontier": self.frontier,
            "failure": None
            if self.failure_signal_id is None
            else {
                "signal_id": self.failure_signal_id,
                "reason": self.failure_reason,
                "chain": self._failure_chain(),
            },
            "summary": {
                "signals": len(self.signals),
                "completed": sum(item["outcome"] == "completed" for item in self.signals),
                "rejected": sum(item["outcome"] == "rejected" for item in self.signals),
                "failed": sum(item["outcome"] == "failed" for item in self.signals),
                "truncated": sum(item["outcome"] == "truncated" for item in self.signals),
                "stopped": sum(item["outcome"] == "stopped" for item in self.signals),
                "pending": 0,
            },
        }


def initial_snapshot(model: dict[str, Any]) -> dict[str, Any]:
    result: dict[str, Any] = {
        "states": {},
        "facts": [],
        "references": {},
        "instances": {},
    }

    def initial_value(item: dict[str, Any], owner: str) -> Any:
        if item["kind"] == "path":
            value = item["value"]
            if value == "self":
                return owner
            if value.startswith("self."):
                return f"{owner}.{value[len('self.') :]}"
            if "." not in value and value and value[0].islower():
                return f"{owner}.{value}"
            return value
        if item["kind"] == "enum":
            return f"{item['type']}::{item['value']}"
        return item.get("value")

    def add_predicate(name: str, arguments: list[Any], visited: set[tuple[str, tuple[str, ...]]] | None = None) -> None:
        result["facts"].append(_fact(name, arguments))
        key = (name, tuple(str(item) for item in arguments))
        active = set() if visited is None else set(visited)
        if key in active:
            return
        active.add(key)
        for declaration in model.get("predicates", {}).get(name, []):
            body = declaration.get("body")
            parameters = declaration.get("parameters", [])
            if not body or len(parameters) != len(arguments):
                continue
            bindings = {item["name"]: str(value) for item, value in zip(parameters, arguments)}
            for raw in body.split(";"):
                statement = raw.strip()
                if "{" in statement:
                    statement = statement.rsplit("{", 1)[-1].strip()
                match = re.fullmatch(r"([A-Za-z_][A-Za-z0-9_]*)\((.*)\)", statement)
                if match is None:
                    continue
                values: list[Any] = []
                for argument in _split_fact_arguments(match.group(2)):
                    substituted = argument
                    for parameter, value in bindings.items():
                        substituted = re.sub(rf"\b{re.escape(parameter)}\b", value, substituted)
                    values.append(substituted)
                add_predicate(match.group(1), values, active)

    for name, system in sorted(model["systems"].items()):
        parent = system.get("parent")
        if parent is not None and parent not in model["systems"]:
            continue
        result["states"][name] = system["initial_state"]
        for reference, target in sorted(system["references"].items()):
            result["references"][f"{name}.{reference}"] = target
        for expression in system["initial_facts"]:
            if expression["kind"] == "fact":
                arguments = [initial_value(item, name) for item in expression["arguments"]]
                add_predicate(expression["name"], arguments)
            elif expression["kind"] in {"assertion", "reference_condition"}:
                text = expression.get("expression", expression.get("text", ""))
                text = re.sub(r"\bself\b", name, text)
                result["facts"].append(_assertion(text))
    for name, declaration in sorted(model.get("types", {}).items()):
        for expression in declaration.get("invariant", []):
            if expression["kind"] == "fact":
                add_predicate(
                    expression["name"],
                    [initial_value(item, name) for item in expression["arguments"]],
                )
            elif expression["kind"] in {"assertion", "reference_condition"}:
                text = expression.get("expression", expression.get("text", ""))
                result["facts"].append(_assertion(re.sub(r"\bself\b", name, text)))
    return _snapshot(result)


def derive(
    model_document: dict[str, Any],
    *,
    signal: str | None = None,
    source: str = "Human",
    until: str | None = None,
    scenario: str | Path | None = None,
    max_depth: int | None = 3,
    max_breadth: int | None = 3,
) -> dict[str, Any]:
    try:
        canonical_signal = (
            None if signal is None else normalize_signal_request(signal, option="--signal")
        )
        canonical_until = (
            None if until is None else normalize_signal_request(until, option="--until")
        )
    except ValueError as exc:
        raise DerivationProblem(str(exc)) from exc
    model = model_document["model"]
    orchestration = None
    if canonical_signal is None:
        externals = list(model.get("externals", {}).values())
        if len(externals) != 1:
            raise DerivationProblem("default derivation requires exactly one external orchestration")
        orchestration = externals[0]
        first_calls = [*orchestration.get("drives", []), *orchestration.get("emits", [])]
        if not first_calls:
            raise DerivationProblem("external orchestration has no Signal calls")
        target, name = first_calls[0]["receiver"], first_calls[0]["name"]
        source = orchestration["name"]
    else:
        target, name = canonical_signal.rsplit(".", 1)
    until_target, until_name = (
        (None, None) if canonical_until is None else canonical_until.rsplit(".", 1)
    )
    base = initial_snapshot(model)
    start = load_scenario(
        scenario,
        model,
        base,
        model_fingerprint=model_document["model_fingerprint"],
    )
    return Engine(
        model_document,
        source=source,
        root_target=target,
        root_name=name,
        orchestration=orchestration,
        until_target=until_target,
        until_name=until_name,
        initial_snapshot=start,
        max_depth=max_depth,
        max_breadth=max_breadth,
    ).run()
