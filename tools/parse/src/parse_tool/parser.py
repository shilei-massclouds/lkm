"""Parser for pyveri object model specs."""

from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path

from common.spec_ast import (
    Block,
    EnumDecl,
    EventDecl,
    ExclusiveContextDecl,
    FunctionDecl,
    LockDecl,
    ObjectDecl,
    PredicateDecl,
    SourceSpan,
    SpecDocument,
    StateDecl,
    TypeDecl,
    WithinDecl,
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
_OBJECT_RE = re.compile(rf"\Aobject\s+({_IDENT})\s*:\s*({_IDENT})\s*\{{", re.S)
_FUNCTION_RE = re.compile(rf"\Afunction\s+({_IDENT})(?P<sig>.*);?\Z", re.S)
_PREDICATE_RE = re.compile(rf"\Apredicate\s+({_IDENT})(?P<rest>.*)\Z", re.S)
_STATE_RE = re.compile(rf"\Astate\s+State::({_IDENT})\s*\{{", re.S)
_EVENT_RE = re.compile(rf"\Aon\s+Event::({_IDENT})\s*->\s*State::({_IDENT})\s*\{{", re.S)
_BLOCK_RE = re.compile(rf"\A({_IDENT})(?P<header>[^\{{]*)\{{", re.S)
_PROP_RE = re.compile(rf"\A({_IDENT})\s*:\s*(.+?)\s*;\Z", re.S)
_INCLUDE_LINE_RE = re.compile(r'\A\s*include\s+"([^"]+)"\s*;\s*\Z')


def parse_file(path: str | Path) -> SpecDocument:
    """Parse a spec file."""

    text = _read_with_includes(Path(path), seen=set(), stack=[])
    return parse_text(text)


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


def _read_with_includes(path: Path, seen: set[Path], stack: list[Path]) -> str:
    return _read_with_include_mode(path, seen, stack, strip=True)


def _read_source_with_includes(path: Path, seen: set[Path], stack: list[Path]) -> str:
    return _read_with_include_mode(path, seen, stack, strip=False)


def _read_with_include_mode(
    path: Path, seen: set[Path], stack: list[Path], *, strip: bool
) -> str:
    resolved = path.resolve()
    if resolved in stack:
        cycle = " -> ".join(str(item) for item in [*stack, resolved])
        raise ParseError(f"include cycle: {cycle}")
    if resolved in seen:
        return "\n"

    seen.add(resolved)
    raw = resolved.read_text(encoding="utf-8")
    text = strip_comments(raw) if strip else raw
    return _expand_includes(text, resolved.parent, seen, [*stack, resolved], strip=strip)


def _expand_includes(
    text: str, base_dir: Path, seen: set[Path], stack: list[Path], *, strip: bool
) -> str:
    lines: list[str] = []
    for line in text.splitlines(keepends=True):
        match = _INCLUDE_LINE_RE.match(line.rstrip("\n"))
        if not match:
            lines.append(line)
            continue
        include_path = (base_dir / match.group(1)).resolve()
        lines.append(_read_with_include_mode(include_path, seen, stack, strip=strip))
        if lines[-1] and not lines[-1].endswith("\n"):
            lines.append("\n")
    return "".join(lines)


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
    properties: dict[str, str] = {}
    for part in _split_members(body, body_start_line):
        stripped = part.text.strip()
        block_match = _BLOCK_RE.match(stripped)
        if block_match:
            blocks.append(_to_block(part, block_match.group(1)))
            continue
        prop_match = _PROP_RE.match(stripped)
        if not prop_match:
            raise ParseError(
                f"line {part.start_line}: invalid type member: {_preview(part.text)}"
            )
        properties[prop_match.group(1)] = prop_match.group(2).strip()
    return TypeDecl(
        name=match.group(1),
        header=match.group("header").strip(),
        span=segment.span,
        blocks=blocks,
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

    body, body_start_line = _body_segment_from_braced_decl(
        segment.text, match.end() - 1, segment.start_line
    )
    parts = _split_members(body, body_start_line)
    lock_ref: str | None = None
    obj_refs: list[str] = []
    other_blocks: list[Block] = []
    properties: dict[str, str] = {}

    for part in parts:
        stripped = part.text.strip()
        block_match = _BLOCK_RE.match(stripped)
        if block_match:
            block = _to_block(part, block_match.group(1))
            if block.kind == "obj_refs":
                obj_refs.extend(block.entries)
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
        lock_ref=lock_ref,
        obj_refs=obj_refs,
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
    other_blocks: list[Block] = []
    properties: dict[str, str] = {}

    for part in parts:
        stripped = part.text.strip()
        state_match = _STATE_RE.match(stripped)
        block_match = _BLOCK_RE.match(stripped)

        if state_match:
            states.append(_parse_state(part))
            continue

        if block_match:
            block = _to_block(part, block_match.group(1))
            if block.kind == "attrs":
                attrs.append(block)
            elif block.kind == "reference":
                references.append(block)
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
        other_blocks=other_blocks,
        properties=properties,
    )


def _parse_state(segment: _Segment) -> StateDecl:
    match = _STATE_RE.match(segment.text)
    if not match:
        raise ParseError(f"line {segment.start_line}: invalid state declaration")

    body, body_start_line = _body_segment_from_braced_decl(
        segment.text, match.end() - 1, segment.start_line
    )
    parts = _split_members(body, body_start_line)

    invariants: list[Block] = []
    deferred: list[Block] = []
    events: list[EventDecl] = []
    other_blocks: list[Block] = []

    for part in parts:
        stripped = part.text.strip()
        event_match = _EVENT_RE.match(stripped)
        block_match = _BLOCK_RE.match(stripped)

        if event_match:
            events.append(_parse_event(part))
            continue

        if not block_match:
            raise ParseError(
                f"line {part.start_line}: invalid state member: {_preview(part.text)}"
            )

        block = _to_block(part, block_match.group(1))
        if block.kind == "invariant":
            invariants.append(block)
        elif block.kind == "deferred":
            deferred.append(block)
        elif block.kind == "events":
            events.extend(_parse_events_block(block))
        else:
            other_blocks.append(block)

    return StateDecl(match.group(1), segment.span, invariants, deferred, events, other_blocks)


def _parse_events_block(block: Block) -> list[EventDecl]:
    parts = _split_members(block.body, block.body_start_line or block.span.start_line)
    events: list[EventDecl] = []
    for part in parts:
        if not _EVENT_RE.match(part.text.strip()):
            raise ParseError(
                f"line {part.start_line}: invalid events member: {_preview(part.text)}"
            )
        events.append(_parse_event(part))
    return events


def _parse_event(segment: _Segment) -> EventDecl:
    match = _EVENT_RE.match(segment.text)
    if not match:
        raise ParseError(f"line {segment.start_line}: invalid event declaration")

    body, body_start_line = _body_segment_from_braced_decl(
        segment.text, match.end() - 1, segment.start_line
    )
    parts = _split_members(body, body_start_line)

    depends_on: list[Block] = []
    drives: list[Block] = []
    within: list[WithinDecl] = []
    may_change: list[Block] = []
    ensures: list[Block] = []
    deferred: list[Block] = []
    other_blocks: list[Block] = []

    for part in parts:
        block_match = _BLOCK_RE.match(part.text.strip())
        if not block_match:
            raise ParseError(
                f"line {part.start_line}: invalid event member: {_preview(part.text)}"
            )
        block = _to_block(part, block_match.group(1))
        if block.kind == "depends_on":
            depends_on.append(block)
        elif block.kind == "drives":
            drives.append(block)
        elif block.kind == "within":
            within.append(_parse_within(block))
        elif block.kind == "may_change":
            may_change.append(block)
        elif block.kind == "ensures":
            ensures.append(block)
        elif block.kind == "deferred":
            deferred.append(block)
        else:
            other_blocks.append(block)

    return EventDecl(
        name=match.group(1),
        target_state=match.group(2),
        span=segment.span,
        depends_on=depends_on,
        drives=drives,
        within=within,
        may_change=may_change,
        ensures=ensures,
        deferred=deferred,
        other_blocks=other_blocks,
    )




def _parse_within(block: Block) -> WithinDecl:
    context = block.header.strip()
    if not context:
        raise ParseError(f"line {block.span.start_line}: within block is missing context")

    entered_by: list[Block] = []
    depends_on: list[Block] = []
    drives: list[Block] = []
    exited_by: list[Block] = []
    may_change: list[Block] = []
    ensures: list[Block] = []
    deferred: list[Block] = []
    other_blocks: list[Block] = []

    for part in _split_members(block.body, block.body_start_line or block.span.start_line):
        block_match = _BLOCK_RE.match(part.text.strip())
        if not block_match:
            raise ParseError(
                f"line {part.start_line}: invalid within member: {_preview(part.text)}"
            )
        child = _to_block(part, block_match.group(1))
        if child.kind == "entered_by":
            entered_by.append(child)
        elif child.kind == "depends_on":
            depends_on.append(child)
        elif child.kind == "drives":
            drives.append(child)
        elif child.kind == "exited_by":
            exited_by.append(child)
        elif child.kind == "may_change":
            may_change.append(child)
        elif child.kind == "ensures":
            ensures.append(child)
        elif child.kind == "deferred":
            deferred.append(child)
        else:
            other_blocks.append(child)

    return WithinDecl(
        context=context,
        span=block.span,
        entered_by=entered_by,
        depends_on=depends_on,
        drives=drives,
        exited_by=exited_by,
        may_change=may_change,
        ensures=ensures,
        deferred=deferred,
        other_blocks=other_blocks,
    )


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
    return Block(
        kind=kind,
        body=body,
        span=segment.span,
        header=match.group("header").strip(),
        body_start_line=body_start_line,
    )


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
    event_count = sum(len(state.events) for obj in document.objects for state in obj.states)
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
        f"events: {event_count}",
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
