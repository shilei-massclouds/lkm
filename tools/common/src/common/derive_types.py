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
    TRIMMED = "trimmed"
    BLOCKED = "blocked"
    CONTRADICTION = "contradiction"


@dataclass(frozen=True)
class DerivationRecord:
    """One derivation fact, proof obligation, or failure."""

    status: DerivationStatus
    message: str
    span: SourceSpan | None = None
    object_name: str | None = None
    transition_name: str | None = None
    state_name: str | None = None
    expression: str | None = None
    display_expression: str | None = None
    source_kind: str | None = None
    predicate: str | None = None
    obligation_category: str | None = None
    proof_class: str | None = None
    proof_provider: str | None = None
    process_parent: str | None = None
    boundary_id: str | None = None
    boundary_category: str | None = None
    boundary_summary: str | None = None
    boundary_resolution: str | None = None
    boundary_owner: str | None = None


@dataclass(frozen=True)
class TransitionCommit:
    """A transition completed during derivation."""

    object_name: str
    transition_name: str
    source_state: str
    target_state: str

    @property
    def label(self) -> str:
        return (
            f"{self.object_name}.Transition::{self.transition_name}: "
            f"State::{self.source_state} -> State::{self.target_state}"
        )


@dataclass(frozen=True)
class DerivationTraceNode:
    """One nested transition node in the derivation trace."""

    object_name: str
    transition_name: str
    source_state: str
    target_state: str
    status: DerivationStatus
    message: str | None = None
    span: SourceSpan | None = None
    edge_kind: str | None = None
    children: tuple["DerivationTraceNode", ...] = ()

    @property
    def label(self) -> str:
        return f"{self.object_name}.Transition::{self.transition_name}"


@dataclass(frozen=True)
class DerivationResult:
    """Result of deriving a target transition from an object model."""

    target: str
    target_object: str | None
    target_transition: str | None
    target_state: str | None
    states: dict[str, str]
    records: tuple[DerivationRecord, ...] = ()
    transitions: tuple[TransitionCommit, ...] = ()
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
    def trimmed(self) -> list[DerivationRecord]:
        return [record for record in self.records if record.status is DerivationStatus.TRIMMED]

    @property
    def target_reached(self) -> bool:
        if self.target_object is None or self.target_transition is None:
            return False
        return any(
            transition.object_name == self.target_object
            and transition.transition_name == self.target_transition
            for transition in self.transitions
        )

    @property
    def ok(self) -> bool:
        return self.target_reached and not self.blocked and not self.contradictions
