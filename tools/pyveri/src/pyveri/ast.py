"""Syntax-level AST nodes for pyveri specs."""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass(frozen=True)
class SourceSpan:
    """Line span in the source file."""

    start_line: int
    end_line: int


@dataclass(frozen=True)
class Block:
    """Raw block content preserved for later semantic analysis."""

    kind: str
    body: str
    span: SourceSpan
    header: str = ""
    body_start_line: int | None = None

    @property
    def entries(self) -> list[str]:
        return statement_entries(self.body)

    @property
    def entry_spans(self) -> list[tuple[str, SourceSpan]]:
        start_line = self.body_start_line or self.span.start_line
        return statement_entry_spans(self.body, start_line)


@dataclass(frozen=True)
class EventDecl:
    """A state-local event transition declaration."""

    name: str
    target_state: str
    span: SourceSpan
    depends_on: list[Block] = field(default_factory=list)
    drives: list[Block] = field(default_factory=list)
    may_change: list[Block] = field(default_factory=list)
    deferred: list[Block] = field(default_factory=list)
    other_blocks: list[Block] = field(default_factory=list)


@dataclass(frozen=True)
class StateDecl:
    """An object state declaration."""

    name: str
    span: SourceSpan
    invariants: list[Block] = field(default_factory=list)
    deferred: list[Block] = field(default_factory=list)
    events: list[EventDecl] = field(default_factory=list)
    other_blocks: list[Block] = field(default_factory=list)


@dataclass(frozen=True)
class ObjectDecl:
    """An object declaration."""

    name: str
    kind: str
    span: SourceSpan
    initial_state: str | None = None
    parent: str | None = None
    attrs: list[Block] = field(default_factory=list)
    references: list[Block] = field(default_factory=list)
    states: list[StateDecl] = field(default_factory=list)
    other_blocks: list[Block] = field(default_factory=list)
    properties: dict[str, str] = field(default_factory=dict)


@dataclass(frozen=True)
class TypeDecl:
    """A type declaration."""

    name: str
    header: str
    span: SourceSpan
    blocks: list[Block] = field(default_factory=list)


@dataclass(frozen=True)
class PredicateDecl:
    """A predicate declaration or definition."""

    name: str
    signature: str
    span: SourceSpan
    body: str | None = None


@dataclass(frozen=True)
class FunctionDecl:
    """A function declaration."""

    name: str
    signature: str
    span: SourceSpan


@dataclass(frozen=True)
class EnumDecl:
    """An enum declaration."""

    name: str
    variants: list[str]
    span: SourceSpan


@dataclass(frozen=True)
class SpecDocument:
    """Parsed syntax-level spec document."""

    enums: list[EnumDecl] = field(default_factory=list)
    functions: list[FunctionDecl] = field(default_factory=list)
    predicates: list[PredicateDecl] = field(default_factory=list)
    types: list[TypeDecl] = field(default_factory=list)
    objects: list[ObjectDecl] = field(default_factory=list)


def statement_entries(body: str) -> list[str]:
    """Split a raw block body into top-level semicolon-terminated entries."""

    return [entry for entry, _span in statement_entry_spans(body, 1)]


def statement_entry_spans(body: str, start_line: int) -> list[tuple[str, SourceSpan]]:
    """Split a raw block body into entries with line spans."""

    entries: list[str] = []
    start = 0
    depth = 0

    for index, char in enumerate(body):
        if char in "([{":
            depth += 1
        elif char in ")]}":
            depth = max(depth - 1, 0)
        elif char == ";" and depth == 0:
            entry = body[start:index].strip()
            if entry:
                entries.append((entry, _entry_span(body, start, index, start_line)))
            start = index + 1

    tail = body[start:].strip()
    if tail:
        entries.append((tail, _entry_span(body, start, len(body), start_line)))
    return entries


def _entry_span(body: str, start: int, end: int, start_line: int) -> SourceSpan:
    while start < end and body[start].isspace():
        start += 1
    while end > start and body[end - 1].isspace():
        end -= 1
    entry_start_line = start_line + body.count("\n", 0, start)
    entry_end_line = start_line + body.count("\n", 0, end)
    return SourceSpan(entry_start_line, entry_end_line)
