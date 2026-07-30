"""tools2 driver: parse -> model -> derive -> check -> view -> render/animate."""

from __future__ import annotations

import argparse
from contextlib import contextmanager
import sys
import tempfile
from pathlib import Path
from typing import Iterator


def _bootstrap() -> None:
    root = Path(__file__).resolve().parents[3]
    sources = sorted(
        package / "src"
        for package in root.iterdir()
        if (package / "pyproject.toml").is_file() and (package / "src").is_dir()
    )
    sys.path[:0] = [str(source) for source in sources if str(source) not in sys.path]


_bootstrap()

from animate_tool.__main__ import main as animate_main
from check_tool.__main__ import main as check_main
from derive_tool.__main__ import main as derive_main
from derive_tool.engine import parse_budget
from model_tool.__main__ import main as model_main
from parse_tool.__main__ import main as parse_main
from render_tool.__main__ import main as render_main
from tools2_common import (
    PRODUCER,
    SNAPSHOT_SCHEMA,
    SNAPSHOT_VERSION,
    normalize_signal_request,
    read_json,
    write_json,
)
from view_tool.__main__ import main as view_main


@contextmanager
def _working_directory(path: Path | None) -> Iterator[Path]:
    if path is not None:
        path.mkdir(parents=True, exist_ok=True)
        yield path
        return
    with tempfile.TemporaryDirectory(prefix="lkm-tools2-") as temporary:
        yield Path(temporary)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Run the independent tools2 Signal pipeline.")
    parser.add_argument("spec", type=Path)
    parser.add_argument(
        "--signal",
        type=lambda value: normalize_signal_request(value, option="--signal"),
        help="one root Signal as Target.SignalName; omit to execute the model external orchestration",
    )
    parser.add_argument(
        "-u",
        "--until",
        type=lambda value: normalize_signal_request(value, option="--until"),
        help="stop immediately before sending Target.SignalName",
    )
    parser.add_argument("--source", default="Human")
    parser.add_argument("--scenario", type=Path)
    parser.add_argument("--snapshot-out", type=Path, help="write a snapshot on complete or reached")
    parser.add_argument("--max-depth", type=parse_budget, default=3, metavar="N|all")
    parser.add_argument("--max-breadth", type=parse_budget, default=3, metavar="N|all")
    parser.add_argument("--work-dir", type=Path)
    parser.add_argument("-o", "--output", type=Path, help="write rendered text")
    parser.add_argument("--html-out", type=Path, help="write a self-contained Signal animation")
    args = parser.parse_args(argv)

    with _working_directory(args.work_dir) as work:
        ast = work / "ast.json"
        model = work / "model.json"
        derivation = work / "derive.json"
        checked = work / "check.json"
        view = work / "view.json"
        rendered = work / "trace.txt"
        if parse_main([str(args.spec), "-o", str(ast)]) != 0:
            return 2
        if model_main([str(ast), "-o", str(model)]) != 0:
            return 2
        derive_args = [
            str(model),
            "--source",
            args.source,
            "--max-depth",
            "all" if args.max_depth is None else str(args.max_depth),
            "--max-breadth",
            "all" if args.max_breadth is None else str(args.max_breadth),
            "-o",
            str(derivation),
        ]
        if args.signal is not None:
            derive_args.extend(["--signal", args.signal])
        if args.scenario is not None:
            derive_args.extend(["--scenario", str(args.scenario)])
        if args.until is not None:
            derive_args.extend(["--until", args.until])
        if derive_main(derive_args) != 0:
            return 2
        check_exit = check_main([str(derivation), "-o", str(checked)])
        if check_exit not in {0, 1}:
            return 2
        if view_main([str(derivation), "-o", str(view)]) != 0:
            return 2
        if render_main([str(view), "--format", "text", "-o", str(rendered)]) != 0:
            return 2
        if args.html_out is not None:
            if animate_main([str(model), str(view), "-o", str(args.html_out)]) != 0:
                return 2
        text = rendered.read_text(encoding="utf-8")
        if args.output is None:
            sys.stdout.write(text)
        else:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(text, encoding="utf-8")
        check_data = read_json(checked)
        if check_data["allowed"] is True and args.snapshot_out is not None:
            derive_data = read_json(derivation)
            snapshot = {
                "schema": SNAPSHOT_SCHEMA,
                "version": SNAPSHOT_VERSION,
                "producer": PRODUCER,
                "source": derive_data["source"],
                "model_fingerprint": derive_data["model_fingerprint"],
                "snapshot": derive_data["last_stable_snapshot"],
                "provenance": {
                    "verdict": derive_data["verdict"],
                    "root_request": derive_data["root_request"],
                    "until_request": derive_data["until_request"],
                    "boundary": derive_data["boundary"],
                },
            }
            write_json(args.snapshot_out, snapshot)
        return check_exit


if __name__ == "__main__":
    raise SystemExit(main())
