"""Static model data types shared by LKM verification tools."""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum

from common.spec_ast import (
    BoundaryDecl,
    ContextGuardDecl,
    EnumDecl,
    TransitionDecl,
    ExclusiveContextDecl,
    FunctionDecl,
    LockDecl,
    ObjectDecl,
    PredicateDecl,
    SourceSpan,
    StateDecl,
    TypeDecl,
)


class Severity(str, Enum):
    """Diagnostic severity."""

    ERROR = "error"
    WARNING = "warning"


@dataclass(frozen=True)
class Diagnostic:
    """A model building diagnostic."""

    severity: Severity
    message: str
    span: SourceSpan | None = None

    def format(self) -> str:
        location = ""
        if self.span is not None:
            if self.span.source_file is not None:
                loc = self.span.source_line or self.span.start_line
                location = f"{self.span.source_file}:{loc}: "
            else:
                location = f"line {self.span.start_line}: "
        return f"{self.severity.value}: {location}{self.message}"


@dataclass(frozen=True)
class TransitionDef:
    """Indexed transition definition."""

    name: str
    object_name: str
    source_state: str
    target_state: str
    decl: TransitionDecl
    lifecycle_owner: str | None = None
    handler_contributions: tuple[tuple[str, SourceSpan], ...] = ()


@dataclass(frozen=True)
class StateDef:
    """Indexed state definition."""

    name: str
    object_name: str
    decl: StateDecl
    transitions: dict[str, TransitionDef] = field(default_factory=dict)
    lifecycle_owner: str | None = None


@dataclass(frozen=True)
class ObjectDef:
    """Indexed object definition."""

    name: str
    kind: str
    decl: ObjectDecl
    initial_state: str | None
    parent: str | None
    states: dict[str, StateDef] = field(default_factory=dict)
    children: list[str] = field(default_factory=list)
    attrs: dict[str, str] = field(default_factory=dict)
    associations: dict[str, str] = field(default_factory=dict)


@dataclass(frozen=True)
class ExclusiveContextDef:
    """Indexed exclusive context definition."""

    name: str
    decl: ExclusiveContextDecl
    kind: str | None
    guard: ContextGuardDecl | None
    lock_ref: str | None
    obj_refs: tuple[str, ...] = ()


@dataclass(frozen=True)
class BoundaryDef:
    """A validated structured deferred or trimmed boundary with inferred owner."""

    id: str
    status: str
    category: str
    summary: str
    resolution: str
    decl: BoundaryDecl
    object_name: str
    state_name: str
    transition_name: str | None = None
    context_path: tuple[str, ...] = ()

    @property
    def owner(self) -> str:
        if self.transition_name is None:
            owner = f"{self.object_name}.State::{self.state_name}"
        else:
            owner = f"{self.object_name}.Transition::{self.transition_name}"
        if self.context_path:
            owner += " within " + " / ".join(self.context_path)
        return owner


@dataclass(frozen=True)
class DeclarationSiteDef:
    """A static runtime-instance declaration template owned by one process."""

    owner_process: str
    ordinal: int
    alias: str
    declared_type: str
    span: SourceSpan


@dataclass(frozen=True)
class ObjectModel:
    """Static model built from the parsed spec."""

    enums: dict[str, EnumDecl]
    functions: dict[str, list[FunctionDecl]]
    predicates: dict[str, list[PredicateDecl]]
    types: dict[str, TypeDecl]
    locks: dict[str, LockDecl]
    exclusive_contexts: dict[str, ExclusiveContextDef]
    objects: dict[str, ObjectDef]
    children: dict[str, list[str]]
    boundaries: dict[str, BoundaryDef] = field(default_factory=dict)
    declaration_sites: tuple[DeclarationSiteDef, ...] = ()
    legacy_boundary_count: int = 0

    @property
    def state_count(self) -> int:
        return sum(len(obj.states) for obj in self.objects.values())

    @property
    def transition_count(self) -> int:
        return sum(len(state.transitions) for obj in self.objects.values() for state in obj.states.values())

    @property
    def deferred_count(self) -> int:
        return sum(boundary.status == "deferred" for boundary in self.boundaries.values())

    @property
    def trimmed_count(self) -> int:
        return sum(boundary.status == "trimmed" for boundary in self.boundaries.values())


@dataclass(frozen=True)
class BuildResult:
    """Model builder result."""

    model: ObjectModel
    diagnostics: list[Diagnostic]

    @property
    def errors(self) -> list[Diagnostic]:
        return [diag for diag in self.diagnostics if diag.severity is Severity.ERROR]

    @property
    def warnings(self) -> list[Diagnostic]:
        return [diag for diag in self.diagnostics if diag.severity is Severity.WARNING]

    @property
    def ok(self) -> bool:
        return not self.errors
