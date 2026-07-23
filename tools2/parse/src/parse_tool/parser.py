"""A strict parser for the deliberately small tools2 Signal DSL subset."""

from __future__ import annotations

from dataclasses import dataclass
import json
from pathlib import Path
from typing import Any

from tools2_common import stable_source_path


@dataclass(frozen=True)
class Token:
    value: str
    path: str
    line: int
    column: int
    kind: str = "symbol"

    def span(self) -> dict[str, Any]:
        return {
            "source_file": self.path,
            "start_line": self.line,
            "start_column": self.column,
            "end_line": self.line,
            "end_column": self.column + max(1, len(self.value)),
        }


class ParseFailure(ValueError):
    def __init__(self, message: str, token: Token):
        super().__init__(message)
        self.message = message
        self.token = token


def _tokenize(text: str, path: Path) -> list[Token]:
    tokens: list[Token] = []
    index = 0
    line = 1
    column = 1
    length = len(text)

    def advance(value: str) -> None:
        nonlocal line, column
        parts = value.split("\n")
        if len(parts) == 1:
            column += len(value)
        else:
            line += len(parts) - 1
            column = len(parts[-1]) + 1

    while index < length:
        char = text[index]
        if char.isspace():
            advance(char)
            index += 1
            continue
        if text.startswith("//", index):
            end = text.find("\n", index)
            end = length if end < 0 else end
            advance(text[index:end])
            index = end
            continue
        if text.startswith("/*", index):
            end = text.find("*/", index + 2)
            if end < 0:
                raise ParseFailure("unterminated block comment", Token("/*", str(path), line, column))
            end += 2
            advance(text[index:end])
            index = end
            continue
        start_line, start_column = line, column
        if char == '"':
            end = index + 1
            escaped = False
            while end < length:
                current = text[end]
                if current == '"' and not escaped:
                    end += 1
                    break
                escaped = current == "\\" and not escaped
                if current != "\\":
                    escaped = False
                end += 1
            else:
                raise ParseFailure(
                    "unterminated string literal", Token(char, str(path), start_line, start_column)
                )
            value = text[index:end]
            tokens.append(Token(value, str(path), start_line, start_column, "string"))
            advance(value)
            index = end
            continue
        if char.isalpha() or char == "_":
            end = index + 1
            while end < length and (text[end].isalnum() or text[end] in "_-"):
                end += 1
            value = text[index:end]
            tokens.append(Token(value, str(path), start_line, start_column, "identifier"))
            advance(value)
            index = end
            continue
        if char.isdigit() or (char == "-" and index + 1 < length and text[index + 1].isdigit()):
            end = index + 1
            while end < length and text[end].isdigit():
                end += 1
            value = text[index:end]
            tokens.append(Token(value, str(path), start_line, start_column, "integer"))
            advance(value)
            index = end
            continue
        operator = next(
            (item for item in ("::", "->", "==", "!=") if text.startswith(item, index)), None
        )
        if operator is not None:
            tokens.append(Token(operator, str(path), start_line, start_column))
            advance(operator)
            index += len(operator)
            continue
        if char in "{}():;,.=":
            tokens.append(Token(char, str(path), start_line, start_column))
            advance(char)
            index += 1
            continue
        raise ParseFailure(
            f"unsupported character {char!r}", Token(char, str(path), start_line, start_column)
        )
    tokens.append(Token("<eof>", str(path), line, column, "eof"))
    return tokens


def _render_tokens(tokens: list[Token]) -> str:
    result = ""
    previous = ""
    tight = {".", "::", ")", ","}
    after_tight = {".", "::", "("}
    for token in tokens:
        value = token.value
        if result and value not in tight and previous not in after_tight:
            result += " "
        result += value
        previous = value
    return result


def _span(start: Token, end: Token | None = None) -> dict[str, Any]:
    finish = start if end is None else end
    return {
        "source_file": start.path,
        "start_line": start.line,
        "start_column": start.column,
        "end_line": finish.line,
        "end_column": finish.column + max(1, len(finish.value)),
    }


class Parser:
    def __init__(self, tokens: list[Token], diagnostics: list[dict[str, Any]]):
        self.tokens = tokens
        self.index = 0
        self.diagnostics = diagnostics

    @property
    def current(self) -> Token:
        return self.tokens[self.index]

    def at(self, value: str) -> bool:
        return self.current.value == value

    def take(self) -> Token:
        token = self.current
        self.index += 1
        return token

    def accept(self, value: str) -> Token | None:
        if self.at(value):
            return self.take()
        return None

    def expect(self, value: str) -> Token:
        if not self.at(value):
            raise ParseFailure(f"expected {value!r}, got {self.current.value!r}", self.current)
        return self.take()

    def identifier(self) -> Token:
        if self.current.kind != "identifier":
            raise ParseFailure(f"expected identifier, got {self.current.value!r}", self.current)
        return self.take()

    def diagnostic(self, category: str, message: str, token: Token) -> None:
        self.diagnostics.append(
            {"category": category, "message": message, "span": token.span()}
        )

    def skip_construct(self) -> None:
        depth = 0
        while not self.at("<eof>"):
            value = self.take().value
            if value == "{":
                depth += 1
            elif value == "}":
                if depth == 0:
                    self.index -= 1
                    return
                depth -= 1
                if depth == 0:
                    return
            elif value == ";" and depth == 0:
                return

    def parse_enum(self) -> dict[str, Any]:
        start = self.expect("enum")
        name = self.identifier().value
        self.expect("{")
        values: list[str] = []
        while not self.at("}"):
            values.append(self.identifier().value)
            if not self.accept(","):
                self.accept(";")
        end = self.expect("}")
        self.accept(";")
        return {"name": name, "values": values, "span": _span(start, end)}

    def parse_system(self) -> dict[str, Any]:
        start = self.take()
        declaration_kind = start.value
        name = self.identifier().value
        declared_type = None
        if self.accept(":"):
            declared_type = self.identifier().value
        self.expect("{")
        system: dict[str, Any] = {
            "name": name,
            "declaration_kind": declaration_kind,
            "declared_type": declared_type,
            "parent": None,
            "initial_state": None,
            "reference_types": {},
            "references": {},
            "initial_facts": [],
            "states": [],
            "span": start.span(),
        }
        while not self.at("}") and not self.at("<eof>"):
            try:
                if self.at("parent"):
                    self.take()
                    self.expect(":")
                    system["parent"] = self.identifier().value
                    self.expect(";")
                elif self.at("initial_state") or self.at("initial"):
                    self.take()
                    if self.tokens[self.index - 1].value == "initial" and self.at("state"):
                        self.take()
                    self.expect(":")
                    self.accept("State")
                    self.accept("::")
                    system["initial_state"] = self.identifier().value
                    self.expect(";")
                elif self.at("references"):
                    self._parse_references(system)
                elif self.at("ref"):
                    self._parse_reference_declaration(system)
                elif self.at("facts"):
                    self.take()
                    system["initial_facts"].extend(self.parse_expression_block("fact"))
                elif self.at("state"):
                    system["states"].append(self.parse_state(name))
                else:
                    token = self.current
                    self.diagnostic(
                        "unsupported",
                        f"unsupported {declaration_kind} member {token.value!r}",
                        token,
                    )
                    self.skip_construct()
            except ParseFailure as exc:
                self.diagnostic("error", exc.message, exc.token)
                self.skip_construct()
        end = self.expect("}")
        system["span"] = _span(start, end)
        self.accept(";")
        return system

    def _parse_references(self, system: dict[str, Any]) -> None:
        self.expect("references")
        self.expect("{")
        while not self.at("}"):
            name = self.identifier()
            if self.accept(":"):
                system["reference_types"][name.value] = self.identifier().value
                if self.accept("="):
                    system["references"][name.value] = self.parse_path()
            elif self.accept("="):
                system["references"][name.value] = self.parse_path()
            else:
                raise ParseFailure("expected ':' or '=' in reference declaration", self.current)
            self.expect(";")
        self.expect("}")

    def _parse_reference_declaration(self, system: dict[str, Any]) -> None:
        self.expect("ref")
        name = self.identifier().value
        self.expect(":")
        system["reference_types"][name] = self.identifier().value
        if self.accept("="):
            system["references"][name] = self.parse_path()
        self.expect(";")

    def parse_state(self, owner: str) -> dict[str, Any]:
        start = self.expect("state")
        self.accept("State")
        self.accept("::")
        name = self.identifier().value
        self.expect("{")
        state = {"name": name, "invariant": [], "handlers": [], "span": start.span()}
        while not self.at("}") and not self.at("<eof>"):
            if self.at("invariant"):
                self.take()
                state["invariant"].extend(self.parse_expression_block("condition"))
            elif self.at("transitions"):
                self.take()
                self.expect("{")
                while not self.at("}"):
                    state["handlers"].append(self.parse_handler(owner, name, "Transition"))
                self.expect("}")
            elif self.at("actions"):
                self.take()
                self.expect("{")
                while not self.at("}"):
                    state["handlers"].append(self.parse_handler(owner, name, "Action"))
                self.expect("}")
            elif self.at("on"):
                kind = self.tokens[self.index + 1].value if self.index + 1 < len(self.tokens) else ""
                if kind not in {"Transition", "Action"}:
                    raise ParseFailure("expected Transition or Action after 'on'", self.current)
                state["handlers"].append(self.parse_handler(owner, name, kind))
            else:
                token = self.current
                self.diagnostic("unsupported", f"unsupported state member {token.value!r}", token)
                self.skip_construct()
        end = self.expect("}")
        state["span"] = _span(start, end)
        return state

    def parse_handler(self, owner: str, source_state: str, expected_kind: str) -> dict[str, Any]:
        start = self.expect("on")
        kind = self.identifier().value
        if kind != expected_kind:
            raise ParseFailure(f"expected {expected_kind} handler, got {kind}", start)
        self.expect("::")
        name = self.identifier().value
        parameters: list[dict[str, Any]] = []
        if self.accept("("):
            while not self.at(")"):
                parameter = self.identifier()
                self.expect(":")
                type_name = self.identifier().value
                parameters.append({"name": parameter.value, "type": type_name, "span": parameter.span()})
                if not self.accept(","):
                    break
            self.expect(")")
        target_state = None
        if self.accept("->"):
            self.accept("State")
            self.accept("::")
            target_state = self.identifier().value
        elif kind == "Transition":
            raise ParseFailure("Transition handler requires '-> State::Target'", self.current)
        self.expect("{")
        body: list[dict[str, Any]] = []
        while not self.at("}") and not self.at("<eof>"):
            token = self.current
            if token.value in {"depends_on", "ensures", "updates"}:
                block_kind = self.take().value
                expression_kind = "condition" if block_kind == "depends_on" else "effect"
                body.append(
                    {
                        "kind": block_kind,
                        "entries": self.parse_expression_block(expression_kind, assignment=block_kind == "updates"),
                        "span": token.span(),
                    }
                )
            elif token.value in {"drives", "emits"}:
                block_kind = self.take().value
                body.append(
                    {
                        "kind": block_kind,
                        "entries": self.parse_call_block(block_kind),
                        "span": token.span(),
                    }
                )
            else:
                self.diagnostic("unsupported", f"unsupported handler member {token.value!r}", token)
                self.skip_construct()
        end = self.expect("}")
        return {
            "owner": owner,
            "kind": kind,
            "name": name,
            "source_state": source_state,
            "target_state": target_state,
            "parameters": parameters,
            "body": body,
            "span": _span(start, end),
        }

    def parse_expression_block(self, expression_kind: str, assignment: bool = False) -> list[dict[str, Any]]:
        self.expect("{")
        entries: list[dict[str, Any]] = []
        while not self.at("}") and not self.at("<eof>"):
            tokens = self.collect_statement()
            if not tokens:
                continue
            try:
                expression = _parse_expression(tokens, assignment=assignment)
                expression["span"] = _span(tokens[0], tokens[-1])
                expression["text"] = _render_tokens(tokens)
                entries.append(expression)
            except ParseFailure as exc:
                self.diagnostic("unsupported", exc.message, exc.token)
        self.expect("}")
        return entries

    def parse_call_block(self, delivery: str) -> list[dict[str, Any]]:
        self.expect("{")
        entries: list[dict[str, Any]] = []
        while not self.at("}") and not self.at("<eof>"):
            tokens = self.collect_statement()
            if not tokens:
                continue
            try:
                call = _parse_call(tokens)
                call["delivery"] = delivery
                call["span"] = _span(tokens[0], tokens[-1])
                call["text"] = _render_tokens(tokens)
                entries.append(call)
            except ParseFailure as exc:
                self.diagnostic("unsupported", exc.message, exc.token)
        self.expect("}")
        return entries

    def collect_statement(self) -> list[Token]:
        tokens: list[Token] = []
        depth = 0
        while not self.at("<eof>"):
            if self.at("}") and depth == 0:
                if tokens:
                    raise ParseFailure("statement requires ';'", self.current)
                return []
            token = self.take()
            if token.value == "(" :
                depth += 1
            elif token.value == ")":
                depth -= 1
            if token.value == ";" and depth == 0:
                return tokens
            tokens.append(token)
        raise ParseFailure("unterminated statement", self.current)

    def parse_path(self) -> str:
        parts = [self.identifier().value]
        while self.accept("."):
            parts.append(self.identifier().value)
        return ".".join(parts)


def _split_top(tokens: list[Token], separator: str) -> list[list[Token]]:
    result: list[list[Token]] = []
    current: list[Token] = []
    depth = 0
    for token in tokens:
        if token.value == "(":
            depth += 1
        elif token.value == ")":
            depth -= 1
        if token.value == separator and depth == 0:
            result.append(current)
            current = []
        else:
            current.append(token)
    result.append(current)
    return result


def _path_value(tokens: list[Token]) -> dict[str, Any]:
    if len(tokens) == 1:
        token = tokens[0]
        if token.kind == "string":
            return {"kind": "string", "value": json.loads(token.value)}
        if token.kind == "integer":
            return {"kind": "integer", "value": int(token.value)}
        if token.value in {"true", "false"}:
            return {"kind": "boolean", "value": token.value == "true"}
        return {"kind": "path", "value": token.value}
    if len(tokens) == 3 and tokens[1].value == "::":
        return {"kind": "enum", "type": tokens[0].value, "value": tokens[2].value}
    if tokens and all(
        (index % 2 == 0 and token.kind == "identifier")
        or (index % 2 == 1 and token.value == ".")
        for index, token in enumerate(tokens)
    ):
        return {"kind": "path", "value": ".".join(token.value for token in tokens[::2])}
    raise ParseFailure("unsupported value expression", tokens[0])


def _parse_expression(tokens: list[Token], *, assignment: bool) -> dict[str, Any]:
    operator = "=" if assignment else "=="
    positions = [index for index, token in enumerate(tokens) if token.value == operator]
    if positions:
        if len(positions) != 1:
            raise ParseFailure("expression must contain exactly one comparison/assignment", tokens[0])
        position = positions[0]
        left = _path_value(tokens[:position])
        right = _path_value(tokens[position + 1 :])
        if left["kind"] != "path":
            raise ParseFailure("left side must be a state or reference path", tokens[0])
        if left["value"].endswith(".state"):
            if right["kind"] != "enum" or right.get("type") != "State":
                raise ParseFailure("state expressions require State::Name", tokens[position + 1])
            return {
                "kind": "state_assignment" if assignment else "state_condition",
                "target": left["value"][: -len(".state")],
                "state": right["value"],
            }
        return {
            "kind": "reference_assignment" if assignment else "reference_condition",
            "reference": left["value"],
            "value": right,
        }
    if assignment:
        raise ParseFailure("updates only support state/reference assignment", tokens[0])
    if len(tokens) == 1 and tokens[0].kind == "identifier":
        return {"kind": "fact", "name": tokens[0].value, "arguments": []}
    if len(tokens) >= 3 and tokens[0].kind == "identifier" and tokens[1].value == "(" and tokens[-1].value == ")":
        arguments = []
        inner = tokens[2:-1]
        if inner:
            arguments = [_path_value(part) for part in _split_top(inner, ",")]
        return {"kind": "fact", "name": tokens[0].value, "arguments": arguments}
    raise ParseFailure("unsupported condition/fact expression", tokens[0])


def _parse_call(tokens: list[Token]) -> dict[str, Any]:
    if tokens and tokens[0].value == "lossy":
        raise ParseFailure("lossy Signal syntax was removed in protocol v4", tokens[0])
    process_position = next(
        (
            index
            for index in range(len(tokens) - 2)
            if tokens[index].value in {"Transition", "Action"} and tokens[index + 1].value == "::"
        ),
        None,
    )
    if process_position is None:
        raise ParseFailure("expected Receiver.Transition::Name or Receiver.Action::Name", tokens[0])
    process_kind = tokens[process_position].value
    name = tokens[process_position + 2].value
    receiver_tokens = tokens[:process_position]
    if receiver_tokens and receiver_tokens[-1].value == ".":
        receiver_tokens = receiver_tokens[:-1]
    receiver = "self" if not receiver_tokens else _path_value(receiver_tokens)["value"]
    remainder = tokens[process_position + 3 :]
    arguments: list[dict[str, Any]] = []
    if remainder:
        if remainder[0].value != "(" or remainder[-1].value != ")":
            raise ParseFailure("malformed call arguments", remainder[0])
        inner = remainder[1:-1]
        for part in ([] if not inner else _split_top(inner, ",")):
            colon = next((index for index, token in enumerate(part) if token.value == ":"), None)
            if colon is None:
                arguments.append({"name": None, "value": _path_value(part)})
            else:
                if colon != 1 or part[0].kind != "identifier":
                    raise ParseFailure("named argument must be name: value", part[0])
                arguments.append({"name": part[0].value, "value": _path_value(part[colon + 1 :])})
    return {
        "receiver": receiver,
        "process_kind": process_kind,
        "name": name,
        "arguments": arguments,
    }


def parse_spec(path: str | Path) -> dict[str, Any]:
    from .full_parser import parse_full_spec

    return parse_full_spec(path)


def _parse_subset_spec(path: str | Path) -> dict[str, Any]:
    root = Path(path).resolve()
    diagnostics: list[dict[str, Any]] = []
    enums: list[dict[str, Any]] = []
    systems: list[dict[str, Any]] = []
    includes: list[dict[str, Any]] = []
    seen: set[Path] = set()
    active: list[Path] = []

    def visit(current: Path) -> None:
        resolved = current.resolve()
        source_file = stable_source_path(resolved)
        if resolved in active:
            token = Token(source_file, source_file, 1, 1)
            diagnostics.append(
                {"category": "error", "message": "include cycle detected", "span": token.span()}
            )
            return
        if resolved in seen:
            return
        try:
            text = resolved.read_text(encoding="utf-8")
            tokens = _tokenize(text, Path(source_file))
        except (OSError, UnicodeError, ParseFailure) as exc:
            if isinstance(exc, ParseFailure):
                diagnostics.append(
                    {"category": "error", "message": exc.message, "span": exc.token.span()}
                )
            else:
                diagnostics.append(
                    {
                        "category": "error",
                        "message": str(exc),
                        "span": Token(source_file, source_file, 1, 1).span(),
                    }
                )
            return
        seen.add(resolved)
        active.append(resolved)
        parser = Parser(tokens, diagnostics)
        while not parser.at("<eof>"):
            try:
                if parser.at("include"):
                    start = parser.take()
                    literal = parser.take()
                    if literal.kind != "string":
                        raise ParseFailure("include requires a string path", literal)
                    parser.expect(";")
                    child = (resolved.parent / json.loads(literal.value)).resolve()
                    includes.append({"path": stable_source_path(child), "span": start.span()})
                    visit(child)
                elif parser.at("enum"):
                    enums.append(parser.parse_enum())
                elif parser.at("object") or parser.at("system"):
                    systems.append(parser.parse_system())
                else:
                    token = parser.current
                    parser.diagnostic("unsupported", f"unsupported top-level declaration {token.value!r}", token)
                    parser.skip_construct()
            except ParseFailure as exc:
                parser.diagnostic("error", exc.message, exc.token)
                parser.skip_construct()
        active.pop()

    visit(root)
    return {
        "root": stable_source_path(root),
        "includes": includes,
        "enums": enums,
        "systems": systems,
        "diagnostics": diagnostics,
    }
