"""Minimal static derivation engine for LKM object models."""

from __future__ import annotations

import re

from common.defaults import DEFAULT_TARGET
from common.derive_types import (
    DerivationRecord,
    DerivationResult,
    DerivationStatus,
    DerivationTraceNode,
    EventTransition,
)
from common.model_types import EventDef, ObjectDef, ObjectModel, StateDef
from common.spec_ast import Block, SourceSpan


_TARGET_RE = re.compile(
    r"\A([A-Z][A-Za-z0-9_]*)\.Event::([A-Za-z_][A-Za-z0-9_]*)\Z"
)
_STATE_EXPR_RE = re.compile(
    r"\A([A-Z][A-Za-z0-9_]*)\.state\s*==\s*State::([A-Za-z_][A-Za-z0-9_]*)\Z"
)
_EVENT_EXPR_RE = re.compile(
    r"\A([A-Z][A-Za-z0-9_]*)\.Event::([A-Za-z_][A-Za-z0-9_]*)\Z"
)
_PREDICATE_CALL_RE = re.compile(r"\A([A-Za-z_][A-Za-z0-9_]*)\s*\(")
_RELATION_RE = re.compile(r"(==|!=|>=|<=|>|<)")
_HAS_SLOT_RE = re.compile(
    r"\Ahas_slot\(\s*Config\.fixmap\s*,\s*FixMapSlot::([A-Za-z_][A-Za-z0-9_]*)\s*\)\Z"
)
_NO_SERVICE_RE = re.compile(r"\Ano_service\(\s*([A-Z][A-Za-z0-9_]*)\s*\)\Z")
_SLOT_ENTRY_RE = re.compile(r"\A([A-Za-z_][A-Za-z0-9_]*)\s*:\s*FixMapSlot<")

_AUTO_PREDICATES = {
    "aligned": "alignment",
    "has_slot": "config_structure",
    "inside": "range",
    "no_service": "state_alias",
    "page_aligned": "alignment",
    "readonly": "object_attribute",
    "valid_fixmap_config": "configuration",
    "valid_satp_mode": "configuration",
}
_ASSUMPTION_PREDICATES = {
    "addr_in_ram": "boot_input",
    "attrs_accessible": "environment",
    "context_is": "environment",
}
_EXTERNAL_PREDICATES = {
    "disjoint": "platform_memory_layout",
    "fits_in_fixmap_slot": "address_mapping",
    "gp_relative_access_ready": "architecture_state",
    "kernel_fpu_disabled": "riscv_status_register",
    "kernel_image_accessible": "address_mapping",
    "kernel_image_mapping_ready": "address_mapping",
    "kernel_vector_disabled": "riscv_status_register",
    "memory_zeroed": "memory_content",
    "phys_to_virt_transition_completed": "architecture_state",
    "range_in_ram": "physical_memory_membership",
    "raw_dtb_accessible": "address_mapping",
    "raw_dtb_mapping_ready": "address_mapping",
    "soc_early_platform_ready": "platform",
    "trampoline_mapping_ready": "address_mapping",
    "valid_dtb_header": "boot_input",
    "valid_dtb_magic": "boot_input",
    "valid_function_symbol": "linker_symbol",
    "valid_hart_id": "boot_hart_identity",
    "valid_object_storage": "object_storage",
    "valid_page_table_storage": "object_storage",
    "valid_phys_range_set": "platform",
    "valid_segment_set": "linker_layout",
    "valid_stack_pointer": "architecture_state",
    "valid_task_ref": "object_storage",
    "valid_task_storage": "object_storage",
    "valid_virt_addr": "address_mapping",
}
_DERIVED_PROVIDERS = {
    "disjoint": "fdt_candidate",
    "kernel_fpu_disabled": "isa_spec_and_boot_code",
    "kernel_vector_disabled": "isa_spec_and_boot_code",
    "range_in_ram": "boot_code_candidate",
    "valid_hart_id": "fdt_and_boot_protocol",
}


def derive(model: ObjectModel, target: str = DEFAULT_TARGET) -> DerivationResult:
    """Derive a target event from the model's declared initial states."""

    return _Deriver(model, target).run()


def summarize_derivation(result: DerivationResult) -> str:
    """Return a compact derivation summary."""

    counts = _record_counts(result.records)
    if result.ok:
        status = "ok"
    elif result.contradictions:
        status = "contradiction"
    elif result.blocked:
        status = "blocked"
    else:
        status = "incomplete"

    lines = [
        f"derive: {status}",
        f"target: {result.target}",
        f"target_reached: {'yes' if result.target_reached else 'no'}",
        f"transitions: {len(result.transitions)}",
    ]
    for status_name in (
        DerivationStatus.PROVED,
        DerivationStatus.ASSUMED,
        DerivationStatus.OBLIGATION,
        DerivationStatus.DEFERRED,
        DerivationStatus.BLOCKED,
        DerivationStatus.CONTRADICTION,
    ):
        lines.append(f"{status_name.value}: {counts.get(status_name, 0)}")
    return "\n".join(lines)


def render_derivation_text(result: DerivationResult) -> str:
    """Render a human-readable derivation report."""

    lines = [summarize_derivation(result)]

    if result.trace:
        lines.append("")
        lines.append("trace:")
        for node in result.trace:
            _append_trace_node(lines, node, depth=0)
    elif result.transitions:
        lines.append("")
        lines.append("transitions:")
        for transition in result.transitions:
            lines.append(f"- {transition.label}")

    for status in (
        DerivationStatus.BLOCKED,
        DerivationStatus.CONTRADICTION,
        DerivationStatus.DEFERRED,
        DerivationStatus.OBLIGATION,
    ):
        records = [record for record in result.records if record.status is status]
        if not records:
            continue
        lines.append("")
        lines.append(f"{status.value}:")
        if status is DerivationStatus.OBLIGATION:
            lines.extend(_format_obligation_category_summary(records))
        for record in records:
            lines.append(f"- {_format_record(record)}")

    return "\n".join(lines)


class _Deriver:
    def __init__(self, model: ObjectModel, target: str) -> None:
        self.model = model
        self.target = target
        self.states: dict[str, str] = {}
        self.records: list[DerivationRecord] = []
        self.transitions: list[EventTransition] = []
        self.trace: list[DerivationTraceNode] = []
        self.trace_stack: list[_TraceFrame] = []
        self.stack: list[tuple[str, str]] = []
        self.validated_states: set[tuple[str, str]] = set()

    def run(self) -> DerivationResult:
        target_object, target_event = self._parse_target()
        target_state = self._target_state(target_object, target_event)
        self._initialize_states()

        if target_object is not None and target_event is not None:
            self._derive_event(target_object, target_event)

        return DerivationResult(
            target=self.target,
            target_object=target_object,
            target_event=target_event,
            target_state=target_state,
            states=dict(self.states),
            records=tuple(self.records),
            transitions=tuple(self.transitions),
            trace=tuple(self.trace),
        )

    def _parse_target(self) -> tuple[str | None, str | None]:
        match = _TARGET_RE.match(self.target)
        if match is None:
            self._record(
                DerivationStatus.CONTRADICTION,
                f"invalid target event: {self.target}",
            )
            return None, None
        return match.group(1), match.group(2)

    def _initialize_states(self) -> None:
        for obj in self.model.objects.values():
            if obj.initial_state is None:
                continue
            self.states[obj.name] = obj.initial_state
            self._record(
                DerivationStatus.ASSUMED,
                f"initial state: {obj.name}.state == State::{obj.initial_state}",
                obj.decl.span,
                object_name=obj.name,
                state_name=obj.initial_state,
            )

        for obj in self.model.objects.values():
            if obj.initial_state is not None:
                self._validate_state(obj.name, obj.initial_state)

    def _target_state(
        self, object_name: str | None, event_name: str | None
    ) -> str | None:
        if object_name is None or event_name is None:
            return None
        obj = self.model.objects.get(object_name)
        if obj is None:
            return None
        event = _find_event(obj, event_name)
        if event is None:
            return None
        return event.target_state

    def _derive_event(self, object_name: str, event_name: str) -> bool:
        key = (object_name, event_name)
        if key in self.stack:
            self._record(
                DerivationStatus.BLOCKED,
                f"recursive event cycle: {_event_label(object_name, event_name)}",
                object_name=object_name,
                event_name=event_name,
            )
            return False

        obj = self.model.objects.get(object_name)
        if obj is None:
            self._record(
                DerivationStatus.CONTRADICTION,
                f"unknown object in target event: {object_name}",
                object_name=object_name,
                event_name=event_name,
            )
            return False

        current_state = self.states.get(object_name)
        event = self._event_from_current_state(obj, event_name, current_state)
        if event is None:
            return False

        trace_frame = _TraceFrame(
            object_name=object_name,
            event_name=event_name,
            source_state=event.source_state,
            target_state=event.target_state,
            span=event.decl.span,
        )
        self.trace_stack.append(trace_frame)
        self.stack.append(key)
        exit_status = DerivationStatus.BLOCKED
        exit_message: str | None = None
        try:
            self._collect_deferred(event.decl.deferred, event, "event")
            if not self._verify_blocks(event.decl.depends_on, "depends_on", event=event):
                exit_message = "depends_on blocked"
                return False

            for block in event.decl.drives:
                for entry, entry_span in block.entry_spans:
                    match = _EVENT_EXPR_RE.match(entry)
                    if match is None:
                        self._record(
                            DerivationStatus.BLOCKED,
                            f"cannot parse drives entry: {entry}",
                            entry_span,
                            object_name=object_name,
                            event_name=event_name,
                            expression=entry,
                        )
                        exit_message = f"cannot parse drives entry: {entry}"
                        return False

                    driven_object, driven_event = match.group(1), match.group(2)
                    if not self._derive_event(driven_object, driven_event):
                        self._record(
                            DerivationStatus.BLOCKED,
                            "driven event blocked: "
                            f"{_event_label(driven_object, driven_event)}",
                            entry_span,
                            object_name=object_name,
                            event_name=event_name,
                            expression=entry,
                        )
                        exit_message = (
                            "driven event blocked: "
                            f"{_event_label(driven_object, driven_event)}"
                        )
                        return False

            if self.states.get(object_name) != event.source_state:
                self._record(
                    DerivationStatus.CONTRADICTION,
                    "event source state changed during drives: "
                    f"{object_name}.state is State::{self.states.get(object_name)}, "
                    f"expected State::{event.source_state}",
                    event.decl.span,
                    object_name=object_name,
                    event_name=event_name,
                )
                exit_status = DerivationStatus.CONTRADICTION
                exit_message = (
                    f"source state changed to State::{self.states.get(object_name)}"
                )
                return False

            self.states[object_name] = event.target_state
            self.transitions.append(
                EventTransition(
                    object_name=object_name,
                    event_name=event_name,
                    source_state=event.source_state,
                    target_state=event.target_state,
                )
            )
            self._record(
                DerivationStatus.PROVED,
                f"transition: {_event_label(object_name, event_name)} "
                f"State::{event.source_state} -> State::{event.target_state}",
                event.decl.span,
                object_name=object_name,
                event_name=event_name,
                state_name=event.target_state,
            )
            if self._validate_state(object_name, event.target_state):
                exit_status = DerivationStatus.PROVED
                return True
            exit_message = "target state invariant blocked"
            return False
        finally:
            self.stack.pop()
            self.trace_stack.pop()
            self._finish_trace(trace_frame, exit_status, exit_message)

    def _event_from_current_state(
        self, obj: ObjectDef, event_name: str, current_state: str | None
    ) -> EventDef | None:
        if current_state is None:
            self._record(
                DerivationStatus.CONTRADICTION,
                f"object has no current state: {obj.name}",
                obj.decl.span,
                object_name=obj.name,
                event_name=event_name,
            )
            return None

        state = obj.states.get(current_state)
        if state is None:
            self._record(
                DerivationStatus.CONTRADICTION,
                f"unknown current state: {obj.name}.State::{current_state}",
                obj.decl.span,
                object_name=obj.name,
                event_name=event_name,
                state_name=current_state,
            )
            return None

        event = state.events.get(event_name)
        if event is not None:
            return event

        other = _find_event(obj, event_name)
        if other is None:
            self._record(
                DerivationStatus.CONTRADICTION,
                f"unknown event: {_event_label(obj.name, event_name)}",
                obj.decl.span,
                object_name=obj.name,
                event_name=event_name,
            )
        else:
            self._record(
                DerivationStatus.BLOCKED,
                f"event not enabled from State::{current_state}: "
                f"{_event_label(obj.name, event_name)} requires State::{other.source_state}",
                other.decl.span,
                object_name=obj.name,
                event_name=event_name,
                state_name=current_state,
            )
        return None

    def _validate_state(self, object_name: str, state_name: str) -> bool:
        key = (object_name, state_name)
        if key in self.validated_states:
            return True
        self.validated_states.add(key)

        obj = self.model.objects.get(object_name)
        if obj is None:
            self._record(
                DerivationStatus.CONTRADICTION,
                f"unknown object while validating state: {object_name}",
                object_name=object_name,
                state_name=state_name,
            )
            return False

        state = obj.states.get(state_name)
        if state is None:
            self._record(
                DerivationStatus.CONTRADICTION,
                f"unknown state while validating: {object_name}.State::{state_name}",
                obj.decl.span,
                object_name=object_name,
                state_name=state_name,
            )
            return False

        self._collect_deferred(state.decl.deferred, state, "state")
        return self._verify_blocks(state.decl.invariants, "invariant", state=state)

    def _verify_blocks(
        self,
        blocks: list[Block],
        kind: str,
        *,
        event: EventDef | None = None,
        state: StateDef | None = None,
    ) -> bool:
        ok = True
        for block in blocks:
            for entry, entry_span in block.entry_spans:
                if _STATE_EXPR_RE.match(entry):
                    ok = (
                        self._verify_state_expression(
                            entry, entry_span, kind, event, state
                        )
                        and ok
                    )
                elif self._try_prove_builtin_predicate(
                    entry, entry_span, kind, event, state
                ):
                    continue
                else:
                    classification = _classify_obligation(entry, kind)
                    self._record(
                        DerivationStatus.OBLIGATION,
                        f"unresolved {kind}: {entry}",
                        entry_span,
                        object_name=_context_object(event, state),
                        event_name=event.name if event is not None else None,
                        state_name=state.name if state is not None else None,
                        expression=entry,
                        source_kind=kind,
                        predicate=classification["predicate"],
                        obligation_category=classification["category"],
                        proof_class=classification["proof_class"],
                        proof_provider=classification["proof_provider"],
                    )
        return ok

    def _verify_state_expression(
        self,
        expression: str,
        span: SourceSpan,
        kind: str,
        event: EventDef | None,
        state: StateDef | None,
    ) -> bool:
        match = _STATE_EXPR_RE.match(expression)
        if match is None:
            return False

        object_name, expected_state = match.group(1), match.group(2)
        actual_state = self.states.get(object_name)
        if actual_state == expected_state:
            self._record(
                DerivationStatus.PROVED,
                f"{kind}: {expression}",
                span,
                object_name=_context_object(event, state),
                event_name=event.name if event is not None else None,
                state_name=state.name if state is not None else None,
                expression=expression,
            )
            return True

        self._record(
            DerivationStatus.BLOCKED,
            f"{kind} requires {expression}, got State::{actual_state}",
            span,
            object_name=_context_object(event, state),
            event_name=event.name if event is not None else None,
            state_name=state.name if state is not None else None,
            expression=expression,
        )
        return False

    def _try_prove_builtin_predicate(
        self,
        expression: str,
        span: SourceSpan,
        kind: str,
        event: EventDef | None,
        state: StateDef | None,
    ) -> bool:
        stripped = expression.strip()
        if stripped == "readonly(self)" and state is not None:
            obj = self.model.objects.get(state.object_name)
            if obj is not None and obj.decl.properties.get("access") == "Access::ReadOnly":
                self._record_builtin_proof(
                    expression,
                    span,
                    kind,
                    event,
                    state,
                    proof_class="object_attribute",
                    proof_provider="builtin",
                )
                return True

        has_slot = _HAS_SLOT_RE.match(stripped)
        if has_slot is not None:
            slot_name = has_slot.group(1)
            if self._fixmap_config_has_slot(slot_name):
                self._record_builtin_proof(
                    expression,
                    span,
                    kind,
                    event,
                    state,
                    proof_class="config_structure",
                    proof_provider="builtin",
                )
                return True

        no_service = _NO_SERVICE_RE.match(stripped)
        if no_service is not None:
            object_name = no_service.group(1)
            if self.states.get(object_name) == "Destroyed":
                self._record_builtin_proof(
                    expression,
                    span,
                    kind,
                    event,
                    state,
                    proof_class="state_alias",
                    proof_provider="builtin",
                )
                return True

        return False

    def _record_builtin_proof(
        self,
        expression: str,
        span: SourceSpan,
        kind: str,
        event: EventDef | None,
        state: StateDef | None,
        *,
        proof_class: str,
        proof_provider: str,
    ) -> None:
        self._record(
            DerivationStatus.PROVED,
            f"{kind}: {expression}",
            span,
            object_name=_context_object(event, state),
            event_name=event.name if event is not None else None,
            state_name=state.name if state is not None else None,
            expression=expression,
            source_kind=kind,
            predicate=_predicate_name(expression),
            proof_class=proof_class,
            proof_provider=proof_provider,
        )

    def _fixmap_config_has_slot(self, slot_name: str) -> bool:
        fixmap = self.model.types.get("FixMapConfig")
        if fixmap is None:
            return False
        expected = _slot_field_name(slot_name)
        for block in fixmap.blocks:
            if block.kind != "slots":
                continue
            for entry in block.entries:
                match = _SLOT_ENTRY_RE.match(entry)
                if match is not None and match.group(1) == expected:
                    return True
        return False

    def _collect_deferred(
        self,
        blocks: list[Block],
        owner: EventDef | StateDef,
        kind: str,
    ) -> None:
        for block in blocks:
            entries = block.entry_spans or [
                (block.body.strip(), SourceSpan(block.body_start_line or block.span.start_line, block.span.end_line))
            ]
            for entry, entry_span in entries:
                if not entry:
                    continue
                self._record(
                    DerivationStatus.DEFERRED,
                    f"{kind} deferred: {_strip_quotes(entry)}",
                    entry_span,
                    object_name=owner.object_name,
                    event_name=owner.name if isinstance(owner, EventDef) else None,
                    state_name=owner.name if isinstance(owner, StateDef) else None,
                    expression=entry,
                )

    def _record(
        self,
        status: DerivationStatus,
        message: str,
        span: SourceSpan | None = None,
        *,
        object_name: str | None = None,
        event_name: str | None = None,
        state_name: str | None = None,
        expression: str | None = None,
        source_kind: str | None = None,
        predicate: str | None = None,
        obligation_category: str | None = None,
        proof_class: str | None = None,
        proof_provider: str | None = None,
    ) -> None:
        self.records.append(
            DerivationRecord(
                status=status,
                message=message,
                span=span,
                object_name=object_name,
                event_name=event_name,
                state_name=state_name,
                expression=expression,
                source_kind=source_kind,
                predicate=predicate,
                obligation_category=obligation_category,
                proof_class=proof_class,
                proof_provider=proof_provider,
            )
        )

    def _finish_trace(
        self,
        frame: "_TraceFrame",
        status: DerivationStatus,
        message: str | None,
    ) -> None:
        node = DerivationTraceNode(
            object_name=frame.object_name,
            event_name=frame.event_name,
            source_state=frame.source_state,
            target_state=frame.target_state,
            status=status,
            message=message,
            span=frame.span,
            children=tuple(frame.children),
        )
        if self.trace_stack:
            self.trace_stack[-1].children.append(node)
        else:
            self.trace.append(node)


class _TraceFrame:
    def __init__(
        self,
        *,
        object_name: str,
        event_name: str,
        source_state: str,
        target_state: str,
        span: SourceSpan,
    ) -> None:
        self.object_name = object_name
        self.event_name = event_name
        self.source_state = source_state
        self.target_state = target_state
        self.span = span
        self.children: list[DerivationTraceNode] = []


def _find_event(obj: ObjectDef, event_name: str) -> EventDef | None:
    for state in obj.states.values():
        event = state.events.get(event_name)
        if event is not None:
            return event
    return None


def _context_object(event: EventDef | None, state: StateDef | None) -> str | None:
    if event is not None:
        return event.object_name
    if state is not None:
        return state.object_name
    return None


def _event_label(object_name: str, event_name: str) -> str:
    return f"{object_name}.Event::{event_name}"


def _strip_quotes(value: str) -> str:
    stripped = value.strip()
    if len(stripped) >= 2 and stripped[0] == '"' and stripped[-1] == '"':
        return stripped[1:-1]
    return stripped


def _record_counts(records: tuple[DerivationRecord, ...]) -> dict[DerivationStatus, int]:
    counts: dict[DerivationStatus, int] = {}
    for record in records:
        counts[record.status] = counts.get(record.status, 0) + 1
    return counts


def _format_obligation_category_summary(records: list[DerivationRecord]) -> list[str]:
    counts: dict[str, int] = {}
    for record in records:
        category = record.obligation_category or "unknown"
        counts[category] = counts.get(category, 0) + 1
    if not counts:
        return []
    summary = ", ".join(f"{name}={count}" for name, count in sorted(counts.items()))
    return [f"  categories: {summary}"]


def _classify_obligation(expression: str, source_kind: str) -> dict[str, str | None]:
    predicate = _predicate_name(expression)
    if predicate in _AUTO_PREDICATES:
        return {
            "predicate": predicate,
            "category": "auto_candidate",
            "proof_class": _AUTO_PREDICATES[predicate],
            "proof_provider": "builtin_candidate",
        }
    if predicate in _ASSUMPTION_PREDICATES:
        return {
            "predicate": predicate,
            "category": "assumption_candidate",
            "proof_class": _ASSUMPTION_PREDICATES[predicate],
            "proof_provider": "assumption_candidate",
        }
    if predicate in _EXTERNAL_PREDICATES:
        return {
            "predicate": predicate,
            "category": "derived_candidate",
            "proof_class": _EXTERNAL_PREDICATES[predicate],
            "proof_provider": _DERIVED_PROVIDERS.get(predicate, "derived_candidate"),
        }

    if _is_relation_expression(expression):
        return {
            "predicate": predicate,
            "category": "auto_candidate",
            "proof_class": "relation",
            "proof_provider": "builtin_candidate",
        }

    if source_kind == "depends_on":
        category = "spec_gap"
    else:
        category = "unknown"
    return {
        "predicate": predicate,
        "category": category,
        "proof_class": "unknown",
        "proof_provider": "unknown",
    }


def _predicate_name(expression: str) -> str | None:
    match = _PREDICATE_CALL_RE.match(expression.strip())
    if match is None:
        return None
    return match.group(1)


def _is_relation_expression(expression: str) -> bool:
    return bool(_RELATION_RE.search(expression))


def _slot_field_name(slot_name: str) -> str:
    return slot_name[:1].lower() + slot_name[1:]


def _append_trace_node(
    lines: list[str], node: DerivationTraceNode, depth: int
) -> None:
    indent = "  " * depth
    lines.append(f"{indent}> {node.label} State::{node.source_state}")
    for child in node.children:
        _append_trace_node(lines, child, depth + 1)

    suffix = ""
    if node.status in (DerivationStatus.BLOCKED, DerivationStatus.CONTRADICTION):
        suffix = f" {node.status.value}"
        if node.message:
            suffix += f": {node.message}"
    lines.append(f"{indent}< {node.label} State::{node.target_state}{suffix}")


def _format_record(record: DerivationRecord) -> str:
    location = ""
    if record.span is not None:
        location = f"line {record.span.start_line}: "
    return f"{location}{record.message}"


__all__ = [
    "DEFAULT_TARGET",
    "DerivationRecord",
    "DerivationResult",
    "DerivationStatus",
    "DerivationTraceNode",
    "EventTransition",
    "derive",
    "render_derivation_text",
    "summarize_derivation",
]
