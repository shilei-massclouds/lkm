"""tools2 test suite with repository-relative source package discovery."""

from pathlib import Path
import sys


TOOLS2_ROOT = Path(__file__).resolve().parents[1]


def _bootstrap_source_packages() -> None:
    sources = sorted(
        package / "src"
        for package in TOOLS2_ROOT.iterdir()
        if (package / "pyproject.toml").is_file() and (package / "src").is_dir()
    )
    sys.path[:0] = [str(source) for source in sources if str(source) not in sys.path]


_bootstrap_source_packages()
