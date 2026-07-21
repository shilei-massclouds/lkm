"""Parser for pyveri object model specs."""

from __future__ import annotations

import ast
import os
import re
from dataclasses import dataclass, replace as dc_replace
from pathlib import Path

from common.spec_ast import (
    BoundaryDecl,
    BodyMember,
    Block,
    ContextGuardDecl,
    EnumDecl,
    TransitionDecl,
    ExclusiveContextDecl,
    FunctionDecl,
    LockDecl,
    ObjectDecl,
    ProcessDecl,
    PredicateDecl,
    SourceSpan,
    SpecDocument,
    StateDecl,
    TypeDecl,
    WithinDecl,
    DriveStatement,
    statement_entries,
)


class ParseError(Exception):
    """Raised when the input spec cannot be parsed."""


@dataclass(frozen=True)
class _Segment:
    text: str
    start_line: int
    end_line: int

    @property
    def span(self) -> SourceSpan:
        return SourceSpan(self.start_line, self.end_line)


_IDENT = r"[A-Za-z_][A-Za-z0-9_]*"
_ENUM_RE = re.compile(rf"\Aenum\s+({_IDENT})\s*\{{", re.S)
_TYPE_RE = re.compile(rf"\Atype\s+({_IDENT})(?P<header>[^\{{]*)\{{", re.S)
_LOCK_RE = re.compile(rf"\Alock\s+({_IDENT})(?:\s*:\s*({_IDENT}))?\s*;\Z", re.S)
_EXCLUSIVE_CONTEXT_RE = re.compile(rf"\Aexclusive_context\s+({_IDENT})\s*\{{", re.S)
_CONTEXT_RE = re.compile(rf"\Acontext\s+({_IDENT})\s*:\s*({_IDENT})\s*\{{", re.S)
_OBJECT_RE = re.compile(rf"\Aobject\s+({_IDENT})\s*:\s*({_IDENT})\s*\{{", re.S)
_FUNCTION_RE = re.compile(rf"\Afunction\s+({_IDENT})(?P<sig>.*);?\Z", re.S)
_PREDICATE_RE = re.compile(rf"\Apredicate\s+({_IDENT})(?P<rest>.*)\Z", re.S)
_STATE_RE = re.compile(rf"\Astate\s+State::({_IDENT})\s*\{{", re.S)
_TRANSITION_RE = re.compile(
    rf"\Aon\s+Transition::({_IDENT})"
    rf"\s*(?:\(([^{{}};]*)\))?\s*->\s*State::({_IDENT})\s*\{{",
    re.S,
)
_PROCESS_RE = re.compile(
    rf"\A(?:on\s+)?(Transition|Action)::({_IDENT})"
    rf"\s*(?:<[^{{}};]*>)?"
    rf"\s*(?:\(([^{{}};]*)\))?\s*(?:->\s*({_IDENT}))?\s*\{{",
    re.S,
)
_DECLARE_RE = re.compile(rf"\Adeclare\s+([a-z][A-Za-z0-9_]*)\s+of\s+({_IDENT})\Z")
_BLOCK_RE = re.compile(rf"\A({_IDENT})(?P<header>[^\{{]*)\{{", re.S)
_PROP_RE = re.compile(rf"\A({_IDENT})\s*:\s*(.+?)\s*;\Z", re.S)
_INCLUDE_LINE_RE = re.compile(r'\A\s*include\s+"([^"]+)"\s*;\s*\Z')


def parse_file(path: str | Path) -> SpecDocument:
    """Parse a spec file."""

    resolved = Path(path).resolve()
    text, line_to_file, line_to_local = _read_with_includes(resolved, seen=set(), stack=[])
    line_to_file, line_to_local = _to_relative_paths(line_to_file, line_to_local)
    try:
        document = parse_text(text)
    except ParseError as exc:
        raise _enrich_parse_error(exc, line_to_file) from None
    return _enrich_document(document, line_to_file, line_to_local)


def _to_relative_paths(
    line_to_file: list[str], line_to_local: list[int]
) -> tuple[list[str], list[int]]:
    """Convert absolute paths in line_to_file to paths relative to the project root."""
    project_root = _find_project_root()
    if project_root is None:
        return [os.path.relpath(f) for f in line_to_file], line_to_local
    return [os.path.relpath(f, project_root) for f in line_to_file], line_to_local


def _find_project_root() -> str | None:
    """Find the project root by walking up to a directory containing .git."""
    d = Path.cwd()
    for _ in range(10):
        if (d / ".git").exists():
            return str(d)
        parent = d.parent
        if parent == d:
            break
        d = parent
    return None


def parse_text(text: str) -> SpecDocument:
    """Parse spec source text into a syntax-level AST."""

    stripped = strip_comments(text)
    segments = _top_level_segments(stripped)

    enums: list[EnumDecl] = []
    functions: list[FunctionDecl] = []
    predicates: list[PredicateDecl] = []
    types: list[TypeDecl] = []
    locks: list[LockDecl] = []
    exclusive_contexts: list[ExclusiveContextDecl] = []
    objects: list[ObjectDecl] = []

    for segment in segments:
        head = segment.text.lstrip()
        if head.startswith("enum "):
            enums.append(_parse_enum(segment))
        elif head.startswith("function "):
            functions.append(_parse_function(segment))
        elif head.startswith("predicate "):
            predicates.append(_parse_predicate(segment))
        elif head.startswith("type "):
            types.append(_parse_type(segment))
        elif head.startswith("lock "):
            locks.append(_parse_lock(segment))
        elif head.startswith("exclusive_context "):
            exclusive_contexts.append(_parse_exclusive_context(segment))
        elif head.startswith("context "):
            exclusive_contexts.append(_parse_context(segment))
        elif head.startswith("object "):
            objects.append(_parse_object(segment))
        else:
            raise ParseError(
                f"line {segment.start_line}: unknown top-level declaration: "
                f"{_preview(segment.text)}"
            )

    return SpecDocument(
        enums=enums,
        functions=functions,
        predicates=predicates,
        types=types,
        locks=locks,
        exclusive_contexts=exclusive_contexts,
        objects=objects,
    )


def _enrich_parse_error(exc: ParseError, line_to_file: list[str]) -> ParseError:
    """Add source file information to a ParseError message when available."""
    msg = str(exc)
    if not msg.startswith("line ") or not line_to_file:
        return exc
    try:
        rest = msg[len("line "):]
        line_str, _, detail = rest.partition(":")
        merged_line = int(line_str.strip())
        if 1 <= merged_line <= len(line_to_file):
            source_file = line_to_file[merged_line - 1]
            return ParseError(f"{source_file}:{merged_line}:{detail}")
    except (ValueError, IndexError):
        pass
    return exc


def _enrich_document(
    document: SpecDocument, line_to_file: list[str], line_to_local: list[int]
) -> SpecDocument:
    """Reconstruct a SpecDocument with source_file filled in on all spans."""
    return dc_replace(
        document,
        enums=[_with_file_node(e, line_to_file, line_to_local) for e in document.enums],
        functions=[_with_file_node(f, line_to_file, line_to_local) for f in document.functions],
        predicates=[_with_file_node(p, line_to_file, line_to_local) for p in document.predicates],
        types=[_enrich_type(t, line_to_file, line_to_local) for t in document.types],
        locks=[_with_file_node(lk, line_to_file, line_to_local) for lk in document.locks],
        exclusive_contexts=[_enrich_ctx(c, line_to_file, line_to_local) for c in document.exclusive_contexts],
        objects=[_enrich_obj(o, line_to_file, line_to_local) for o in document.objects],
    )


def _enrich_ctx(
    ctx: "ExclusiveContextDecl", lf: list[str], ll: list[int]
) -> "ExclusiveContextDecl":
    guard = None
    if ctx.guard is not None:
        guard = dc_replace(
            ctx.guard,
            span=_with_file(ctx.guard.span, lf, ll),
            entered_by=[_with_file_block(b, lf, ll) for b in ctx.guard.entered_by],
            exited_by=[_with_file_block(b, lf, ll) for b in ctx.guard.exited_by],
            holds=[_with_file_block(b, lf, ll) for b in ctx.guard.holds],
            other_blocks=[_with_file_block(b, lf, ll) for b in ctx.guard.other_blocks],
        )
    return dc_replace(
        ctx,
        span=_with_file(ctx.span, lf, ll),
        guard=guard,
        effects=[_with_file_block(b, lf, ll) for b in ctx.effects],
        other_blocks=[_with_file_block(b, lf, ll) for b in ctx.other_blocks],
    )


def _enrich_obj(
    obj: "ObjectDecl", lf: list[str], ll: list[int]
) -> "ObjectDecl":
    return dc_replace(
        obj,
        span=_with_file(obj.span, lf, ll),
        attrs=[_with_file_block(b, lf, ll) for b in obj.attrs],
        references=[_with_file_block(b, lf, ll) for b in obj.references],
        states=[_enrich_state(s, lf, ll) for s in obj.states],
        processes=[_enrich_process(p, lf, ll) for p in obj.processes],
        other_blocks=[_with_file_block(b, lf, ll) for b in obj.other_blocks],
    )


def _enrich_state(
    state: "StateDecl", lf: list[str], ll: list[int]
) -> "StateDecl":
    return dc_replace(
        state,
        span=_with_file(state.span, lf, ll),
        invariants=[_with_file_block(b, lf, ll) for b in state.invariants],
        boundaries=[_enrich_boundary(b, lf, ll) for b in state.boundaries],
        deferred=[_with_file_block(b, lf, ll) for b in state.deferred],
        transitions=[_enrich_tr(t, lf, ll) for t in state.transitions],
        processes=[_enrich_process(p, lf, ll) for p in state.processes],
        other_blocks=[_with_file_block(b, lf, ll) for b in state.other_blocks],
    )


def _enrich_tr(
    tr: "TransitionDecl", lf: list[str], ll: list[int]
) -> "TransitionDecl":
    return dc_replace(
        tr,
        span=_with_file(tr.span, lf, ll),
        depends_on=[_with_file_block(b, lf, ll) for b in tr.depends_on],
        drives=[_with_file_block(b, lf, ll) for b in tr.drives],
        emits=[_with_file_block(b, lf, ll) for b in tr.emits],
        within=[_enrich_within(w, lf, ll) for w in tr.within],
        may_change=[_with_file_block(b, lf, ll) for b in tr.may_change],
        ensures=[_with_file_block(b, lf, ll) for b in tr.ensures],
        boundaries=[_enrich_boundary(b, lf, ll) for b in tr.boundaries],
        deferred=[_with_file_block(b, lf, ll) for b in tr.deferred],
        other_blocks=[_with_file_block(b, lf, ll) for b in tr.other_blocks],
        body_members=[_enrich_bm(bm, lf, ll) for bm in tr.body_members],
    )


def _enrich_within(
    w: "WithinDecl", lf: list[str], ll: list[int]
) -> "WithinDecl":
    return dc_replace(
        w,
        span=_with_file(w.span, lf, ll),
        entered_by=[_with_file_block(b, lf, ll) for b in w.entered_by],
        depends_on=[_with_file_block(b, lf, ll) for b in w.depends_on],
        drives=[_with_file_block(b, lf, ll) for b in w.drives],
        within=[_enrich_within(n, lf, ll) for n in w.within],
        exited_by=[_with_file_block(b, lf, ll) for b in w.exited_by],
        may_change=[_with_file_block(b, lf, ll) for b in w.may_change],
        ensures=[_with_file_block(b, lf, ll) for b in w.ensures],
        boundaries=[_enrich_boundary(b, lf, ll) for b in w.boundaries],
        deferred=[_with_file_block(b, lf, ll) for b in w.deferred],
        other_blocks=[_with_file_block(b, lf, ll) for b in w.other_blocks],
        body_members=[_enrich_bm(bm, lf, ll) for bm in w.body_members],
    )


def _enrich_bm(bm: "BodyMember", lf: list[str], ll: list[int]) -> "BodyMember":
    block = None
    if bm.block is not None:
        block = _with_file_block(bm.block, lf, ll)
    within = None
    if bm.within is not None:
        within = _enrich_within(bm.within, lf, ll)
    boundary = None
    if bm.boundary is not None:
        boundary = _enrich_boundary(bm.boundary, lf, ll)
    return dc_replace(
        bm,
        span=_with_file(bm.span, lf, ll),
        block=block,
        within=within,
        boundary=boundary,
    )


def _enrich_boundary(
    boundary: BoundaryDecl, lf: list[str], ll: list[int]
) -> BoundaryDecl:
    return dc_replace(
        boundary,
        span=_with_file(boundary.span, lf, ll),
        evidence=[_with_file_block(block, lf, ll) for block in boundary.evidence],
        other_blocks=[
            _with_file_block(block, lf, ll) for block in boundary.other_blocks
        ],
    )


def _enrich_type(t: "TypeDecl", lf: list[str], ll: list[int]) -> "TypeDecl":
    return dc_replace(
        t,
        span=_with_file(t.span, lf, ll),
        states=[_enrich_state(s, lf, ll) for s in t.states],
        blocks=[_with_file_block(b, lf, ll) for b in t.blocks],
        processes=[_enrich_process(p, lf, ll) for p in t.processes],
    )


def _enrich_process(
    process: ProcessDecl, lf: list[str], ll: list[int]
) -> ProcessDecl:
    return dc_replace(
        process,
        span=_with_file(process.span, lf, ll),
        depends_on=[_with_file_block(b, lf, ll) for b in process.depends_on],
        drives=[_with_file_block(b, lf, ll) for b in process.drives],
        within=[_enrich_within(w, lf, ll) for w in process.within],
        may_change=[_with_file_block(b, lf, ll) for b in process.may_change],
        ensures=[_with_file_block(b, lf, ll) for b in process.ensures],
        result=[_with_file_block(b, lf, ll) for b in process.result],
        other_blocks=[_with_file_block(b, lf, ll) for b in process.other_blocks],
        body_members=[_enrich_bm(bm, lf, ll) for bm in process.body_members],
    )


def _with_file_node(node, lf: list[str], ll: list[int]):
    return dc_replace(node, span=_with_file(node.span, lf, ll))


def _with_file_block(block: "Block", lf: list[str], ll: list[int]) -> "Block":
    return dc_replace(
        block,
        span=_with_file(block.span, lf, ll),
        statements=[
            dc_replace(statement, span=_with_file(statement.span, lf, ll))
            for statement in block.statements
        ],
    )


def _with_file(
    span: SourceSpan, line_to_file: list[str], line_to_local: list[int]
) -> SourceSpan:
    if span.source_file is not None:
        return span
    line = span.start_line
    if 1 <= line <= len(line_to_file):
        return SourceSpan(
            span.start_line, span.end_line,
            source_file=line_to_file[line - 1],
            source_line=line_to_local[line - 1],
        )
    return span


def _read_with_includes(
    path: Path, seen: set[Path], stack: list[Path]
) -> tuple[str, list[str], list[int]]:
    return _read_with_include_mode(path, seen, stack, strip=True)


def _read_source_with_includes(
    path: Path, seen: set[Path], stack: list[Path]
) -> tuple[str, list[str], list[int]]:
    return _read_with_include_mode(path, seen, stack, strip=False)


def _read_with_include_mode(
    path: Path, seen: set[Path], stack: list[Path], *, strip: bool
) -> tuple[str, list[str], list[int]]:
    resolved = path.resolve()
    if resolved in stack:
        cycle = " -> ".join(str(item) for item in [*stack, resolved])
        raise ParseError(f"include cycle: {cycle}")
    if resolved in seen:
        return "\n", [], []

    seen.add(resolved)
    raw = resolved.read_text(encoding="utf-8")
    text = strip_comments(raw) if strip else raw
    return _expand_includes(text, resolved.parent, seen, [*stack, resolved], strip=strip)


def _expand_includes(
    text: str, base_dir: Path, seen: set[Path], stack: list[Path], *, strip: bool
) -> tuple[str, list[str], list[int]]:
    lines: list[str] = []
    line_to_file: list[str] = []
    line_to_local: list[int] = []
    current_file = str(stack[-1])
    current_local = 1
    for raw_line in text.splitlines(keepends=True):
        match = _INCLUDE_LINE_RE.match(raw_line.rstrip("\n"))
        if not match:
            lines.append(raw_line)
            line_to_file.append(current_file)
            line_to_local.append(current_local)
            current_local += 1
            continue
        include_path = (base_dir / match.group(1)).resolve()
        inc_text, inc_lf, inc_ll = _read_with_include_mode(
            include_path, seen, stack, strip=strip
        )
        inc_line_count = inc_text.count("\n") + (1 if inc_text and not inc_text.endswith("\n") else 0)
        lines.append(inc_text)
        line_to_file.extend(inc_lf)
        line_to_local.extend(inc_ll)
        if inc_text and not inc_text.endswith("\n"):
            lines.append("\n")
            line_to_file.append(current_file)
            line_to_local.append(current_local)
        current_local += 1
    return "".join(lines), line_to_file, line_to_local


def strip_comments(text: str) -> str:
    """Remove Rust/C style comments while preserving line positions."""

    result: list[str] = []
    index = 0
    length = len(text)
    in_string = False

    while index < length:
        char = text[index]
        nxt = text[index + 1] if index + 1 < length else ""

        if in_string:
            result.append(char)
            if char == "\\" and index + 1 < length:
                index += 1
                result.append(text[index])
            elif char == '"':
                in_string = False
            index += 1
            continue

        if char == '"':
            in_string = True
            result.append(char)
            index += 1
            continue

        if char == "/" and nxt == "/":
            result.extend("  ")
            index += 2
            while index < length and text[index] != "\n":
                result.append(" ")
                index += 1
            continue

        if char == "/" and nxt == "*":
            result.extend("  ")
            index += 2
            closed = False
            while index < length:
                if text[index] == "*" and index + 1 < length and text[index + 1] == "/":
                    result.extend("  ")
                    index += 2
                    closed = True
                    break
                result.append("\n" if text[index] == "\n" else " ")
                index += 1
            if not closed:
                raise ParseError("unterminated block comment")
            continue

        result.append(char)
        index += 1

    return "".join(result)


def _parse_enum(segment: _Segment) -> EnumDecl:
    match = _ENUM_RE.match(segment.text)
    if not match:
        raise ParseError(f"line {segment.start_line}: invalid enum declaration")
    body, _body_start_line = _body_segment_from_braced_decl(
        segment.text, match.end() - 1, segment.start_line
    )
    variants = [part.strip().rstrip(",") for part in body.splitlines()]
    variants = [variant for variant in variants if variant]
    return EnumDecl(match.group(1), variants, segment.span)


def _parse_function(segment: _Segment) -> FunctionDecl:
    match = _FUNCTION_RE.match(segment.text.strip())
    if not match:
        raise ParseError(f"line {segment.start_line}: invalid function declaration")
    return FunctionDecl(match.group(1), match.group("sig").strip(), segment.span)


def _parse_predicate(segment: _Segment) -> PredicateDecl:
    text = segment.text.strip()
    match = _PREDICATE_RE.match(text)
    if not match:
        raise ParseError(f"line {segment.start_line}: invalid predicate declaration")
    name = match.group(1)
    rest = match.group("rest").strip()
    brace = _find_top_level_char(rest, "{")
    if brace is None:
        signature = rest.rstrip(";").strip()
        body = None
    else:
        signature = rest[:brace].strip()
        body, _body_start_line = _body_segment_from_braced_decl(
            rest, brace, segment.start_line
        )
    return PredicateDecl(name, signature, segment.span, body)


def _parse_type(segment: _Segment) -> TypeDecl:
    match = _TYPE_RE.match(segment.text)
    if not match:
        raise ParseError(f"line {segment.start_line}: invalid type declaration")
    body, body_start_line = _body_segment_from_braced_decl(
        segment.text, match.end() - 1, segment.start_line
    )
    blocks: list[Block] = []
    processes: list[ProcessDecl] = []
    states: list[StateDecl] = []
    initial_state: str | None = None
    properties: dict[str, str] = {}
    for part in _split_members(body, body_start_line):
        stripped = part.text.strip()
        state_match = _STATE_RE.match(stripped)
        block_match = _BLOCK_RE.match(stripped)
        if state_match:
            states.append(_parse_state(part, owner=match.group(1)))
            continue
        if block_match:
            block = _to_block(part, block_match.group(1))
            blocks.append(block)
            if block.kind in {"lifecycle", "processes", "transitions", "actions"}:
                processes.extend(_parse_process_container(block, owner=match.group(1)))
            continue
        prop_match = _PROP_RE.match(stripped)
        if not prop_match:
            raise ParseError(
                f"line {part.start_line}: invalid type member: {_preview(part.text)}"
            )
        key = prop_match.group(1)
        value = prop_match.group(2).strip()
        properties[key] = value
        if key == "initial_state":
            initial_state = _state_name(value)
    return TypeDecl(
        name=match.group(1),
        header=match.group("header").strip(),
        span=segment.span,
        initial_state=initial_state,
        states=states,
        blocks=blocks,
        processes=processes,
        properties=properties,
    )




def _parse_lock(segment: _Segment) -> LockDecl:
    match = _LOCK_RE.match(segment.text.strip())
    if not match:
        raise ParseError(f"line {segment.start_line}: invalid lock declaration")
    return LockDecl(match.group(1), segment.span, match.group(2))


def _parse_exclusive_context(segment: _Segment) -> ExclusiveContextDecl:
    match = _EXCLUSIVE_CONTEXT_RE.match(segment.text)
    if not match:
        raise ParseError(f"line {segment.start_line}: invalid exclusive_context declaration")

    return _parse_context_body(segment, match, kind=None)


def _parse_context(segment: _Segment) -> ExclusiveContextDecl:
    match = _CONTEXT_RE.match(segment.text)
    if not match:
        raise ParseError(f"line {segment.start_line}: invalid context declaration")

    return _parse_context_body(segment, match, kind=match.group(2))


def _parse_context_body(
    segment: _Segment, match: re.Match[str], *, kind: str | None
) -> ExclusiveContextDecl:
    body, body_start_line = _body_segment_from_braced_decl(
        segment.text, match.end() - 1, segment.start_line
    )
    parts = _split_members(body, body_start_line)
    lock_ref: str | None = None
    guard: ContextGuardDecl | None = None
    obj_refs: list[str] = []
    effects: list[Block] = []
    other_blocks: list[Block] = []
    properties: dict[str, str] = {}

    for part in parts:
        stripped = part.text.strip()
        block_match = _BLOCK_RE.match(stripped)
        if block_match:
            block = _to_block(part, block_match.group(1))
            if block.kind == "guard":
                if guard is not None:
                    raise ParseError(
                        f"line {part.start_line}: duplicate context guard block"
                    )
                guard = _parse_context_guard(block)
                if guard.lock_ref is not None:
                    lock_ref = guard.lock_ref
            elif block.kind == "obj_refs":
                obj_refs.extend(block.entries)
            elif block.kind == "effects":
                effects.append(block)
            else:
                other_blocks.append(block)
            continue

        prop_match = _PROP_RE.match(stripped)
        if not prop_match:
            raise ParseError(
                f"line {part.start_line}: invalid exclusive_context member: {_preview(part.text)}"
            )
        key = prop_match.group(1)
        value = prop_match.group(2).strip()
        properties[key] = value
        if key == "lock_ref":
            lock_ref = value

    return ExclusiveContextDecl(
        name=match.group(1),
        span=segment.span,
        kind=kind,
        guard=guard,
        lock_ref=lock_ref,
        obj_refs=obj_refs,
        effects=effects,
        other_blocks=other_blocks,
        properties=properties,
    )


def _parse_context_guard(block: Block) -> ContextGuardDecl:
    header = block.header.strip()
    if header:
        raise ParseError(f"line {block.span.start_line}: guard block must not declare kind")

    lock_ref: str | None = None
    entered_by: list[Block] = []
    exited_by: list[Block] = []
    holds: list[Block] = []
    other_blocks: list[Block] = []
    properties: dict[str, str] = {}

    for part in _split_members(block.body, block.body_start_line or block.span.start_line):
        stripped = part.text.strip()
        block_match = _BLOCK_RE.match(stripped)
        if block_match:
            child = _to_block(part, block_match.group(1))
            if child.kind == "entered_by":
                entered_by.append(child)
            elif child.kind == "exited_by":
                exited_by.append(child)
            elif child.kind == "holds":
                holds.append(child)
            else:
                other_blocks.append(child)
            continue

        prop_match = _PROP_RE.match(stripped)
        if not prop_match:
            raise ParseError(
                f"line {part.start_line}: invalid guard member: {_preview(part.text)}"
            )
        key = prop_match.group(1)
        value = prop_match.group(2).strip()
        properties[key] = value
        if key == "lock_ref":
            lock_ref = value

    return ContextGuardDecl(
        span=block.span,
        lock_ref=lock_ref,
        entered_by=entered_by,
        exited_by=exited_by,
        holds=holds,
        other_blocks=other_blocks,
        properties=properties,
    )


def _parse_object(segment: _Segment) -> ObjectDecl:
    match = _OBJECT_RE.match(segment.text)
    if not match:
        raise ParseError(f"line {segment.start_line}: invalid object declaration")

    body, body_start_line = _body_segment_from_braced_decl(
        segment.text, match.end() - 1, segment.start_line
    )
    parts = _split_members(body, body_start_line)

    initial_state: str | None = None
    parent: str | None = None
    attrs: list[Block] = []
    references: list[Block] = []
    states: list[StateDecl] = []
    processes: list[ProcessDecl] = []
    other_blocks: list[Block] = []
    properties: dict[str, str] = {}

    for part in parts:
        stripped = part.text.strip()
        state_match = _STATE_RE.match(stripped)
        block_match = _BLOCK_RE.match(stripped)

        if state_match:
            states.append(_parse_state(part, owner=match.group(1)))
            continue

        if block_match:
            block = _to_block(part, block_match.group(1))
            if block.kind == "attrs":
                attrs.append(block)
            elif block.kind == "reference":
                references.append(block)
            elif block.kind in {"actions", "processes", "lifecycle"}:
                processes.extend(_parse_process_container(block, owner=match.group(1)))
                other_blocks.append(block)
            else:
                other_blocks.append(block)
            continue

        prop_match = _PROP_RE.match(stripped)
        if not prop_match:
            raise ParseError(
                f"line {part.start_line}: invalid object member: {_preview(part.text)}"
            )
        key = prop_match.group(1)
        value = prop_match.group(2).strip()
        properties[key] = value
        if key == "initial_state":
            initial_state = _state_name(value)
        elif key == "parent":
            parent = value

    return ObjectDecl(
        name=match.group(1),
        kind=match.group(2),
        span=segment.span,
        initial_state=initial_state,
        parent=parent,
        attrs=attrs,
        references=references,
        states=states,
        processes=processes,
        other_blocks=other_blocks,
        properties=properties,
    )


def _parse_state(segment: _Segment, *, owner: str) -> StateDecl:
    match = _STATE_RE.match(segment.text)
    if not match:
        raise ParseError(f"line {segment.start_line}: invalid state declaration")

    body, body_start_line = _body_segment_from_braced_decl(
        segment.text, match.end() - 1, segment.start_line
    )
    parts = _split_members(body, body_start_line)

    invariants: list[Block] = []
    boundaries: list[BoundaryDecl] = []
    deferred: list[Block] = []
    transitions: list[TransitionDecl] = []
    processes: list[ProcessDecl] = []
    other_blocks: list[Block] = []

    for part in parts:
        stripped = part.text.strip()
        event_match = _TRANSITION_RE.match(stripped)
        block_match = _BLOCK_RE.match(stripped)

        if event_match:
            transitions.append(_parse_event(part, owner=owner))
            continue

        if not block_match:
            raise ParseError(
                f"line {part.start_line}: invalid state member: {_preview(part.text)}"
            )

        block = _to_block(part, block_match.group(1))
        if block.kind == "invariant":
            invariants.append(block)
        elif block.kind == "deferred":
            if block.header.strip():
                boundaries.append(_parse_boundary(block))
            else:
                deferred.append(block)
        elif block.kind == "trimmed":
            boundaries.append(_parse_boundary(block))
        elif block.kind == "transitions":
            transitions.extend(_parse_events_block(block, owner=owner))
        elif block.kind in {"actions", "processes"}:
            processes.extend(_parse_process_container(block, owner=owner))
            other_blocks.append(block)
        else:
            other_blocks.append(block)

    return StateDecl(
        name=match.group(1),
        span=segment.span,
        invariants=invariants,
        boundaries=boundaries,
        deferred=deferred,
        transitions=transitions,
        processes=processes,
        other_blocks=other_blocks,
    )


def _parse_events_block(block: Block, *, owner: str) -> list[TransitionDecl]:
    parts = _split_members(block.body, block.body_start_line or block.span.start_line)
    transitions: list[TransitionDecl] = []
    for part in parts:
        if not _TRANSITION_RE.match(part.text.strip()):
            raise ParseError(
                f"line {part.start_line}: invalid transitions member: {_preview(part.text)}"
            )
        transitions.append(_parse_event(part, owner=owner))
    return transitions


def _parse_event(segment: _Segment, *, owner: str) -> TransitionDecl:
    match = _TRANSITION_RE.match(segment.text)
    if not match:
        raise ParseError(f"line {segment.start_line}: invalid transition declaration")

    body, body_start_line = _body_segment_from_braced_decl(
        segment.text, match.end() - 1, segment.start_line
    )
    parts = _split_members(body, body_start_line)

    depends_on: list[Block] = []
    drives: list[Block] = []
    emits: list[Block] = []
    within: list[WithinDecl] = []
    may_change: list[Block] = []
    ensures: list[Block] = []
    boundaries: list[BoundaryDecl] = []
    deferred: list[Block] = []
    other_blocks: list[Block] = []
    body_members: list[BodyMember] = []

    for part in parts:
        block_match = _BLOCK_RE.match(part.text.strip())
        if not block_match:
            raise ParseError(
                f"line {part.start_line}: invalid transition member: {_preview(part.text)}"
            )
        block = _to_block(part, block_match.group(1))
        if block.kind == "depends_on":
            depends_on.append(block)
            body_members.append(_block_body_member(block))
        elif block.kind == "drives":
            drives.append(block)
            body_members.append(_block_body_member(block))
        elif block.kind == "emits":
            emits.append(block)
            body_members.append(_block_body_member(block))
        elif block.kind == "within":
            child = _parse_within(block)
            within.append(child)
            body_members.append(_within_body_member(child))
        elif block.kind == "may_change":
            may_change.append(block)
            body_members.append(_block_body_member(block))
        elif block.kind == "ensures":
            ensures.append(block)
            body_members.append(_block_body_member(block))
        elif block.kind == "deferred":
            if block.header.strip():
                boundary = _parse_boundary(block)
                boundaries.append(boundary)
                body_members.append(_boundary_body_member(boundary))
            else:
                deferred.append(block)
                body_members.append(_block_body_member(block))
        elif block.kind == "trimmed":
            boundary = _parse_boundary(block)
            boundaries.append(boundary)
            body_members.append(_boundary_body_member(boundary))
        else:
            other_blocks.append(block)
            body_members.append(_block_body_member(block))

    transition = TransitionDecl(
        name=match.group(1),
        target_state=match.group(3),
        span=segment.span,
        parameters=_parse_process_parameters(match.group(2) or "", segment.start_line),
        depends_on=depends_on,
        drives=drives,
        emits=emits,
        within=within,
        may_change=may_change,
        ensures=ensures,
        boundaries=boundaries,
        deferred=deferred,
        other_blocks=other_blocks,
        body_members=body_members,
    )
    return _number_transition_statements(
        transition, f"{owner}.Transition::{transition.name}"
    )


def _parse_process_container(block: Block, *, owner: str) -> list[ProcessDecl]:
    processes: list[ProcessDecl] = []
    for part in _split_members(
        block.body, block.body_start_line or block.span.start_line
    ):
        if _PROCESS_RE.match(part.text.strip()) is None:
            raise ParseError(
                f"line {part.start_line}: invalid {block.kind} process member: "
                f"{_preview(part.text)}"
            )
        processes.append(_parse_process(part, owner=owner))
    return processes


def _parse_process(segment: _Segment, *, owner: str) -> ProcessDecl:
    match = _PROCESS_RE.match(segment.text.strip())
    if match is None:
        raise ParseError(f"line {segment.start_line}: invalid process declaration")
    body, body_start_line = _body_segment_from_braced_decl(
        segment.text, match.end() - 1, segment.start_line
    )
    depends_on: list[Block] = []
    drives: list[Block] = []
    within: list[WithinDecl] = []
    may_change: list[Block] = []
    ensures: list[Block] = []
    result: list[Block] = []
    other_blocks: list[Block] = []
    body_members: list[BodyMember] = []
    properties: dict[str, str] = {}

    for part in _split_members(body, body_start_line):
        block_match = _BLOCK_RE.match(part.text.strip())
        if block_match is None:
            prop_match = _PROP_RE.match(part.text.strip())
            if prop_match is None:
                raise ParseError(
                    f"line {part.start_line}: invalid process member: {_preview(part.text)}"
                )
            properties[prop_match.group(1)] = prop_match.group(2).strip()
            continue
        child = _to_block(part, block_match.group(1))
        if child.kind == "depends_on":
            depends_on.append(child)
            body_members.append(_block_body_member(child))
        elif child.kind == "drives":
            drives.append(child)
            body_members.append(_block_body_member(child))
        elif child.kind == "within":
            nested = _parse_within(child)
            within.append(nested)
            body_members.append(_within_body_member(nested))
        elif child.kind == "may_change":
            may_change.append(child)
            body_members.append(_block_body_member(child))
        elif child.kind == "ensures":
            ensures.append(child)
            body_members.append(_block_body_member(child))
        elif child.kind == "result":
            result.append(child)
            body_members.append(_block_body_member(child))
        else:
            other_blocks.append(child)
            body_members.append(_block_body_member(child))

    process = ProcessDecl(
        kind=match.group(1),
        name=match.group(2),
        span=segment.span,
        parameters=_parse_process_parameters(match.group(3) or "", segment.start_line),
        return_type=match.group(4),
        depends_on=depends_on,
        drives=drives,
        within=within,
        may_change=may_change,
        ensures=ensures,
        result=result,
        other_blocks=other_blocks,
        body_members=body_members,
        properties=properties,
    )
    return _number_process_statements(process, f"{owner}.{process.kind}::{process.name}")


def _parse_process_parameters(raw: str, line: int) -> tuple[tuple[str, str], ...]:
    raw = raw.strip()
    if not raw:
        return ()
    parameters: list[tuple[str, str]] = []
    for item in raw.split(","):
        name, sep, type_name = item.strip().partition(":")
        if not sep or not re.fullmatch(r"[a-z][A-Za-z0-9_]*", name.strip()):
            raise ParseError(f"line {line}: invalid process parameter: {_preview(item)}")
        if not re.fullmatch(_IDENT, type_name.strip()):
            raise ParseError(f"line {line}: invalid process parameter type: {_preview(item)}")
        parameters.append((name.strip(), type_name.strip()))
    return tuple(parameters)


def _number_process_statements(process: ProcessDecl, owner: str) -> ProcessDecl:
    ordinal = 0

    def number_block(block: Block) -> Block:
        nonlocal ordinal
        if block.kind != "drives":
            return block
        statements: list[DriveStatement] = []
        for statement in block.statements:
            ordinal += 1
            statements.append(
                dc_replace(statement, ordinal=ordinal, owner_process=owner)
            )
        return dc_replace(block, statements=statements)

    def number_within(within: WithinDecl) -> WithinDecl:
        members: list[BodyMember] = []
        for member in within.body_members:
            if member.block is not None:
                block = number_block(member.block)
                members.append(dc_replace(member, block=block))
            elif member.within is not None:
                nested = number_within(member.within)
                members.append(dc_replace(member, within=nested))
            else:
                members.append(member)
        return dc_replace(
            within,
            drives=[m.block for m in members if m.kind == "drives" and m.block is not None],
            within=[m.within for m in members if m.within is not None],
            body_members=members,
        )

    members: list[BodyMember] = []
    for member in process.body_members:
        if member.block is not None:
            block = number_block(member.block)
            members.append(dc_replace(member, block=block))
        elif member.within is not None:
            nested = number_within(member.within)
            members.append(dc_replace(member, within=nested))
        else:
            members.append(member)
    return dc_replace(
        process,
        depends_on=[m.block for m in members if m.kind == "depends_on" and m.block is not None],
        drives=[m.block for m in members if m.kind == "drives" and m.block is not None],
        within=[m.within for m in members if m.within is not None],
        may_change=[m.block for m in members if m.kind == "may_change" and m.block is not None],
        ensures=[m.block for m in members if m.kind == "ensures" and m.block is not None],
        result=[m.block for m in members if m.kind == "result" and m.block is not None],
        body_members=members,
    )


def _number_transition_statements(
    transition: TransitionDecl, owner: str
) -> TransitionDecl:
    ordinal = 0

    def number_block(block: Block) -> Block:
        nonlocal ordinal
        if block.kind != "drives":
            return block
        statements: list[DriveStatement] = []
        for statement in block.statements:
            ordinal += 1
            statements.append(dc_replace(statement, ordinal=ordinal, owner_process=owner))
        return dc_replace(block, statements=statements)

    def number_within(within: WithinDecl) -> WithinDecl:
        members: list[BodyMember] = []
        for member in within.body_members:
            if member.block is not None:
                members.append(dc_replace(member, block=number_block(member.block)))
            elif member.within is not None:
                members.append(dc_replace(member, within=number_within(member.within)))
            else:
                members.append(member)
        return dc_replace(
            within,
            drives=[m.block for m in members if m.kind == "drives" and m.block is not None],
            within=[m.within for m in members if m.within is not None],
            body_members=members,
        )

    members: list[BodyMember] = []
    for member in transition.body_members:
        if member.block is not None:
            members.append(dc_replace(member, block=number_block(member.block)))
        elif member.within is not None:
            members.append(dc_replace(member, within=number_within(member.within)))
        else:
            members.append(member)
    return dc_replace(
        transition,
        drives=[m.block for m in members if m.kind == "drives" and m.block is not None],
        within=[m.within for m in members if m.within is not None],
        body_members=members,
    )




def _parse_within(block: Block) -> WithinDecl:
    context_header = block.header.strip()
    context, parameters, only_once = _parse_within_header(context_header)
    if not context:
        raise ParseError(f"line {block.span.start_line}: within block is missing context")

    entered_by: list[Block] = []
    depends_on: list[Block] = []
    drives: list[Block] = []
    within: list[WithinDecl] = []
    exited_by: list[Block] = []
    may_change: list[Block] = []
    ensures: list[Block] = []
    boundaries: list[BoundaryDecl] = []
    deferred: list[Block] = []
    other_blocks: list[Block] = []
    body_members: list[BodyMember] = []

    for part in _split_members(block.body, block.body_start_line or block.span.start_line):
        block_match = _BLOCK_RE.match(part.text.strip())
        if not block_match:
            raise ParseError(
                f"line {part.start_line}: invalid within member: {_preview(part.text)}"
            )
        child = _to_block(part, block_match.group(1))
        if child.kind == "entered_by":
            entered_by.append(child)
            body_members.append(_block_body_member(child))
        elif child.kind == "depends_on":
            depends_on.append(child)
            body_members.append(_block_body_member(child))
        elif child.kind == "drives":
            drives.append(child)
            body_members.append(_block_body_member(child))
        elif child.kind == "within":
            nested = _parse_within(child)
            within.append(nested)
            body_members.append(_within_body_member(nested))
        elif child.kind == "exited_by":
            exited_by.append(child)
            body_members.append(_block_body_member(child))
        elif child.kind == "may_change":
            may_change.append(child)
            body_members.append(_block_body_member(child))
        elif child.kind == "ensures":
            ensures.append(child)
            body_members.append(_block_body_member(child))
        elif child.kind == "deferred":
            if child.header.strip():
                boundary = _parse_boundary(child)
                boundaries.append(boundary)
                body_members.append(_boundary_body_member(boundary))
            else:
                deferred.append(child)
                body_members.append(_block_body_member(child))
        elif child.kind == "trimmed":
            boundary = _parse_boundary(child)
            boundaries.append(boundary)
            body_members.append(_boundary_body_member(boundary))
        else:
            other_blocks.append(child)
            body_members.append(_block_body_member(child))

    return WithinDecl(
        context=context,
        span=block.span,
        only_once=only_once,
        parameters=parameters,
        entered_by=entered_by,
        depends_on=depends_on,
        drives=drives,
        within=within,
        exited_by=exited_by,
        may_change=may_change,
        ensures=ensures,
        boundaries=boundaries,
        deferred=deferred,
        other_blocks=other_blocks,
        body_members=body_members,
    )


def _parse_within_header(header: str) -> tuple[str, dict[str, str], bool]:
    only_once = False
    marker = " only-once"
    if header.endswith(marker):
        only_once = True
        header = header[: -len(marker)].rstrip()
    if "(" not in header:
        return header, {}, only_once
    if not header.endswith(")"):
        raise ParseError(f"invalid within header: {_preview(header)}")
    context, args = header.split("(", 1)
    context = context.strip()
    args = args[:-1].strip()
    parameters: dict[str, str] = {}
    if not args:
        return context, parameters, only_once
    for raw_arg in args.split(","):
        arg = raw_arg.strip()
        if not arg:
            continue
        if ":" not in arg:
            raise ParseError(f"invalid within parameter: {_preview(arg)}")
        name, value = arg.split(":", 1)
        name = name.strip()
        value = value.strip()
        if not name or not value:
            raise ParseError(f"invalid within parameter: {_preview(arg)}")
        parameters[name] = value
    return context, parameters, only_once


def _block_body_member(block: Block) -> BodyMember:
    return BodyMember(kind=block.kind, span=block.span, block=block)


def _within_body_member(within: WithinDecl) -> BodyMember:
    return BodyMember(kind="within", span=within.span, within=within)


def _boundary_body_member(boundary: BoundaryDecl) -> BodyMember:
    return BodyMember(
        kind=boundary.status,
        span=boundary.span,
        boundary=boundary,
    )


def _parse_boundary(block: Block) -> BoundaryDecl:
    properties: dict[str, str] = {}
    property_counts: dict[str, int] = {}
    evidence: list[Block] = []
    other_blocks: list[Block] = []

    for part in _split_members(
        block.body, block.body_start_line or block.span.start_line
    ):
        stripped = part.text.strip()
        block_match = _BLOCK_RE.match(stripped)
        if block_match:
            child = _to_block(part, block_match.group(1))
            if child.kind == "evidence":
                evidence.append(child)
            else:
                other_blocks.append(child)
            continue

        prop_match = _PROP_RE.match(stripped)
        if prop_match is None:
            raise ParseError(
                f"line {part.start_line}: invalid {block.kind} boundary member: "
                f"{_preview(part.text)}"
            )
        key = prop_match.group(1)
        value = prop_match.group(2).strip()
        property_counts[key] = property_counts.get(key, 0) + 1
        properties[key] = value

    resolution_key = "close_when" if block.kind == "deferred" else "revisit_when"
    known = {"category", "summary", resolution_key}
    return BoundaryDecl(
        status=block.kind,
        id=block.header.strip(),
        span=block.span,
        category=properties.get("category"),
        summary=_boundary_string_value(properties.get("summary"), block, "summary"),
        evidence=evidence,
        resolution=_boundary_string_value(
            properties.get(resolution_key), block, resolution_key
        ),
        property_counts=property_counts,
        other_blocks=other_blocks,
        unknown_properties={
            key: value for key, value in properties.items() if key not in known
        },
    )


def _boundary_string_value(
    raw: str | None, block: Block, field_name: str
) -> str | None:
    if raw is None:
        return None
    try:
        value = ast.literal_eval(raw)
    except (SyntaxError, ValueError):
        raise ParseError(
            f"line {block.span.start_line}: {block.kind} {field_name} must be a string"
        ) from None
    if not isinstance(value, str):
        raise ParseError(
            f"line {block.span.start_line}: {block.kind} {field_name} must be a string"
        )
    return value


def _parse_named_blocks(body: str, start_line: int) -> list[Block]:
    blocks: list[Block] = []
    for part in _split_members(body, start_line):
        block_match = _BLOCK_RE.match(part.text.strip())
        if block_match:
            blocks.append(_to_block(part, block_match.group(1)))
    return blocks


def _to_block(segment: _Segment, kind: str) -> Block:
    text = segment.text
    match = _BLOCK_RE.match(text)
    if not match:
        raise ParseError(f"line {segment.start_line}: invalid block")
    body, body_start_line = _body_segment_from_braced_decl(
        text, match.end() - 1, segment.start_line
    )
    block = Block(
        kind=kind,
        body=body,
        span=segment.span,
        header=match.group("header").strip(),
        body_start_line=body_start_line,
    )
    if kind == "drives":
        if body.strip() and not body.rstrip().endswith(";"):
            raise ParseError(
                f"line {segment.start_line}: drives statement is missing semicolon"
            )
        statements: list[DriveStatement] = []
        for entry, entry_span in block.entry_spans:
            declare = _DECLARE_RE.match(entry)
            if declare is not None:
                statements.append(
                    DriveStatement(
                        kind="declare",
                        text=entry,
                        span=entry_span,
                        alias=declare.group(1),
                        declared_type=declare.group(2),
                    )
                )
                continue
            if entry.lstrip().startswith("declare"):
                raise ParseError(
                    f"line {entry_span.start_line}: malformed declare statement: "
                    f"{_preview(entry)}"
                )
            statement_kind = "let" if entry.lstrip().startswith("let ") else "call"
            statements.append(
                DriveStatement(kind=statement_kind, text=entry, span=entry_span)
            )
        block = dc_replace(block, statements=statements)
    else:
        for entry, entry_span in block.entry_spans:
            if entry.lstrip().startswith("declare"):
                raise ParseError(
                    f"line {entry_span.start_line}: declare is only allowed in drives"
                )
    return block


def _top_level_segments(text: str) -> list[_Segment]:
    return _split_members(text, 1)


def _split_members(text: str, start_line: int) -> list[_Segment]:
    segments: list[_Segment] = []
    index = 0
    length = len(text)

    while index < length:
        index = _skip_ws(text, index)
        if index >= length:
            break

        member_start = index
        member_line = start_line + text.count("\n", 0, member_start)
        brace = _find_next_delimiter(text, member_start)
        if brace is None:
            raise ParseError(f"line {member_line}: unterminated declaration")

        delimiter = text[brace]
        if delimiter == ";":
            member_end = brace + 1
        elif delimiter == "{":
            member_end = _matching_brace_end(text, brace, member_line)
        else:
            raise ParseError(f"line {member_line}: unexpected delimiter {delimiter!r}")

        raw = text[member_start:member_end]
        if raw:
            end_line = start_line + text.count("\n", 0, member_end)
            segments.append(_Segment(raw, member_line, end_line))
        index = member_end

    return segments


def _skip_ws(text: str, index: int) -> int:
    length = len(text)
    while index < length and text[index].isspace():
        index += 1
    return index


def _find_next_delimiter(text: str, start: int) -> int | None:
    index = start
    length = len(text)
    in_string = False
    angle_depth = 0
    paren_depth = 0
    bracket_depth = 0

    while index < length:
        char = text[index]
        if in_string:
            if char == "\\":
                index += 2
                continue
            if char == '"':
                in_string = False
            index += 1
            continue

        if char == '"':
            in_string = True
        elif char == "<":
            angle_depth += 1
        elif char == ">":
            angle_depth = max(angle_depth - 1, 0)
        elif char == "(":
            paren_depth += 1
        elif char == ")":
            paren_depth = max(paren_depth - 1, 0)
        elif char == "[":
            bracket_depth += 1
        elif char == "]":
            bracket_depth = max(bracket_depth - 1, 0)
        elif char in "{;" and angle_depth == 0 and paren_depth == 0 and bracket_depth == 0:
            return index
        index += 1
    return None


def _matching_brace_end(text: str, open_index: int, line: int) -> int:
    depth = 0
    index = open_index
    length = len(text)
    in_string = False

    while index < length:
        char = text[index]
        if in_string:
            if char == "\\":
                index += 2
                continue
            if char == '"':
                in_string = False
            index += 1
            continue

        if char == '"':
            in_string = True
        elif char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return index + 1
        index += 1

    raise ParseError(f"line {line}: unterminated block")


def _body_from_braced_decl(text: str, open_index: int, line: int) -> str:
    body, _body_start_line = _body_segment_from_braced_decl(text, open_index, line)
    return body.strip()


def _body_segment_from_braced_decl(
    text: str, open_index: int, line: int
) -> tuple[str, int]:
    end = _matching_brace_end(text, open_index, line)
    body_start = open_index + 1
    body = text[body_start : end - 1]
    body_start_line = line + text.count("\n", 0, body_start)
    return body, body_start_line


def _find_top_level_char(text: str, target: str) -> int | None:
    depth = 0
    for index, char in enumerate(text):
        if char in "([{<":
            if char == target and depth == 0:
                return index
            depth += 1
        elif char in ")]}>":
            depth = max(depth - 1, 0)
        elif char == target and depth == 0:
            return index
    return None


def _state_name(value: str) -> str:
    value = value.rstrip(";").strip()
    prefix = "State::"
    return value[len(prefix) :] if value.startswith(prefix) else value


def _line_count(text: str) -> int:
    return text.count("\n")


def _preview(text: str) -> str:
    return " ".join(text.strip().split())[:80]


def summarize(document: SpecDocument) -> str:
    """Return a small human-readable parse summary."""

    state_count = sum(len(obj.states) for obj in document.objects)
    transition_count = sum(len(state.transitions) for obj in document.objects for state in obj.states)
    lines = [
        "parse: ok",
        f"enums: {len(document.enums)}",
        f"functions: {len(document.functions)}",
        f"predicates: {len(document.predicates)}",
        f"types: {len(document.types)}",
        f"locks: {len(document.locks)}",
        f"exclusive_contexts: {len(document.exclusive_contexts)}",
        f"objects: {len(document.objects)}",
        f"states: {state_count}",
        f"transitions: {transition_count}",
    ]
    return "\n".join(lines)


__all__ = [
    "ParseError",
    "parse_file",
    "parse_text",
    "strip_comments",
    "summarize",
    "statement_entries",
]
