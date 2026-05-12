"""Minimal static derivation engine for pyveri object models."""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from enum import Enum

from .ast import Block, SourceSpan
from .model import EventDef, ObjectDef, ObjectModel, StateDef


DEFAULT_TARGET = "StartupTimeline.Event::Setup"


class DerivationStatus(str, Enum):
    """Status categories produced by the derivation engine."""

    PROVED = "proved"
    ASSUMED = "assumed"
    OBLIGATION = "obligation"
    DEFERRED = "deferred"
    BLOCKED = "blocked"
    CONTRADICTION = "contradiction"


@dataclass(frozen=True)
class DerivationRecord:
    """One derivation fact, proof obligation, or failure."""

    status: DerivationStatus
    message: str
    span: SourceSpan | None = None
    object_name: str | None = None
    event_name: str | None = None
    state_name: str | None = None
    expression: str | None = None


@dataclass(frozen=True)
class EventTransition:
    """An event transition completed during derivation."""

    object_name: str
    event_name: str
    source_state: str
    target_state: str

    @property
    def label(self) -> str:
        return (
            f"{self.object_name}.Event::{self.event_name}: "
            f"State::{self.source_state} -> State::{self.target_state}"
        )


@dataclass(frozen=True)
class DerivationResult:
    """Result of deriving a target event from an object model."""

    target: str
    target_object: str | None
    target_event: str | None
    target_state: str | None
    states: dict[str, str]
    records: tuple[DerivationRecord, ...] = ()
    transitions: tuple[EventTransition, ...] = ()

    @property
    def blocked(self) -> list[DerivationRecord]:
        return [record for record in self.records if record.status is DerivationStatus.BLOCKED]

    @property
    def contradictions(self) -> list[DerivationRecord]:
        return [
            record
            for record in self.records
            if record.status is DerivationStatus.CONTRADICTION
        ]

    @property
    def obligations(self) -> list[DerivationRecord]:
        return [
            record
            for record in self.records
            if record.status is DerivationStatus.OBLIGATION
        ]

    @property
    def deferred(self) -> list[DerivationRecord]:
        return [record for record in self.records if record.status is DerivationStatus.DEFERRED]

    @property
    def target_reached(self) -> bool:
        if self.target_object is None or self.target_state is None:
            return False
        return self.states.get(self.target_object) == self.target_state

    @property
    def ok(self) -> bool:
        return self.target_reached and not self.blocked and not self.contradictions


_TARGET_RE = re.compile(
    r"\A([A-Z][A-Za-z0-9_]*)\.Event::([A-Za-z_][A-Za-z0-9_]*)\Z"
)
_STATE_EXPR_RE = re.compile(
    r"\A([A-Z][A-Za-z0-9_]*)\.state\s*==\s*State::([A-Za-z_][A-Za-z0-9_]*)\Z"
)
_EVENT_EXPR_RE = re.compile(
    r"\A([A-Z][A-Za-z0-9_]*)\.Event::([A-Za-z_][A-Za-z0-9_]*)\Z"
)


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

    if result.transitions:
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

        self.stack.append(key)
        try:
            self._collect_deferred(event.decl.deferred, event, "event")
            if not self._verify_blocks(event.decl.depends_on, "depends_on", event=event):
                return False

            for block in event.decl.drives:
                for entry in block.entries:
                    match = _EVENT_EXPR_RE.match(entry)
                    if match is None:
                        self._record(
                            DerivationStatus.BLOCKED,
                            f"cannot parse drives entry: {entry}",
                            block.span,
                            object_name=object_name,
                            event_name=event_name,
                            expression=entry,
                        )
                        return False

                    driven_object, driven_event = match.group(1), match.group(2)
                    if not self._derive_event(driven_object, driven_event):
                        self._record(
                            DerivationStatus.BLOCKED,
                            "driven event blocked: "
                            f"{_event_label(driven_object, driven_event)}",
                            block.span,
                            object_name=object_name,
                            event_name=event_name,
                            expression=entry,
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
            return self._validate_state(object_name, event.target_state)
        finally:
            self.stack.pop()

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
            for entry in block.entries:
                if _STATE_EXPR_RE.match(entry):
                    ok = self._verify_state_expression(entry, block, kind, event, state) and ok
                else:
                    self._record(
                        DerivationStatus.OBLIGATION,
                        f"unresolved {kind}: {entry}",
                        block.span,
                        object_name=_context_object(event, state),
                        event_name=event.name if event is not None else None,
                        state_name=state.name if state is not None else None,
                        expression=entry,
                    )
        return ok

    def _verify_state_expression(
        self,
        expression: str,
        block: Block,
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
                block.span,
                object_name=_context_object(event, state),
                event_name=event.name if event is not None else None,
                state_name=state.name if state is not None else None,
                expression=expression,
            )
            return True

        self._record(
            DerivationStatus.BLOCKED,
            f"{kind} requires {expression}, got State::{actual_state}",
            block.span,
            object_name=_context_object(event, state),
            event_name=event.name if event is not None else None,
            state_name=state.name if state is not None else None,
            expression=expression,
        )
        return False

    def _collect_deferred(
        self,
        blocks: list[Block],
        owner: EventDef | StateDef,
        kind: str,
    ) -> None:
        for block in blocks:
            entries = block.entries or [block.body.strip()]
            for entry in entries:
                if not entry:
                    continue
                self._record(
                    DerivationStatus.DEFERRED,
                    f"{kind} deferred: {_strip_quotes(entry)}",
                    block.span,
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
            )
        )


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
    "EventTransition",
    "derive",
    "render_derivation_text",
    "summarize_derivation",
]
