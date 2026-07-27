"""Syntax-level AST nodes shared by LKM verification tools."""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass(frozen=True)
class SourceSpan:
    """Line span in the source file."""

    start_line: int
    end_line: int
    source_file: str | None = None
    source_line: int | None = None


@dataclass(frozen=True)
class DriveStatement:
    """One source-ordered executable statement in a drives block."""

    kind: str
    text: str
    span: SourceSpan
    ordinal: int = 0
    alias: str | None = None
    declared_type: str | None = None
    owner_process: str | None = None


@dataclass(frozen=True)
class Block:
    """Raw block content preserved for later semantic analysis."""

    kind: str
    body: str
    span: SourceSpan
    header: str = ""
    body_start_line: int | None = None
    statements: list[DriveStatement] = field(default_factory=list)

    @property
    def entries(self) -> list[str]:
        return statement_entries(self.body)

    @property
    def entry_spans(self) -> list[tuple[str, SourceSpan]]:
        start_line = self.body_start_line or self.span.start_line
        return statement_entry_spans(self.body, start_line, source_file=self.span.source_file, source_line=self.span.source_line)


@dataclass(frozen=True)
class BoundaryDecl:
    """A structured deferred or trimmed responsibility boundary."""

    status: str
    id: str
    span: SourceSpan
    category: str | None = None
    summary: str | None = None
    evidence: list[Block] = field(default_factory=list)
    resolution: str | None = None
    property_counts: dict[str, int] = field(default_factory=dict)
    other_blocks: list[Block] = field(default_factory=list)
    unknown_properties: dict[str, str] = field(default_factory=dict)


@dataclass(frozen=True)
class WithinDecl:
    """A transition/action block executed within an exclusive context."""

    context: str
    span: SourceSpan
    only_once: bool = False
    parameters: dict[str, str] = field(default_factory=dict)
    entered_by: list[Block] = field(default_factory=list)
    depends_on: list[Block] = field(default_factory=list)
    drives: list[Block] = field(default_factory=list)
    within: list["WithinDecl"] = field(default_factory=list)
    exited_by: list[Block] = field(default_factory=list)
    may_change: list[Block] = field(default_factory=list)
    ensures: list[Block] = field(default_factory=list)
    boundaries: list[BoundaryDecl] = field(default_factory=list)
    deferred: list[Block] = field(default_factory=list)
    other_blocks: list[Block] = field(default_factory=list)
    body_members: list["BodyMember"] = field(default_factory=list)


@dataclass(frozen=True)
class TransitionDecl:
    """A state-local transition declaration."""

    name: str
    target_state: str
    span: SourceSpan
    parameters: tuple[tuple[str, str], ...] = ()
    depends_on: list[Block] = field(default_factory=list)
    drives: list[Block] = field(default_factory=list)
    emits: list[Block] = field(default_factory=list)
    within: list[WithinDecl] = field(default_factory=list)
    may_change: list[Block] = field(default_factory=list)
    ensures: list[Block] = field(default_factory=list)
    boundaries: list[BoundaryDecl] = field(default_factory=list)
    deferred: list[Block] = field(default_factory=list)
    other_blocks: list[Block] = field(default_factory=list)
    body_members: list["BodyMember"] = field(default_factory=list)


@dataclass(frozen=True)
class ProcessDecl:
    """A reusable Type process or object-local action declaration."""

    kind: str
    name: str
    span: SourceSpan
    parameters: tuple[tuple[str, str], ...] = ()
    return_type: str | None = None
    depends_on: list[Block] = field(default_factory=list)
    drives: list[Block] = field(default_factory=list)
    within: list[WithinDecl] = field(default_factory=list)
    may_change: list[Block] = field(default_factory=list)
    ensures: list[Block] = field(default_factory=list)
    result: list[Block] = field(default_factory=list)
    other_blocks: list[Block] = field(default_factory=list)
    body_members: list["BodyMember"] = field(default_factory=list)
    properties: dict[str, str] = field(default_factory=dict)


@dataclass(frozen=True)
class BodyMember:
    """A source-ordered transition/within body member."""

    kind: str
    span: SourceSpan
    block: Block | None = None
    within: WithinDecl | None = None
    boundary: BoundaryDecl | None = None


@dataclass(frozen=True)
class StateDecl:
    """An object state declaration."""

    name: str
    span: SourceSpan
    invariants: list[Block] = field(default_factory=list)
    boundaries: list[BoundaryDecl] = field(default_factory=list)
    deferred: list[Block] = field(default_factory=list)
    transitions: list[TransitionDecl] = field(default_factory=list)
    processes: list[ProcessDecl] = field(default_factory=list)
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
    processes: list[ProcessDecl] = field(default_factory=list)
    other_blocks: list[Block] = field(default_factory=list)
    properties: dict[str, str] = field(default_factory=dict)


@dataclass(frozen=True)
class TypeDecl:
    """A type declaration."""

    name: str
    header: str
    span: SourceSpan
    initial_state: str | None = None
    states: list[StateDecl] = field(default_factory=list)
    blocks: list[Block] = field(default_factory=list)
    processes: list[ProcessDecl] = field(default_factory=list)
    properties: dict[str, str] = field(default_factory=dict)


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
class LockDecl:
    """A lock declaration used by exclusive contexts."""

    name: str
    span: SourceSpan
    kind: str | None = None


@dataclass(frozen=True)
class ContextGuardDecl:
    """A guard that establishes context entry and exit boundaries."""

    span: SourceSpan
    lock_ref: str | None = None
    entered_by: list[Block] = field(default_factory=list)
    exited_by: list[Block] = field(default_factory=list)
    holds: list[Block] = field(default_factory=list)
    other_blocks: list[Block] = field(default_factory=list)
    properties: dict[str, str] = field(default_factory=dict)


@dataclass(frozen=True)
class ExclusiveContextDecl:
    """An exclusive context declaration."""

    name: str
    span: SourceSpan
    kind: str | None = None
    guard: ContextGuardDecl | None = None
    lock_ref: str | None = None
    obj_refs: list[str] = field(default_factory=list)
    effects: list[Block] = field(default_factory=list)
    other_blocks: list[Block] = field(default_factory=list)
    properties: dict[str, str] = field(default_factory=dict)


@dataclass(frozen=True)
class ExternalDecl:
    """A model-external, lifecycle-free Signal orchestration declaration."""

    name: str
    span: SourceSpan
    drives: list[Block] = field(default_factory=list)
    emits: list[Block] = field(default_factory=list)


@dataclass(frozen=True)
class SpecDocument:
    """Parsed syntax-level spec document."""

    enums: list[EnumDecl] = field(default_factory=list)
    functions: list[FunctionDecl] = field(default_factory=list)
    predicates: list[PredicateDecl] = field(default_factory=list)
    types: list[TypeDecl] = field(default_factory=list)
    locks: list[LockDecl] = field(default_factory=list)
    exclusive_contexts: list[ExclusiveContextDecl] = field(default_factory=list)
    externals: list[ExternalDecl] = field(default_factory=list)
    objects: list[ObjectDecl] = field(default_factory=list)


def statement_entries(body: str, *, source_file: str | None = None, source_line: int | None = None) -> list[str]:
    """Split a raw block body into top-level semicolon-terminated entries."""

    return [entry for entry, _span in statement_entry_spans(body, 1, source_file=source_file, source_line=source_line)]


def statement_entry_spans(body: str, start_line: int, *, source_file: str | None = None, source_line: int | None = None) -> list[tuple[str, SourceSpan]]:
    """Split a raw block body into entries with line spans."""

    entries: list[tuple[str, SourceSpan]] = []
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
                entries.append((entry, _entry_span(body, start, index, start_line, source_file=source_file, source_line=source_line)))
            start = index + 1

    tail = body[start:].strip()
    if tail:
        entries.append((tail, _entry_span(body, start, len(body), start_line, source_file=source_file, source_line=source_line)))
    return entries


def _entry_span(
    body: str, start: int, end: int, start_line: int, *, source_file: str | None = None, source_line: int | None = None
) -> SourceSpan:
    while start < end and body[start].isspace():
        start += 1
    while end > start and body[end - 1].isspace():
        end -= 1
    entry_start_line = start_line + body.count("\n", 0, start)
    entry_end_line = start_line + body.count("\n", 0, end)
    if source_line is not None:
        entry_source_line = source_line + (entry_start_line - start_line)
    else:
        entry_source_line = None
    return SourceSpan(entry_start_line, entry_end_line, source_file=source_file, source_line=entry_source_line)
