"""Source-preserving parser for the complete model syntax consumed by tools2.

The parser deliberately keeps expressions and executable statements as text.
The model stage owns their semantic normalization.  Structural syntax is
strict: an unknown declaration or block member becomes a spanned diagnostic.
"""

from __future__ import annotations

from dataclasses import dataclass
import json
from pathlib import Path
import re
from typing import Any

from tools2_common import stable_source_path


_IDENT = r"[A-Za-z_][A-Za-z0-9_-]*"
_TOP_RE = re.compile(rf"^\s*({_IDENT})")
_BLOCK_RE = re.compile(rf"^\s*({_IDENT})(?P<header>[^{{;]*)\{{", re.S)
_PROP_RE = re.compile(rf"^\s*({_IDENT})\s*:\s*(.*?)\s*;\s*$", re.S)
_STATE_RE = re.compile(rf"^\s*state\s+State::({_IDENT})\s*\{{", re.S)
_PROCESS_RE = re.compile(
    rf"^\s*(?:on\s+)?(?P<kind>Transition|Action)::(?P<name>{_IDENT})"
    rf"\s*(?:<(?P<type_parameters>[^{{;]*)>)?\s*(?:\((?P<parameters>.*?)\))?"
    rf"\s*(?:->\s*(?:State::)?(?P<target>{_IDENT}))?\s*\{{",
    re.S,
)


@dataclass(frozen=True)
class Segment:
    text: str
    path: str
    start_line: int
    end_line: int

    def span(self) -> dict[str, Any]:
        last = self.text.splitlines()[-1] if self.text.splitlines() else ""
        return {
            "source_file": self.path,
            "start_line": self.start_line,
            "start_column": 1,
            "end_line": self.end_line,
            "end_column": len(last) + 1,
        }


class StructuralFailure(ValueError):
    def __init__(self, message: str, segment: Segment):
        super().__init__(message)
        self.message = message
        self.segment = segment


def _strip_comments(text: str) -> str:
    result: list[str] = []
    index = 0
    in_string = False
    while index < len(text):
        char = text[index]
        nxt = text[index + 1] if index + 1 < len(text) else ""
        if in_string:
            result.append(char)
            if char == "\\" and index + 1 < len(text):
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
            while index < len(text) and text[index] != "\n":
                result.append(" ")
                index += 1
            continue
        if char == "/" and nxt == "*":
            result.extend("  ")
            index += 2
            while index < len(text):
                if text[index : index + 2] == "*/":
                    result.extend("  ")
                    index += 2
                    break
                result.append("\n" if text[index] == "\n" else " ")
                index += 1
            else:
                raise ValueError("unterminated block comment")
            continue
        result.append(char)
        index += 1
    return "".join(result)


def _normalize_description(lines: list[str]) -> str | None:
    normalized: list[str] = []
    for line in lines:
        value = line.strip()
        if value.startswith("*"):
            value = value[1:].lstrip()
        normalized.append(value.rstrip())
    while normalized and not normalized[0]:
        normalized.pop(0)
    while normalized and not normalized[-1]:
        normalized.pop()
    description = "\n".join(normalized).strip()
    return description or None


def _leading_description(text: str, start_line: int) -> str | None:
    """Return a declaration-adjacent Model comment as display-only metadata."""

    lines = text.splitlines()
    index = start_line - 2
    if index < 0:
        return None
    stripped = lines[index].strip()
    if stripped.startswith("//"):
        first = index
        while first >= 0 and lines[first].strip().startswith("//"):
            first -= 1
        return _normalize_description(
            [line.strip()[2:].lstrip() for line in lines[first + 1 : index + 1]]
        )
    if not stripped.endswith("*/"):
        return None
    first = index
    while first >= 0 and "/*" not in lines[first]:
        first -= 1
    if first < 0:
        return None
    raw = "\n".join(lines[first : index + 1])
    if re.fullmatch(r"\s*/\*.*?\*/\s*", raw, re.S) is None:
        return None
    body = raw[raw.find("/*") + 2 : raw.rfind("*/")]
    return _normalize_description(body.splitlines())


def _attach_handler_descriptions(value: Any, source_text: str) -> None:
    if isinstance(value, list):
        for item in value:
            _attach_handler_descriptions(item, source_text)
        return
    if not isinstance(value, dict):
        return
    span = value.get("span")
    if (
        value.get("kind") in {"Transition", "Action"}
        and isinstance(value.get("name"), str)
        and isinstance(span, dict)
        and isinstance(span.get("start_line"), int)
    ):
        description = _leading_description(source_text, span["start_line"])
        if description is not None:
            value["description"] = description
    for child in value.values():
        _attach_handler_descriptions(child, source_text)


def _matching_brace(text: str, opening: int, segment: Segment) -> int:
    depth = 0
    in_string = False
    index = opening
    while index < len(text):
        char = text[index]
        if in_string:
            if char == "\\":
                index += 2
                continue
            if char == '"':
                in_string = False
        elif char == '"':
            in_string = True
        elif char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return index + 1
        index += 1
    raise StructuralFailure("unterminated block", segment)


def _next_delimiter(text: str, start: int) -> int | None:
    paren = 0
    angle = 0
    in_string = False
    index = start
    while index < len(text):
        char = text[index]
        if in_string:
            if char == "\\":
                index += 2
                continue
            if char == '"':
                in_string = False
        elif char == '"':
            in_string = True
        elif char == "(":
            paren += 1
        elif char == ")":
            paren = max(0, paren - 1)
        elif char == "<" and index + 1 < len(text) and text[index + 1] not in "-=":
            angle += 1
        elif char == ">" and angle:
            angle -= 1
        elif char in "{;" and paren == 0 and angle == 0:
            return index
        index += 1
    return None


def _split_members(text: str, *, path: str, start_line: int) -> list[Segment]:
    members: list[Segment] = []
    index = 0
    while index < len(text):
        while index < len(text) and text[index].isspace():
            index += 1
        if index >= len(text):
            break
        opening = _next_delimiter(text, index)
        line = start_line + text.count("\n", 0, index)
        placeholder = Segment(text[index:], path, line, line)
        if opening is None:
            raise StructuralFailure("unterminated declaration", placeholder)
        if text[opening] == ";":
            end = opening + 1
        else:
            end = _matching_brace(text, opening, placeholder)
        raw = text[index:end]
        end_line = start_line + text.count("\n", 0, end)
        members.append(Segment(raw, path, line, end_line))
        index = end
    return members


def _block_body(segment: Segment) -> tuple[str, int, str]:
    match = _BLOCK_RE.match(segment.text)
    if match is None:
        raise StructuralFailure("expected a named block", segment)
    opening = segment.text.find("{", match.start())
    end = _matching_brace(segment.text, opening, segment)
    body_line = segment.start_line + segment.text.count("\n", 0, opening + 1)
    return segment.text[opening + 1 : end - 1], body_line, match.group("header").strip()


def _split_commas(text: str) -> list[str]:
    parts: list[str] = []
    depth = 0
    start = 0
    in_string = False
    for index, char in enumerate(text):
        if in_string:
            if char == '"' and (index == 0 or text[index - 1] != "\\"):
                in_string = False
            continue
        if char == '"':
            in_string = True
        elif char in "(<{[":
            depth += 1
        elif char in ")>}]":
            depth = max(0, depth - 1)
        elif char == "," and depth == 0:
            parts.append(text[start:index].strip())
            start = index + 1
    tail = text[start:].strip()
    if tail:
        parts.append(tail)
    return parts


def _parameters(raw: str | None, segment: Segment) -> list[dict[str, Any]]:
    if raw is None or not raw.strip():
        return []
    result: list[dict[str, Any]] = []
    for part in _split_commas(raw):
        name, separator, type_name = part.partition(":")
        if not separator or re.fullmatch(_IDENT, name.strip()) is None or not type_name.strip():
            raise StructuralFailure(f"invalid process parameter {part!r}", segment)
        result.append({"name": name.strip(), "type": type_name.strip(), "span": segment.span()})
    return result


def _type_parameters(raw: str | None, segment: Segment) -> list[dict[str, Any]]:
    if raw is None or not raw.strip():
        return []
    result: list[dict[str, Any]] = []
    for part in _split_commas(raw):
        name, separator, bound = part.partition(":")
        if re.fullmatch(_IDENT, name.strip()) is None:
            raise StructuralFailure(f"invalid process type parameter {part!r}", segment)
        if separator and not bound.strip():
            raise StructuralFailure(f"missing process type parameter bound {part!r}", segment)
        result.append(
            {
                "name": name.strip(),
                "bound": bound.strip() if separator else "System",
                "span": segment.span(),
            }
        )
    return result


def _entries(body: str, *, path: str, start_line: int) -> list[dict[str, Any]]:
    result: list[dict[str, Any]] = []
    start = 0
    paren = 0
    brace = 0
    in_string = False
    for index, char in enumerate(body):
        if in_string:
            if char == '"' and (index == 0 or body[index - 1] != "\\"):
                in_string = False
            continue
        if char == '"':
            in_string = True
        elif char == "(":
            paren += 1
        elif char == ")":
            paren = max(0, paren - 1)
        elif char == "{":
            brace += 1
        elif char == "}":
            brace = max(0, brace - 1)
        elif char == ";" and paren == 0 and brace == 0:
            raw = body[start:index].strip()
            if raw:
                line = start_line + body.count("\n", 0, start)
                result.append(
                    {
                        "text": " ".join(raw.split()),
                        "span": {
                            "source_file": path,
                            "start_line": line,
                            "start_column": 1,
                            "end_line": start_line + body.count("\n", 0, index),
                            "end_column": 1,
                        },
                    }
                )
            start = index + 1
    if body[start:].strip():
        tail = Segment(body[start:], path, start_line + body.count("\n", 0, start), start_line)
        raise StructuralFailure("statement requires ';'", tail)
    return result


def _plain_block(segment: Segment, kind: str | None = None) -> dict[str, Any]:
    body, body_line, header = _block_body(segment)
    return {
        "kind": kind or (_TOP_RE.match(segment.text).group(1)),
        "header": header,
        "entries": _entries(body, path=segment.path, start_line=body_line),
        "span": segment.span(),
    }


def _boundary(segment: Segment, status: str, diagnostics: list[dict[str, Any]]) -> dict[str, Any]:
    body, body_line, header = _block_body(segment)
    evidence: list[dict[str, Any]] = []
    properties: dict[str, str] = {}
    evidence_blocks = 0
    for member in _split_members(body, path=segment.path, start_line=body_line):
        block = _BLOCK_RE.match(member.text)
        prop = _PROP_RE.match(member.text)
        if block and block.group(1) == "evidence":
            evidence_blocks += 1
            if evidence_blocks > 1:
                diagnostics.append(
                    _diagnostic("error", f"duplicate {status} evidence block", member)
                )
            evidence.extend(_plain_block(member)["entries"])
        elif prop:
            name = prop.group(1)
            if name in properties:
                diagnostics.append(
                    _diagnostic("error", f"duplicate {status} property {name}", member)
                )
            else:
                properties[name] = prop.group(2).strip()
        else:
            diagnostics.append(_diagnostic("unsupported", f"unsupported {status} member", member))
    return {
        "kind": status,
        "id": header,
        "properties": properties,
        "evidence": evidence,
        "span": segment.span(),
    }


_HANDLER_BLOCKS = frozenset(
    {
        "depends_on",
        "drives",
        "emits",
        "yields",
        "ensures",
        "updates",
        "may_change",
        "result",
        "transitions",
    }
)


def _within(segment: Segment, diagnostics: list[dict[str, Any]]) -> dict[str, Any]:
    body, body_line, header = _block_body(segment)
    only_once = header.endswith(" only-once")
    if only_once:
        header = header[: -len(" only-once")].rstrip()
    members = _handler_members(body, segment.path, body_line, diagnostics)
    return {
        "kind": "within",
        "context": header,
        "only_once": only_once,
        "members": members,
        "span": segment.span(),
    }


def _result_block(segment: Segment, diagnostics: list[dict[str, Any]]) -> dict[str, Any]:
    body, body_line, header = _block_body(segment)
    variants: list[dict[str, Any]] = []
    try:
        for member in _split_members(body, path=segment.path, start_line=body_line):
            match = _BLOCK_RE.match(member.text)
            if match is None:
                variants.append({"kind": "entry", **_plain_block(segment)})
                break
            child_body, child_line, child_header = _block_body(member)
            variants.append(
                {
                    "name": match.group(1),
                    "header": child_header,
                    "members": _handler_members(child_body, member.path, child_line, diagnostics),
                    "span": member.span(),
                }
            )
    except StructuralFailure:
        variants = []
    return {"kind": "result", "header": header, "variants": variants, "span": segment.span()}


def _handler_members(
    body: str, path: str, start_line: int, diagnostics: list[dict[str, Any]]
) -> list[dict[str, Any]]:
    result: list[dict[str, Any]] = []
    for member in _split_members(body, path=path, start_line=start_line):
        block = _BLOCK_RE.match(member.text)
        prop = _PROP_RE.match(member.text)
        if block:
            kind = block.group(1)
            if kind in _HANDLER_BLOCKS:
                result.append(
                    _result_block(member, diagnostics) if kind == "result" else _plain_block(member, kind)
                )
            elif kind == "within":
                result.append(_within(member, diagnostics))
            elif kind in {"deferred", "trimmed"}:
                result.append(_boundary(member, kind, diagnostics))
            elif kind in {"actions", "processes"}:
                result.append(
                    {
                        "kind": kind,
                        "handlers": _process_container(member, diagnostics, source_state=None),
                        "span": member.span(),
                    }
                )
            else:
                diagnostics.append(_diagnostic("unsupported", f"unsupported handler member {kind!r}", member))
        elif prop:
            result.append(
                {"kind": "property", "name": prop.group(1), "value": prop.group(2).strip(), "span": member.span()}
            )
        else:
            diagnostics.append(_diagnostic("unsupported", "unsupported handler member", member))
    return result


def _handler(
    segment: Segment, diagnostics: list[dict[str, Any]], source_state: str | None
) -> dict[str, Any]:
    match = _PROCESS_RE.match(segment.text)
    if match is None:
        raise StructuralFailure("invalid process declaration", segment)
    body, body_line, _header = _block_body(segment)
    target = match.group("target")
    return {
        "kind": match.group("kind"),
        "name": match.group("name"),
        "source_state": source_state,
        "target_state": target
        if source_state is not None and match.group("kind") == "Transition"
        else None,
        "return_type": target if source_state is None else None,
        "type_parameters": _type_parameters(match.group("type_parameters"), segment),
        "parameters": _parameters(match.group("parameters"), segment),
        "members": _handler_members(body, segment.path, body_line, diagnostics),
        "span": segment.span(),
    }


def _process_container(
    segment: Segment, diagnostics: list[dict[str, Any]], source_state: str | None
) -> list[dict[str, Any]]:
    body, body_line, _header = _block_body(segment)
    handlers: list[dict[str, Any]] = []
    for member in _split_members(body, path=segment.path, start_line=body_line):
        if _PROCESS_RE.match(member.text):
            handlers.append(_handler(member, diagnostics, source_state))
        else:
            diagnostics.append(_diagnostic("unsupported", "unsupported process container member", member))
    return handlers


def _state(segment: Segment, owner: str, diagnostics: list[dict[str, Any]]) -> dict[str, Any]:
    match = _STATE_RE.match(segment.text)
    if match is None:
        raise StructuralFailure("invalid state declaration", segment)
    body, body_line, _header = _block_body(segment)
    state = {"name": match.group(1), "invariant": [], "handlers": [], "boundaries": [], "span": segment.span()}
    for member in _split_members(body, path=segment.path, start_line=body_line):
        direct = _PROCESS_RE.match(member.text)
        block = _BLOCK_RE.match(member.text)
        if direct:
            state["handlers"].append(_handler(member, diagnostics, state["name"]))
        elif block and block.group(1) == "invariant":
            state["invariant"].extend(_plain_block(member)["entries"])
        elif block and block.group(1) in {"transitions", "actions", "processes"}:
            state["handlers"].extend(_process_container(member, diagnostics, state["name"]))
        elif block and block.group(1) in {"deferred", "trimmed"}:
            state["boundaries"].append(_boundary(member, block.group(1), diagnostics))
        else:
            diagnostics.append(_diagnostic("unsupported", f"unsupported state member on {owner}", member))
    return state


def _field_block(segment: Segment, diagnostics: list[dict[str, Any]]) -> dict[str, Any]:
    body, body_line, header = _block_body(segment)
    fields: list[dict[str, Any]] = []
    for member in _split_members(body, path=segment.path, start_line=body_line):
        raw = member.text.strip()[:-1].strip()
        mutable = raw.startswith("mutable ")
        if mutable:
            raw = raw[len("mutable ") :].strip()
        indexed_match = re.fullmatch(
            rf"indexed\s+({_IDENT})\s*\[\s*({_IDENT})\s*:\s*(.+?)\s*\]\s*:\s*(.+)",
            raw,
            re.S,
        )
        if indexed_match is not None:
            fields.append(
                {
                    "name": indexed_match.group(1),
                    "type": indexed_match.group(4).strip(),
                    "value": None,
                    "mutable": mutable,
                    "indexed": True,
                    "key_name": indexed_match.group(2),
                    "key_type": indexed_match.group(3).strip(),
                    "span": member.span(),
                }
            )
            continue
        name_match = re.match(rf"^({_IDENT})\b", raw)
        if name_match is None:
            diagnostics.append(_diagnostic("unsupported", "unsupported field declaration", member))
            continue
        name = name_match.group(1)
        rest = raw[name_match.end() :].strip()
        type_name: str | None = None
        value: str | None = None
        if rest.startswith(":"):
            typed = rest[1:].strip()
            parts = typed.split("=", 1)
            type_name = parts[0].strip()
            value = parts[1].strip() if len(parts) == 2 else None
        elif rest.startswith("="):
            value = rest[1:].strip()
        else:
            diagnostics.append(_diagnostic("unsupported", "field requires ':' or '='", member))
            continue
        fields.append(
            {
                "name": name,
                "type": type_name,
                "value": value,
                "mutable": mutable,
                "indexed": False,
                "key_name": None,
                "key_type": None,
                "span": member.span(),
            }
        )
    return {"header": header, "fields": fields, "span": segment.span()}


_TYPE_BLOCKS = frozenset({"attrs", "owned", "associations", "slots"})
_OBJECT_BLOCKS = frozenset({"attrs", "owned", "associations", "references"})


def _type(segment: Segment, diagnostics: list[dict[str, Any]]) -> dict[str, Any]:
    match = re.match(rf"^\s*type\s+({_IDENT})(?P<header>[^{{]*)\{{", segment.text, re.S)
    if match is None:
        raise StructuralFailure("invalid type declaration", segment)
    header = match.group("header").strip()
    base_match = re.search(rf":\s*({_IDENT})", header)
    body, body_line, _unused = _block_body(segment)
    result: dict[str, Any] = {
        "name": match.group(1),
        "header": header,
        "base_type": base_match.group(1) if base_match else None,
        "initial_state": None,
        "properties": {},
        "fields": {},
        "states": [],
        "processes": [],
        "boundaries": [],
        "invariant": [],
        "span": segment.span(),
    }
    for member in _split_members(body, path=segment.path, start_line=body_line):
        state_match = _STATE_RE.match(member.text)
        block = _BLOCK_RE.match(member.text)
        prop = _PROP_RE.match(member.text)
        if state_match:
            result["states"].append(_state(member, result["name"], diagnostics))
        elif block and block.group(1) in {"lifecycle", "processes", "transitions", "actions"}:
            result["processes"].extend(_process_container(member, diagnostics, None))
        elif block and block.group(1) == "invariant":
            result["invariant"].extend(_plain_block(member)["entries"])
        elif block and block.group(1) in _TYPE_BLOCKS:
            result["fields"][block.group(1)] = _field_block(member, diagnostics)["fields"]
        elif block and block.group(1) in {"deferred", "trimmed"}:
            result["boundaries"].append(_boundary(member, block.group(1), diagnostics))
        elif prop:
            result["properties"][prop.group(1)] = prop.group(2).strip()
            if prop.group(1) == "initial_state":
                result["initial_state"] = prop.group(2).strip().removeprefix("State::")
        else:
            diagnostics.append(_diagnostic("unsupported", f"unsupported type member on {result['name']}", member))
    return result


def _system(segment: Segment, diagnostics: list[dict[str, Any]]) -> dict[str, Any]:
    match = re.match(
        rf"^\s*(object|system)\s+({_IDENT})(?:\s*:\s*({_IDENT}))?\s*\{{", segment.text, re.S
    )
    if match is None:
        raise StructuralFailure("invalid object/system declaration", segment)
    body, body_line, _header = _block_body(segment)
    result: dict[str, Any] = {
        "name": match.group(2),
        "declaration_kind": match.group(1),
        "declared_type": match.group(3),
        "parent": None,
        "initial_state": None,
        "properties": {},
        "fields": {},
        "references": {},
        "reference_types": {},
        "initial_facts": [],
        "states": [],
        "processes": [],
        "boundaries": [],
        "span": segment.span(),
    }
    for member in _split_members(body, path=segment.path, start_line=body_line):
        state_match = _STATE_RE.match(member.text)
        block = _BLOCK_RE.match(member.text)
        prop = _PROP_RE.match(member.text)
        raw = member.text.strip()
        if state_match:
            result["states"].append(_state(member, result["name"], diagnostics))
        elif block and block.group(1) in {"actions", "processes", "lifecycle"}:
            result["processes"].extend(_process_container(member, diagnostics, None))
        elif block and block.group(1) in _OBJECT_BLOCKS:
            fields = _field_block(member, diagnostics)["fields"]
            result["fields"][block.group(1)] = fields
            if block.group(1) in {"associations", "references"}:
                for field in fields:
                    if field["type"]:
                        result["reference_types"][field["name"]] = field["type"]
                    if field["value"]:
                        result["references"][field["name"]] = field["value"]
        elif block and block.group(1) == "reference":
            result["fields"].setdefault("reference_evidence", []).append(_plain_block(member))
        elif block and block.group(1) == "facts":
            result["initial_facts"].extend(_plain_block(member)["entries"])
        elif block and block.group(1) in {"deferred", "trimmed"}:
            result["boundaries"].append(_boundary(member, block.group(1), diagnostics))
        elif raw.startswith("ref ") and raw.endswith(";"):
            ref = raw[len("ref ") : -1].strip()
            name, colon, rest = ref.partition(":")
            if not colon:
                diagnostics.append(_diagnostic("error", "ref requires a type", member))
            else:
                type_parts = rest.split("=", 1)
                result["reference_types"][name.strip()] = type_parts[0].strip()
                if len(type_parts) == 2:
                    result["references"][name.strip()] = type_parts[1].strip()
        elif prop:
            key, value = prop.group(1), prop.group(2).strip()
            result["properties"][key] = value
            if key == "initial_state":
                result["initial_state"] = value.removeprefix("State::")
            elif key == "parent":
                result["parent"] = value
        else:
            diagnostics.append(_diagnostic("unsupported", f"unsupported object member on {result['name']}", member))
    return result


def _predicate(segment: Segment, diagnostics: list[dict[str, Any]]) -> dict[str, Any]:
    match = re.match(rf"^\s*(predicate|function)\s+({_IDENT})(?P<rest>.*)$", segment.text, re.S)
    if match is None:
        raise StructuralFailure("invalid predicate/function declaration", segment)
    rest = match.group("rest").strip()
    opening = rest.find("{")
    signature = rest.rstrip(";").strip() if opening < 0 else rest[:opening].strip()
    parameters: list[dict[str, Any]] = []
    paren = signature.find("(")
    if paren >= 0:
        depth = 0
        closing = None
        for index in range(paren, len(signature)):
            if signature[index] == "(":
                depth += 1
            elif signature[index] == ")":
                depth -= 1
                if depth == 0:
                    closing = index
                    break
        if closing is not None:
            parameters = _parameters(signature[paren + 1 : closing], segment)
    body_text = None
    if opening >= 0:
        body_text, _body_line, _header = _block_body(segment)
        body_text = " ".join(body_text.split())
    return {
        "kind": match.group(1),
        "name": match.group(2),
        "signature": signature,
        "parameters": parameters,
        "body": body_text,
        "span": segment.span(),
    }


def _context(segment: Segment, diagnostics: list[dict[str, Any]]) -> dict[str, Any]:
    match = re.match(
        rf"^\s*(exclusive_context|context)\s+({_IDENT})(?:\s*:\s*({_IDENT}))?\s*\{{",
        segment.text,
        re.S,
    )
    if match is None:
        raise StructuralFailure("invalid context declaration", segment)
    body, body_line, _header = _block_body(segment)
    members: list[dict[str, Any]] = []
    for member in _split_members(body, path=segment.path, start_line=body_line):
        block = _BLOCK_RE.match(member.text)
        prop = _PROP_RE.match(member.text)
        if block and block.group(1) in {"guard", "obj_refs", "effects", "entered_by", "exited_by", "holds"}:
            child_body, child_line, child_header = _block_body(member)
            if block.group(1) == "guard":
                children = []
                for child in _split_members(child_body, path=member.path, start_line=child_line):
                    child_block = _BLOCK_RE.match(child.text)
                    child_prop = _PROP_RE.match(child.text)
                    if child_block:
                        children.append(_plain_block(child))
                    elif child_prop:
                        children.append(
                            {"kind": "property", "name": child_prop.group(1), "value": child_prop.group(2).strip(), "span": child.span()}
                        )
                    else:
                        diagnostics.append(_diagnostic("unsupported", "unsupported guard member", child))
                members.append({"kind": "guard", "header": child_header, "members": children, "span": member.span()})
            else:
                members.append(_plain_block(member))
        elif prop:
            members.append({"kind": "property", "name": prop.group(1), "value": prop.group(2).strip(), "span": member.span()})
        else:
            diagnostics.append(_diagnostic("unsupported", "unsupported context member", member))
    return {
        "name": match.group(2),
        "declaration_kind": match.group(1),
        "declared_type": match.group(3),
        "members": members,
        "span": segment.span(),
    }


def _enum(segment: Segment) -> dict[str, Any]:
    match = re.match(rf"^\s*enum\s+({_IDENT})\s*\{{", segment.text, re.S)
    if match is None:
        raise StructuralFailure("invalid enum declaration", segment)
    body, _body_line, _header = _block_body(segment)
    values = [part.strip() for part in re.split(r"[,;]", body) if part.strip()]
    return {"name": match.group(1), "values": values, "span": segment.span()}


def _external(segment: Segment, diagnostics: list[dict[str, Any]]) -> dict[str, Any]:
    match = re.match(rf"^\s*external\s+({_IDENT})\s*\{{", segment.text, re.S)
    if match is None:
        raise StructuralFailure("invalid external declaration", segment)
    body, body_line, _header = _block_body(segment)
    result = {"name": match.group(1), "drives": [], "emits": [], "span": segment.span()}
    seen: set[str] = set()
    for member in _split_members(body, path=segment.path, start_line=body_line):
        block = _BLOCK_RE.match(member.text)
        if block is None or block.group(1) not in {"drives", "emits"}:
            diagnostics.append(
                _diagnostic("error", "external members must be drives or emits blocks", member)
            )
            continue
        kind = block.group(1)
        if kind in seen:
            diagnostics.append(_diagnostic("error", f"duplicate external {kind} block", member))
            continue
        seen.add(kind)
        result[kind] = _plain_block(member, kind)["entries"]
    if not result["drives"]:
        diagnostics.append(_diagnostic("error", "external declaration requires drives", segment))
    if not result["emits"]:
        diagnostics.append(_diagnostic("error", "external declaration requires emits", segment))
    return result


def _diagnostic(category: str, message: str, segment: Segment) -> dict[str, Any]:
    return {"category": category, "message": message, "span": segment.span()}


def parse_full_spec(path: str | Path) -> dict[str, Any]:
    root = Path(path).resolve()
    diagnostics: list[dict[str, Any]] = []
    document: dict[str, Any] = {
        "root": stable_source_path(root),
        "includes": [],
        "enums": [],
        "types": [],
        "predicates": [],
        "functions": [],
        "locks": [],
        "contexts": [],
        "externals": [],
        "systems": [],
        "diagnostics": diagnostics,
    }
    seen: set[Path] = set()
    active: list[Path] = []

    def visit(current: Path) -> None:
        resolved = current.resolve()
        source_file = stable_source_path(resolved)
        if resolved in active:
            segment = Segment(source_file, source_file, 1, 1)
            diagnostics.append(_diagnostic("error", "include cycle detected", segment))
            return
        if resolved in seen:
            return
        try:
            source_text = resolved.read_text(encoding="utf-8")
            text = _strip_comments(source_text)
            members = _split_members(text, path=source_file, start_line=1)
        except (OSError, UnicodeError, ValueError, StructuralFailure) as exc:
            if isinstance(exc, StructuralFailure):
                diagnostics.append(_diagnostic("error", exc.message, exc.segment))
            else:
                diagnostics.append(_diagnostic("error", str(exc), Segment("", source_file, 1, 1)))
            return
        seen.add(resolved)
        active.append(resolved)
        for member in members:
            head_match = _TOP_RE.match(member.text)
            head = "" if head_match is None else head_match.group(1)
            try:
                if head == "include":
                    include = re.match(r'^\s*include\s+"([^"]+)"\s*;', member.text, re.S)
                    if include is None:
                        raise StructuralFailure("invalid include declaration", member)
                    child = (resolved.parent / json.loads(json.dumps(include.group(1)))).resolve()
                    document["includes"].append(
                        {"path": stable_source_path(child), "span": member.span()}
                    )
                    visit(child)
                elif head == "enum":
                    document["enums"].append(_enum(member))
                elif head == "type":
                    declaration = _type(member, diagnostics)
                    _attach_handler_descriptions(declaration, source_text)
                    document["types"].append(declaration)
                elif head in {"object", "system"}:
                    declaration = _system(member, diagnostics)
                    _attach_handler_descriptions(declaration, source_text)
                    document["systems"].append(declaration)
                elif head == "external":
                    document["externals"].append(_external(member, diagnostics))
                elif head in {"predicate", "function"}:
                    document[head + "s"].append(_predicate(member, diagnostics))
                elif head in {"context", "exclusive_context"}:
                    document["contexts"].append(_context(member, diagnostics))
                elif head == "lock":
                    lock = re.match(rf"^\s*lock\s+({_IDENT})(?:\s*:\s*({_IDENT}))?\s*;", member.text)
                    if lock is None:
                        raise StructuralFailure("invalid lock declaration", member)
                    document["locks"].append({"name": lock.group(1), "declared_type": lock.group(2), "span": member.span()})
                else:
                    diagnostics.append(_diagnostic("unsupported", f"unsupported top-level declaration {head!r}", member))
            except StructuralFailure as exc:
                diagnostics.append(_diagnostic("error", exc.message, exc.segment))
        active.pop()

    visit(root)
    return document


__all__ = ["parse_full_spec"]
