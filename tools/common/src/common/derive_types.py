"""Derivation result data types shared by LKM verification tools."""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

from common.spec_ast import SourceSpan


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
class DerivationTraceNode:
    """One nested event node in the derivation trace."""

    object_name: str
    event_name: str
    source_state: str
    target_state: str
    status: DerivationStatus
    message: str | None = None
    span: SourceSpan | None = None
    children: tuple["DerivationTraceNode", ...] = ()

    @property
    def label(self) -> str:
        return f"{self.object_name}.Event::{self.event_name}"


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
    trace: tuple[DerivationTraceNode, ...] = ()

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
