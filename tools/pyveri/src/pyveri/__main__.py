"""Driver entry point for the pyveri toolchain."""

from __future__ import annotations

import argparse
from collections.abc import Iterator
from contextlib import contextmanager
import os
import signal
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any


def _bootstrap_tool_paths() -> None:
    tools_root = Path(__file__).resolve().parents[3]
    for name in ("common", "parse", "model", "derive", "check", "view", "render"):
        source = str(tools_root / name / "src")
        if source not in sys.path:
            sys.path.insert(0, source)


_bootstrap_tool_paths()

from common import read_json
from pyveri.derive import DEFAULT_TARGET

if hasattr(signal, "SIGPIPE"):
    signal.signal(signal.SIGPIPE, signal.SIG_DFL)


VIEW_CHOICES = ("object", "drives", "timeline")
TEXT_VIEW_CHOICES = (*VIEW_CHOICES, "trace")
COMMANDS = frozenset({"parse", "model", "derive", "check", "view", "render"})


def main(argv: list[str] | None = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    parser = _build_command_parser() if argv and argv[0] in COMMANDS else _build_legacy_parser()
    args = parser.parse_args(argv)

    if not hasattr(args, "command"):
        return _run_legacy(args, parser)
    if args.command == "parse":
        return _run_parse(args)
    if args.command == "model":
        return _run_model(args)
    if args.command == "derive":
        return _run_derive(args)
    if args.command == "check":
        return _run_check(args)
    if args.command == "view":
        return _run_view(args)
    if args.command == "render":
        return _run_render(args)
    parser.error(f"unknown command: {args.command}")
    return 2


def _build_legacy_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Run the pyveri verification driver.")
    _add_legacy_arguments(parser)
    return parser


def _build_command_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Run pyveri pipeline stages.")
    subparsers = parser.add_subparsers(dest="command")

    parse_parser = subparsers.add_parser("parse", help="parse the spec and print an AST summary")
    parse_parser.add_argument("spec", type=Path, help="path to the .spec input file")
    _add_work_dir_argument(parse_parser)

    model_parser = subparsers.add_parser("model", help="build the static object model")
    model_parser.add_argument("spec", type=Path, help="path to the .spec input file")
    _add_work_dir_argument(model_parser)

    derive_parser = subparsers.add_parser("derive", help="run static derivation")
    derive_parser.add_argument("spec", type=Path, help="path to the .spec input file")
    derive_parser.add_argument(
        "--target",
        default=DEFAULT_TARGET,
        help=f"target event, default: {DEFAULT_TARGET}",
    )
    derive_parser.add_argument(
        "--strict",
        action="store_true",
        help="return a non-zero exit code when derivation does not reach the target",
    )
    derive_parser.add_argument("-o", "--output", type=Path, help="write the derivation report")
    _add_work_dir_argument(derive_parser)

    check_parser = subparsers.add_parser("check", help="run verification check")
    check_parser.add_argument("spec", type=Path, help="path to the .spec input file")
    check_parser.add_argument(
        "--target",
        default=DEFAULT_TARGET,
        help=f"target event, default: {DEFAULT_TARGET}",
    )
    _add_work_dir_argument(check_parser)

    view_parser = subparsers.add_parser("view", help="print a plain text model view")
    view_parser.add_argument("spec", type=Path, help="path to the .spec input file")
    view_parser.add_argument("view", choices=TEXT_VIEW_CHOICES, help="view name")
    view_parser.add_argument(
        "--target",
        default=DEFAULT_TARGET,
        help=f"target event for trace view, default: {DEFAULT_TARGET}",
    )
    view_parser.add_argument("-o", "--output", type=Path, help="write the view to a file")
    _add_work_dir_argument(view_parser)

    render_parser = subparsers.add_parser("render", help="render a graph or SVG model view")
    render_parser.add_argument("spec", type=Path, help="path to the .spec input file")
    render_parser.add_argument("view", choices=VIEW_CHOICES, help="view name")
    render_parser.add_argument(
        "--format",
        choices=("dot", "svg"),
        default="dot",
        help="render format, default: dot",
    )
    render_parser.add_argument("-o", "--output", type=Path, help="write the rendering to a file")
    _add_work_dir_argument(render_parser)

    return parser


def _add_work_dir_argument(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--work-dir",
        type=Path,
        help="retain intermediate files in this directory instead of a temporary directory",
    )


def _add_legacy_arguments(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("spec", nargs="?", type=Path, help="path to the .spec input file")
    parser.add_argument(
        "--tree",
        action="store_true",
        help="compatibility alias for --text object",
    )
    parser.add_argument("--text", choices=TEXT_VIEW_CHOICES, metavar="VIEW", help="print text view")
    parser.add_argument("--graph", choices=VIEW_CHOICES, metavar="VIEW", help="print graph view")
    parser.add_argument(
        "--derive",
        action="store_true",
        help="run the static derivation engine and print a derivation report",
    )
    parser.add_argument(
        "--target",
        default=DEFAULT_TARGET,
        help=f"target event for --derive, default: {DEFAULT_TARGET}",
    )
    parser.add_argument(
        "--strict",
        action="store_true",
        help="return a non-zero exit code when derivation does not reach the target",
    )
    parser.add_argument("-o", "--output", type=Path, help="write selected output to a file")
    _add_work_dir_argument(parser)


def _run_legacy(args: argparse.Namespace, parser: argparse.ArgumentParser) -> int:
    if args.spec is None:
        parser.error("the following arguments are required: spec")

    output_modes = [bool(args.tree), bool(args.text), bool(args.graph), bool(args.derive)]
    if args.output is not None:
        if not any(output_modes):
            parser.error("-o/--output requires --text, --graph, --tree, or --derive")
        if sum(output_modes) > 1:
            parser.error("-o/--output can write only one selected output")

    with _workspace(args.work_dir) as work:
        paths = _pipeline_paths(work, args.spec)
        parse_code = _run_parse_stage(args.spec, paths["ast"])
        if parse_code != 0:
            return parse_code
        model_code = _run_model_stage(paths["ast"], paths["model"])
        if model_code != 0:
            return model_code

        selected_outputs: list[str] = []
        derivation_data: dict[str, Any] | None = None
        needs_derivation = (
            not args.graph
            or args.derive
            or args.strict
            or args.text == "trace"
        )
        if needs_derivation:
            derive_code = _run_derive_stage(paths["model"], paths["derive"], args.target)
            if derive_code != 0:
                return derive_code
            derivation_data = read_json(paths["derive"])

        graph_only = args.graph and not args.text and not args.tree and not args.derive
        output_only = args.output is not None
        if not graph_only and not output_only:
            print(_parse_summary(read_json(paths["ast"])))
            print(_model_summary(read_json(paths["model"])))
            if derivation_data is not None and not args.derive:
                print(_derive_summary(derivation_data))

        if args.tree:
            selected_outputs.append(
                _render_view_output(paths, "object", "text", work, args.spec)
            )
        if args.text:
            selected_outputs.append(
                _render_view_output(paths, args.text, "text", work, args.spec)
            )
        if args.graph:
            fmt = "svg" if args.graph == "timeline" else "dot"
            selected_outputs.append(
                _render_view_output(paths, args.graph, fmt, work, args.spec)
            )
        if args.derive and derivation_data is not None:
            selected_outputs.append(_derive_report(derivation_data))

        if args.output is not None and selected_outputs:
            _write_output(args.output, "\n\n".join(selected_outputs), ascii_only=bool(args.graph))
        else:
            for output in selected_outputs:
                print(output)

        if args.strict and derivation_data is not None and not _derive_ok(derivation_data):
            return 1
        return 0


def _run_parse(args: argparse.Namespace) -> int:
    with _workspace(args.work_dir) as work:
        ast = _pipeline_paths(work, args.spec)["ast"]
        code = _run_parse_stage(args.spec, ast)
        if code != 0:
            return code
        print(_parse_summary(read_json(ast)))
    return 0


def _run_model(args: argparse.Namespace) -> int:
    with _workspace(args.work_dir) as work:
        paths = _pipeline_paths(work, args.spec)
        code = _run_parse_stage(args.spec, paths["ast"])
        if code != 0:
            return code
        code = _run_model_stage(paths["ast"], paths["model"])
        if code != 0:
            return code
        print(_model_summary(read_json(paths["model"])))
    return 0


def _run_derive(args: argparse.Namespace) -> int:
    with _workspace(args.work_dir) as work:
        paths = _pipeline_paths(work, args.spec)
        code = _run_parse_stage(args.spec, paths["ast"])
        if code != 0:
            return code
        code = _run_model_stage(paths["ast"], paths["model"])
        if code != 0:
            return code
        code = _run_derive_stage(paths["model"], paths["derive"], args.target)
        if code != 0:
            return code
        data = read_json(paths["derive"])
        output = _derive_report(data)
        if args.output is not None:
            _write_output(args.output, output, ascii_only=False)
        else:
            print(output)
        if args.strict and not _derive_ok(data):
            return 1
    return 0


def _run_check(args: argparse.Namespace) -> int:
    with _workspace(args.work_dir) as work:
        paths = _pipeline_paths(work, args.spec)
        code = _run_parse_stage(args.spec, paths["ast"])
        if code != 0:
            return code
        code = _run_model_stage(paths["ast"], paths["model"])
        if code != 0:
            return code
        code = _run_derive_stage(paths["model"], paths["derive"], args.target)
        if code != 0:
            return code
        code = _run_check_stage(paths["derive"], paths["check"])
        if code != 0:
            return code
    return 0


def _run_view(args: argparse.Namespace) -> int:
    with _workspace(args.work_dir) as work:
        paths = _pipeline_paths(work, args.spec)
        code = _run_parse_stage(args.spec, paths["ast"])
        if code != 0:
            return code
        code = _run_model_stage(paths["ast"], paths["model"])
        if code != 0:
            return code
        if args.view == "trace":
            code = _ensure_derivation(paths, args.target)
            if code != 0:
                return code
        output = _render_view_output(paths, args.view, "text", work, args.spec)
        if args.output is not None:
            _write_output(args.output, output, ascii_only=False)
        else:
            print(output)
    return 0


def _run_render(args: argparse.Namespace) -> int:
    with _workspace(args.work_dir) as work:
        paths = _pipeline_paths(work, args.spec)
        code = _run_parse_stage(args.spec, paths["ast"])
        if code != 0:
            return code
        code = _run_model_stage(paths["ast"], paths["model"])
        if code != 0:
            return code
        output = _render_view_output(paths, args.view, args.format, work, args.spec)
        if args.output is not None:
            _write_output(args.output, output, ascii_only=args.format == "dot")
        else:
            print(output)
    return 0


def _run_parse_stage(spec: Path, output: Path) -> int:
    return _run_stage(["-m", "parse_tool", str(spec), "-o", str(output)])


def _run_model_stage(ast: Path, output: Path) -> int:
    return _run_stage(["-m", "model_tool", str(ast), "-o", str(output)])


def _run_derive_stage(model: Path, output: Path, target: str) -> int:
    return _run_stage(["-m", "derive_tool", str(model), "-o", str(output), "--target", target])


def _run_check_stage(derive: Path, output: Path) -> int:
    return _run_stage(["-m", "check_tool", str(derive), "-o", str(output)])


def _run_view_stage(model: Path, view: str, output: Path) -> int:
    return _run_stage(["-m", "view_tool", str(model), view, "-o", str(output)])


def _run_render_stage(view: Path, fmt: str, output: Path) -> int:
    return _run_stage(["-m", "render_tool", str(view), "--format", fmt, "-o", str(output)])


def _run_stage(args: list[str]) -> int:
    completed = subprocess.run(
        [sys.executable, *args],
        env=_stage_env(),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        check=False,
    )
    if completed.stdout:
        print(completed.stdout, end="")
    if completed.stderr:
        print(completed.stderr, end="", file=sys.stderr)
    return completed.returncode


@contextmanager
def _workspace(work_dir: Path | None) -> Iterator[Path]:
    if work_dir is None:
        with tempfile.TemporaryDirectory(prefix="pyveri-") as tmp:
            yield Path(tmp)
        return

    work_dir.mkdir(parents=True, exist_ok=True)
    yield work_dir


def _stage_env() -> dict[str, str]:
    env = dict(os.environ)
    root = Path(__file__).resolve().parents[3]
    paths = [
        root / "common" / "src",
        root / "parse" / "src",
        root / "model" / "src",
        root / "derive" / "src",
        root / "check" / "src",
        root / "view" / "src",
        root / "render" / "src",
        root / "pyveri" / "src",
    ]
    existing = env.get("PYTHONPATH")
    path_text = os.pathsep.join(str(path) for path in paths)
    env["PYTHONPATH"] = path_text if not existing else path_text + os.pathsep + existing
    return env


def _render_view_output(
    paths: dict[str, Path], view_name: str, fmt: str, work: Path, spec: Path
) -> str:
    stem = spec.stem
    suffix = "gv" if fmt == "dot" else fmt
    view_path = work / f"{stem}.{view_name}.view.json"
    output = work / f"{stem}.{view_name}.{suffix}"
    view_input = paths["derive"] if view_name == "trace" else paths["model"]
    view_code = _run_view_stage(view_input, view_name, view_path)
    if view_code != 0:
        raise SystemExit(view_code)
    render_code = _run_render_stage(view_path, fmt, output)
    if render_code != 0:
        raise SystemExit(render_code)
    encoding = "ascii" if fmt == "dot" else "utf-8"
    return output.read_text(encoding=encoding)


def _ensure_derivation(paths: dict[str, Path], target: str) -> int:
    if paths["derive"].is_file():
        return 0
    return _run_derive_stage(paths["model"], paths["derive"], target)


def _pipeline_paths(tmp: Path, spec: Path) -> dict[str, Path]:
    stem = spec.stem
    return {
        "ast": tmp / f"{stem}.ast.json",
        "model": tmp / f"{stem}.model.json",
        "derive": tmp / f"{stem}.derive.json",
        "check": tmp / f"{stem}.check.json",
    }


def _parse_summary(data: dict[str, Any]) -> str:
    document = data["document"]
    state_count = sum(len(obj["states"]) for obj in document["objects"])
    event_count = sum(
        len(state["events"])
        for obj in document["objects"]
        for state in obj["states"]
    )
    return "\n".join(
        [
            "parse: ok",
            f"enums: {len(document['enums'])}",
            f"functions: {len(document['functions'])}",
            f"predicates: {len(document['predicates'])}",
            f"types: {len(document['types'])}",
            f"objects: {len(document['objects'])}",
            f"states: {state_count}",
            f"events: {event_count}",
        ]
    )


def _model_summary(data: dict[str, Any]) -> str:
    summary = data["summary"]
    status = "ok" if summary["ok"] else "failed"
    return "\n".join(
        [
            f"model: {status}",
            f"objects: {summary['objects']}",
            f"states: {summary['states']}",
            f"events: {summary['events']}",
            f"errors: {summary['errors']}",
            f"warnings: {summary['warnings']}",
        ]
    )


def _derive_summary(data: dict[str, Any]) -> str:
    summary = data["summary"]
    status = _derive_status(summary)
    return "\n".join(
        [
            f"derive: {status}",
            f"target: {data['target']['event']}",
            f"target_reached: {'yes' if summary['target_reached'] else 'no'}",
            f"transitions: {summary['transitions']}",
            f"proved: {summary['proved']}",
            f"assumed: {summary['assumed']}",
            f"obligation: {summary['obligation']}",
            f"deferred: {summary['deferred']}",
            f"blocked: {summary['blocked']}",
            f"contradiction: {summary['contradiction']}",
        ]
    )


def _derive_report(data: dict[str, Any]) -> str:
    lines = [_derive_summary(data)]
    trace = data.get("trace", [])
    if trace:
        lines.append("")
        lines.append("trace:")
        for node in trace:
            _append_trace_node(lines, node, depth=0)
    else:
        transitions = data.get("transitions", [])
        if transitions:
            lines.append("")
            lines.append("transitions:")
            for transition in transitions:
                lines.append(f"- {transition['label']}")

    for status in ("blocked", "contradiction", "deferred", "obligation"):
        records = [record for record in data["records"] if record["status"] == status]
        if not records:
            continue
        lines.append("")
        lines.append(f"{status}:")
        for record in records:
            lines.append(f"- {_format_record(record)}")
    return "\n".join(lines)


def _append_trace_node(lines: list[str], node: dict[str, Any], depth: int) -> None:
    indent = "  " * depth
    label = node["label"]
    lines.append(f"{indent}> {label} State::{node['source_state']}")
    for child in node.get("children", []):
        _append_trace_node(lines, child, depth + 1)

    status = node.get("status")
    suffix = ""
    if status in ("blocked", "contradiction"):
        message = node.get("message")
        suffix = f" {status}: {message}" if message else f" {status}"
    lines.append(f"{indent}< {label} State::{node['target_state']}{suffix}")


def _derive_status(summary: dict[str, Any]) -> str:
    if _derive_ok_summary(summary):
        return "ok"
    if summary["contradiction"]:
        return "contradiction"
    if summary["blocked"]:
        return "blocked"
    return "incomplete"


def _derive_ok(data: dict[str, Any]) -> bool:
    return _derive_ok_summary(data["summary"])


def _derive_ok_summary(summary: dict[str, Any]) -> bool:
    return bool(summary["target_reached"]) and not summary["blocked"] and not summary["contradiction"]


def _format_record(record: dict[str, Any]) -> str:
    span = record.get("span")
    location = f"line {span['start_line']}: " if span is not None else ""
    return f"{location}{record['message']}"


def _write_output(path: Path, text: str, ascii_only: bool) -> None:
    encoding = "ascii" if ascii_only else "utf-8"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text + "\n", encoding=encoding)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except BrokenPipeError:
        raise SystemExit(0)
