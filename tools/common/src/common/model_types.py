"""Static model data types shared by LKM verification tools."""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum

from common.spec_ast import (
    EnumDecl,
    EventDecl,
    FunctionDecl,
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
            location = f"line {self.span.start_line}: "
        return f"{self.severity.value}: {location}{self.message}"


@dataclass(frozen=True)
class EventDef:
    """Indexed event definition."""

    name: str
    object_name: str
    source_state: str
    target_state: str
    decl: EventDecl


@dataclass(frozen=True)
class StateDef:
    """Indexed state definition."""

    name: str
    object_name: str
    decl: StateDecl
    events: dict[str, EventDef] = field(default_factory=dict)


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


@dataclass(frozen=True)
class ObjectModel:
    """Static model built from the parsed spec."""

    enums: dict[str, EnumDecl]
    functions: dict[str, list[FunctionDecl]]
    predicates: dict[str, list[PredicateDecl]]
    types: dict[str, TypeDecl]
    objects: dict[str, ObjectDef]
    children: dict[str, list[str]]

    @property
    def state_count(self) -> int:
        return sum(len(obj.states) for obj in self.objects.values())

    @property
    def event_count(self) -> int:
        return sum(len(state.events) for obj in self.objects.values() for state in obj.states.values())


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
