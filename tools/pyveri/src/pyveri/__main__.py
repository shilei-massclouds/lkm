"""Command line entry point for pyveri."""

from __future__ import annotations

import argparse
import sys
from dataclasses import dataclass
from pathlib import Path

from .derive import DEFAULT_TARGET, derive, render_derivation_text, summarize_derivation
from .model import BuildResult, ObjectModel, build_model, summarize_model
from .parser import ParseError, SpecDocument, parse_file, summarize
from .view import (
    ViewModel,
    build_drives_view,
    build_object_view,
    build_timeline_view,
    render_dot,
    render_svg,
    render_text,
)


VIEW_CHOICES = ("object", "drives", "timeline")
COMMANDS = frozenset({"parse", "model", "derive", "check", "view", "render"})


@dataclass(frozen=True)
class Pipeline:
    """Loaded command pipeline state."""

    document: SpecDocument
    build: BuildResult

    @property
    def model(self) -> ObjectModel:
        return self.build.model


@dataclass(frozen=True)
class LoadFailure:
    """Failed source loading result."""

    exit_code: int


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
    parser = argparse.ArgumentParser(description="Parse and derive an LKM object model spec.")
    _add_legacy_arguments(parser)
    return parser


def _build_command_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Run pyveri pipeline stages.")

    subparsers = parser.add_subparsers(dest="command")

    parse_parser = subparsers.add_parser("parse", help="parse the spec and print an AST summary")
    parse_parser.add_argument("spec", type=Path, help="path to the .spec input file")

    model_parser = subparsers.add_parser("model", help="build the static object model")
    model_parser.add_argument("spec", type=Path, help="path to the .spec input file")

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
    derive_parser.add_argument(
        "-o",
        "--output",
        type=Path,
        help="write the derivation report to a file",
    )

    check_parser = subparsers.add_parser(
        "check", help="run static derivation and return failure for blocked targets"
    )
    check_parser.add_argument("spec", type=Path, help="path to the .spec input file")
    check_parser.add_argument(
        "--target",
        default=DEFAULT_TARGET,
        help=f"target event, default: {DEFAULT_TARGET}",
    )

    view_parser = subparsers.add_parser("view", help="print a plain text model view")
    view_parser.add_argument("spec", type=Path, help="path to the .spec input file")
    view_parser.add_argument("view", choices=VIEW_CHOICES, help="view name")
    view_parser.add_argument("-o", "--output", type=Path, help="write the view to a file")

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

    return parser


def _add_legacy_arguments(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("spec", nargs="?", type=Path, help="path to the .spec input file")
    parser.add_argument(
        "--tree",
        action="store_true",
        help="compatibility alias for --text object",
    )
    parser.add_argument(
        "--text",
        choices=VIEW_CHOICES,
        metavar="VIEW",
        help="print a plain text model view",
    )
    parser.add_argument(
        "--graph",
        choices=VIEW_CHOICES,
        metavar="VIEW",
        help="print a Graphviz DOT model view",
    )
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
    parser.add_argument(
        "-o",
        "--output",
        type=Path,
        help="write the selected text or graph view to a file",
    )


def _run_legacy(args: argparse.Namespace, parser: argparse.ArgumentParser) -> int:
    if args.spec is None:
        parser.error("the following arguments are required: spec")

    output_modes = [bool(args.tree), bool(args.text), bool(args.graph), bool(args.derive)]
    if args.output is not None:
        if not any(output_modes):
            parser.error("-o/--output requires --text, --graph, --tree, or --derive")
        if sum(output_modes) > 1:
            parser.error("-o/--output can write only one selected output")

    loaded = _load_pipeline(args.spec)
    if isinstance(loaded, LoadFailure):
        return loaded.exit_code
    pipeline = loaded
    _print_diagnostics(pipeline.build)

    selected_outputs: list[str] = []
    derivation = derive(pipeline.model, args.target) if pipeline.build.ok else None

    graph_only = (
        pipeline.build.ok
        and args.graph
        and not args.text
        and not args.tree
        and not args.derive
    )
    output_only = args.output is not None
    if not graph_only and not output_only:
        print(summarize(pipeline.document))
        print(summarize_model(pipeline.build))
        if derivation is not None and not args.derive:
            print(summarize_derivation(derivation))

    if pipeline.build.ok:
        if args.tree:
            selected_outputs.append(render_text(build_object_view(pipeline.model)))
        if args.text:
            selected_outputs.append(render_text(_build_view(pipeline.model, args.text)))
        if args.graph:
            selected_outputs.append(_render_dot(_build_view(pipeline.model, args.graph)))
        if args.derive and derivation is not None:
            selected_outputs.append(render_derivation_text(derivation))

    if args.output is not None and selected_outputs:
        _write_output(args.output, "\n\n".join(selected_outputs), ascii_only=bool(args.graph))
    else:
        _print_outputs(selected_outputs)

    if not pipeline.build.ok:
        return 1
    if args.strict and derivation is not None and not derivation.ok:
        return 1
    return 0


def _run_parse(args: argparse.Namespace) -> int:
    loaded = _load_document(args.spec)
    if isinstance(loaded, LoadFailure):
        return loaded.exit_code
    document = loaded
    print(summarize(document))
    return 0


def _run_model(args: argparse.Namespace) -> int:
    loaded = _load_pipeline(args.spec)
    if isinstance(loaded, LoadFailure):
        return loaded.exit_code
    pipeline = loaded
    _print_diagnostics(pipeline.build)
    print(summarize_model(pipeline.build))
    return 0 if pipeline.build.ok else 1


def _run_derive(args: argparse.Namespace) -> int:
    loaded = _load_pipeline(args.spec)
    if isinstance(loaded, LoadFailure):
        return loaded.exit_code
    pipeline = loaded
    _print_diagnostics(pipeline.build)
    if not pipeline.build.ok:
        return 1

    derivation = derive(pipeline.model, args.target)
    output = render_derivation_text(derivation)
    if args.output is not None:
        _write_output(args.output, output, ascii_only=False)
    else:
        print(output)
    if args.strict and not derivation.ok:
        return 1
    return 0


def _run_check(args: argparse.Namespace) -> int:
    loaded = _load_pipeline(args.spec)
    if isinstance(loaded, LoadFailure):
        return loaded.exit_code
    pipeline = loaded
    _print_diagnostics(pipeline.build)
    if not pipeline.build.ok:
        return 1

    derivation = derive(pipeline.model, args.target)
    print(summarize_derivation(derivation))
    return 0 if derivation.ok else 1


def _run_view(args: argparse.Namespace) -> int:
    loaded = _load_pipeline(args.spec)
    if isinstance(loaded, LoadFailure):
        return loaded.exit_code
    pipeline = loaded
    _print_diagnostics(pipeline.build)
    if not pipeline.build.ok:
        return 1

    output = render_text(_build_view(pipeline.model, args.view))
    if args.output is not None:
        _write_output(args.output, output, ascii_only=False)
    else:
        print(output)
    return 0


def _run_render(args: argparse.Namespace) -> int:
    loaded = _load_pipeline(args.spec)
    if isinstance(loaded, LoadFailure):
        return loaded.exit_code
    pipeline = loaded
    _print_diagnostics(pipeline.build)
    if not pipeline.build.ok:
        return 1

    view = _build_view(pipeline.model, args.view)
    try:
        output = _render_view(view, args.format)
    except ValueError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2
    if args.output is not None:
        _write_output(args.output, output, ascii_only=args.format == "dot")
    else:
        print(output)
    return 0


def _load_document(path: Path) -> SpecDocument | LoadFailure:
    try:
        return parse_file(path)
    except OSError as exc:
        print(f"error: cannot read {path}: {exc}", file=sys.stderr)
        return LoadFailure(2)
    except ParseError as exc:
        print(f"syntax_error: {exc}", file=sys.stderr)
        return LoadFailure(1)


def _load_pipeline(path: Path) -> Pipeline | LoadFailure:
    loaded = _load_document(path)
    if isinstance(loaded, LoadFailure):
        return loaded
    document = loaded
    return Pipeline(document=document, build=build_model(document))


def _print_diagnostics(result: BuildResult) -> None:
    for diagnostic in result.diagnostics:
        print(diagnostic.format(), file=sys.stderr)


def _print_outputs(outputs: list[str]) -> None:
    for output in outputs:
        print(output)


def _build_view(model: ObjectModel, name: str) -> ViewModel:
    if name == "object":
        return build_object_view(model)
    if name == "drives":
        return build_drives_view(model)
    if name == "timeline":
        return build_timeline_view(model)
    raise ValueError(f"unknown view: {name}")


def _render_view(view: ViewModel, fmt: str) -> str:
    if fmt == "svg":
        return render_svg(view)
    if fmt == "dot":
        return render_dot(view)
    raise ValueError(f"unknown render format: {fmt}")


def _render_dot(view: ViewModel) -> str:
    """Render legacy graph output, preserving timeline SVG behavior."""

    if view.graph_format == "svg":
        return render_svg(view)
    return render_dot(view)


def _write_output(path: Path, text: str, ascii_only: bool) -> None:
    encoding = "ascii" if ascii_only else "utf-8"
    path.write_text(text + "\n", encoding=encoding)


if __name__ == "__main__":
    raise SystemExit(main())
