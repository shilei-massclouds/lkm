"""Deterministic bounded Signal derivation for the tools2 compatibility model."""

from __future__ import annotations

from copy import deepcopy
import json
from pathlib import Path
from typing import Any

from tools2_common import (
    PRODUCER,
    SNAPSHOT_SCHEMA,
    SNAPSHOT_VERSION,
    ProtocolError,
    read_json,
    require_protocol,
)


class DerivationProblem(ValueError):
    pass


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
    }


def _fact_argument(value: Any) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, str) and not value.replace("_", "").replace("-", "").replace("::", "").isalnum():
        return json.dumps(value, ensure_ascii=False, separators=(",", ":"))
    return str(value)


def _fact(name: str, arguments: list[Any]) -> str:
    return name if not arguments else f"{name}({','.join(_fact_argument(item) for item in arguments)})"


def _matches_system_type(model: dict[str, Any], target: str, expected: str) -> bool:
    if target not in model["systems"]:
        return False
    return (
        expected == "System"
        or target == expected
        or model["systems"][target].get("declared_type") == expected
    )


def load_scenario(
    path: str | Path | None, model: dict[str, Any], initial: dict[str, Any]
) -> dict[str, Any]:
    if path is None:
        return _snapshot(initial)
    value = read_json(path)
    if "schema" in value:
        require_protocol(
            value, schema=SNAPSHOT_SCHEMA, version=SNAPSHOT_VERSION, label="snapshot"
        )
        value = value.get("snapshot", {})
    elif "snapshot" in value:
        value = value["snapshot"]
    result = deepcopy(initial)
    states = value.get("states", value.get("state", {}))
    if not isinstance(states, dict):
        raise ProtocolError("scenario states must be an object")
    for target, state in states.items():
        if target not in model["systems"]:
            raise ProtocolError(f"scenario has unknown system state target {target}")
        if state not in model["systems"][target]["states"]:
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
        owner, field = reference.split(".", 1)
        if owner not in model["systems"]:
            raise ProtocolError(f"scenario has unknown reference owner {owner}")
        if field not in model["systems"][owner]["reference_types"]:
            raise ProtocolError(f"scenario has unknown reference {reference}")
        if target not in model["systems"]:
            raise ProtocolError(f"scenario has unknown reference target {target}")
        expected = model["systems"][owner]["reference_types"][field]
        if not _matches_system_type(model, target, expected):
            raise ProtocolError(
                f"scenario reference {reference} expects {expected}, got {target}"
            )
        result["references"][reference] = target
    return _snapshot(result)


class Engine:
    def __init__(
        self,
        model_document: dict[str, Any],
        *,
        source: str,
        root_target: str,
        root_name: str,
        initial_snapshot: dict[str, Any],
        max_depth: int | None,
        max_breadth: int | None,
    ) -> None:
        self.document = model_document
        self.model = model_document["model"]
        self.systems = self.model["systems"]
        self.source = source
        self.root_target = root_target
        self.root_name = root_name
        self.initial_snapshot = _snapshot(initial_snapshot)
        self.current = _snapshot(initial_snapshot)
        self.max_depth = max_depth
        self.max_breadth = max_breadth
        self.signals: list[dict[str, Any]] = []
        self.signal_index: dict[str, dict[str, Any]] = {}
        self.events: list[dict[str, Any]] = []
        self.queue: list[dict[str, Any]] = []
        self.frontier: list[dict[str, Any]] = []
        self.failed = False
        self.was_bounded = False
        self.failure_signal_id: str | None = None
        self.failure_reason: str | None = None
        self.next_signal = 1
        self.next_event = 1

    def event(self, kind: str, **fields: Any) -> None:
        self.events.append({"sequence": self.next_event, "kind": kind, **fields})
        self.next_event += 1

    def new_signal(
        self,
        *,
        source: str,
        target: str,
        name: str,
        raw_arguments: list[dict[str, Any]],
        delivery: str,
        lossy: bool,
        cause_id: str | None,
        coordinate: dict[str, Any],
        compat_process_kind: str | None,
    ) -> dict[str, Any]:
        signal_id = f"sig-{self.next_signal:04d}"
        self.next_signal += 1
        signal = {
            "id": signal_id,
            "source": source,
            "target": target,
            "name": name,
            "delivery": delivery,
            "strict": not lossy,
            "lossy": lossy,
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
            lossy=lossy,
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
        if signal["lossy"]:
            signal["outcome"] = "discarded"
            self.event("signal_discarded", signal_id=signal["id"], reason=reason)
        else:
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
        if first == "self":
            value: Any = signal["target"]
        elif first in bindings:
            value = bindings[first]
        elif first in self.systems:
            value = first
        else:
            raise DerivationProblem(f"unresolved system/reference path {path}")
        for field in parts[1:]:
            key = f"{value}.{field}"
            if key not in self.current["references"]:
                raise DerivationProblem(f"unbound system reference {key}")
            value = self.current["references"][key]
        return value

    def value(
        self, expression: dict[str, Any], *, signal: dict[str, Any], bindings: dict[str, Any]
    ) -> Any:
        kind = expression["kind"]
        if kind in {"string", "integer", "boolean"}:
            return expression["value"]
        if kind == "enum":
            return f"{expression['type']}::{expression['value']}"
        if kind == "path":
            return self.resolve_path(expression["value"], signal=signal, bindings=bindings)
        raise DerivationProblem(f"unsupported runtime value kind {kind}")

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
            if len(parameters) != 1 or len(positional) != 1:
                raise DerivationProblem("positional payload is allowed only for a one-parameter handler")
            assignments = {parameters[0]["name"]: positional[0]["value"]}
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
        for parameter in parameters:
            name = parameter["name"]
            value = self.value(assignments[name], signal=signal, bindings=bindings)
            self.validate_type(parameter["type"], value)
            bindings[name] = value
            payload.append({"name": name, "type": parameter["type"], "value": value})
        signal["payload"] = payload
        return bindings

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
        elif expected not in {"String", "Int", "Bool"} and expected not in self.model["enums"]:
            if value not in self.systems:
                raise DerivationProblem(f"expected {expected} system reference, got {value!r}")
            target = self.systems[value]
            if value != expected and target.get("declared_type") != expected:
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
            if kind == "state_condition":
                target = self.resolve_path(expression["target"], signal=signal, bindings=bindings)
                return self.current["states"].get(target) == expression["state"]
            if kind == "reference_condition":
                reference = expression["reference"]
                parts = reference.split(".")
                if parts[0] == "self":
                    key = ".".join([signal["target"], *parts[1:]])
                elif parts[0] in bindings:
                    key = ".".join([str(bindings[parts[0]]), *parts[1:]])
                else:
                    key = reference
                expected = self.value(expression["value"], signal=signal, bindings=bindings)
                return self.current["references"].get(key) == expected
            if kind == "fact":
                values = [self.value(item, signal=signal, bindings=bindings) for item in expression["arguments"]]
                return _fact(expression["name"], values) in self.current["facts"]
            raise DerivationProblem(f"unsupported condition kind {kind}")
        finally:
            self.current = saved

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
        if kind == "fact":
            values = [self.value(item, signal=signal, bindings=bindings) for item in expression["arguments"]]
            candidate["facts"].append(_fact(expression["name"], values))
            candidate["facts"] = sorted(set(candidate["facts"]))
            return
        if kind == "reference_assignment":
            parts = expression["reference"].split(".")
            if parts[0] == "self":
                key = ".".join([signal["target"], *parts[1:]])
            elif parts[0] in bindings:
                key = ".".join([str(bindings[parts[0]]), *parts[1:]])
            else:
                key = expression["reference"]
            owner, field = key.split(".", 1)
            if owner not in self.systems or field not in self.systems[owner]["reference_types"]:
                raise DerivationProblem(f"unknown reference update {key}")
            value = self.value(expression["value"], signal=signal, bindings=bindings)
            if value not in self.systems:
                raise DerivationProblem(f"reference update target is not a system: {value!r}")
            expected = self.systems[owner]["reference_types"][field]
            if not _matches_system_type(self.model, value, expected):
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
        raise DerivationProblem(f"unsupported effect kind {kind}")

    def _handler_conditions(self, handler: dict[str, Any]) -> list[dict[str, Any]]:
        return [
            entry
            for block in handler["body"]
            if block["kind"] == "depends_on"
            for entry in block["entries"]
        ]

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
        system = self.systems[signal["target"]]
        candidates = system["handlers_by_name"].get(signal["name"], [])
        if not candidates:
            self.reject(signal, "no_handler")
            return
        if len(candidates) != 1:
            self.reject(signal, "ambiguous_handler")
            return
        handler = candidates[0]
        signal["handler"] = {
            "id": handler["id"],
            "kind": handler["kind"],
            "source_state": handler["source_state"],
            "target_state": handler["target_state"],
            "span": handler["span"],
        }
        try:
            bindings = self.bind_payload(signal, handler)
        except DerivationProblem as exc:
            self.fail(signal, f"payload_error: {exc}")
            return
        actual_state = self.current["states"].get(signal["target"])
        if actual_state != handler["source_state"]:
            self.reject(
                signal,
                f"state_not_accepted: expected State::{handler['source_state']}, got State::{actual_state}",
            )
            return
        for condition in self._handler_conditions(handler):
            try:
                result = self.expression_value(condition, signal=signal, bindings=bindings)
            except DerivationProblem as exc:
                self.fail(signal, f"condition_error: {exc}")
                return
            self.event(
                "condition_checked",
                signal_id=signal["id"],
                expression=condition["text"],
                result=result,
            )
            if not result:
                self.reject(signal, f"condition_not_satisfied: {condition['text']}")
                return
        signal["outcome"] = "accepted"
        self.event("response_started", signal_id=signal["id"], handler=handler["id"])
        pending_emits: list[dict[str, Any]] = []
        try:
            for block in handler["body"]:
                if block["kind"] == "drives":
                    for call in block["entries"]:
                        target = self.resolve_path(call["receiver"], signal=signal, bindings=bindings)
                        coordinate = self.coordinate(signal["target"], target, signal["coordinate"])
                        child = self.new_signal(
                            source=signal["target"],
                            target=target,
                            name=call["name"],
                            raw_arguments=self.materialize_arguments(
                                call["arguments"], signal=signal, bindings=bindings
                            ),
                            delivery="drives",
                            lossy=call["lossy"],
                            cause_id=signal["id"],
                            coordinate=coordinate,
                            compat_process_kind=call["process_kind"],
                        )
                        self.event("drives_wait_started", signal_id=signal["id"], child_id=child["id"])
                        self.deliver(child)
                        self.event(
                            "drives_wait_finished",
                            signal_id=signal["id"],
                            child_id=child["id"],
                            child_outcome=child["outcome"],
                        )
                        if self.failed:
                            signal["outcome"] = "failed"
                            signal["reason"] = f"strict child {child['id']} did not complete"
                            signal["after_snapshot"] = _snapshot(self.current)
                            return
                elif block["kind"] == "emits":
                    pending_emits.extend(block["entries"])
            candidate = _snapshot(self.current)
            if handler["kind"] == "Transition":
                candidate["states"][signal["target"]] = handler["target_state"]
            for block in handler["body"]:
                if block["kind"] in {"ensures", "updates"}:
                    for expression in block["entries"]:
                        self.apply_effect(
                            expression,
                            signal=signal,
                            bindings=bindings,
                            candidate=candidate,
                            handler=handler,
                        )
            if handler["kind"] == "Transition":
                target_state = system["states"][handler["target_state"]]
                for invariant in target_state["invariant"]:
                    result = self.expression_value(
                        invariant, signal=signal, bindings=bindings, snapshot=candidate
                    )
                    self.event(
                        "invariant_checked",
                        signal_id=signal["id"],
                        expression=invariant["text"],
                        result=result,
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
                target = self.resolve_path(call["receiver"], signal=signal, bindings=bindings)
                coordinate = self.coordinate(signal["target"], target, signal["coordinate"])
                child = self.new_signal(
                    source=signal["target"],
                    target=target,
                    name=call["name"],
                    raw_arguments=self.materialize_arguments(
                        call["arguments"], signal=signal, bindings=bindings
                    ),
                    delivery="emits",
                    lossy=call["lossy"],
                    cause_id=signal["id"],
                    coordinate=coordinate,
                    compat_process_kind=call["process_kind"],
                )
                self.queue.append(child)
                self.event(
                    "emits_enqueued",
                    signal_id=signal["id"],
                    child_id=child["id"],
                    fifo_position=len(self.queue),
                )
        except DerivationProblem as exc:
            self.fail(signal, str(exc))

    def run(self) -> dict[str, Any]:
        root = self.new_signal(
            source=self.source,
            target=self.root_target,
            name=self.root_name,
            raw_arguments=[],
            delivery="root",
            lossy=False,
            cause_id=None,
            coordinate={
                "depth": 0,
                "breadth": 0,
                "movement": {"up": 0, "across": 0, "down": 0},
                "_budget_depth": 0,
                "_budget_breadth": 0,
            },
            compat_process_kind=None,
        )
        diagnostics = self.document.get("diagnostics", [])
        if diagnostics:
            root["before_snapshot"] = _snapshot(self.current)
            self.fail(root, "model_has_errors_or_unsupported_syntax")
        else:
            self.deliver(root)
        while self.queue and not self.failed:
            signal = self.queue.pop(0)
            self.event("emits_dequeued", signal_id=signal["id"], remaining=len(self.queue))
            self.deliver(signal)
        verdict = "failed" if self.failed else "bounded" if self.was_bounded else "complete"
        for signal in self.signals:
            signal.pop("_raw_arguments", None)
            signal.pop("_budget_depth", None)
            signal.pop("_budget_breadth", None)
        return {
            "root_request": {
                "source": self.source,
                "target": self.root_target,
                "signal": self.root_name,
                "signal_id": root["id"],
            },
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
                "discarded": sum(item["outcome"] == "discarded" for item in self.signals),
                "rejected": sum(item["outcome"] == "rejected" for item in self.signals),
                "failed": sum(item["outcome"] == "failed" for item in self.signals),
                "truncated": sum(item["outcome"] == "truncated" for item in self.signals),
                "pending": 0,
            },
        }


def initial_snapshot(model: dict[str, Any]) -> dict[str, Any]:
    result: dict[str, Any] = {"states": {}, "facts": [], "references": {}}
    for name, system in sorted(model["systems"].items()):
        result["states"][name] = system["initial_state"]
        for reference, target in sorted(system["references"].items()):
            result["references"][f"{name}.{reference}"] = target
        for expression in system["initial_facts"]:
            if expression["kind"] != "fact":
                continue
            arguments: list[Any] = []
            for item in expression["arguments"]:
                if item["kind"] == "path" and item["value"] == "self":
                    arguments.append(name)
                elif item["kind"] == "enum":
                    arguments.append(f"{item['type']}::{item['value']}")
                else:
                    arguments.append(item["value"])
            result["facts"].append(_fact(expression["name"], arguments))
    return _snapshot(result)


def derive(
    model_document: dict[str, Any],
    *,
    signal: str,
    source: str = "Environment",
    scenario: str | Path | None = None,
    max_depth: int | None = 3,
    max_breadth: int | None = 3,
) -> dict[str, Any]:
    if "." not in signal or signal.startswith(".") or signal.endswith("."):
        raise DerivationProblem("--signal must be Target.SignalName")
    target, name = signal.rsplit(".", 1)
    model = model_document["model"]
    base = initial_snapshot(model)
    start = load_scenario(scenario, model, base)
    return Engine(
        model_document,
        source=source,
        root_target=target,
        root_name=name,
        initial_snapshot=start,
        max_depth=max_depth,
        max_breadth=max_breadth,
    ).run()
