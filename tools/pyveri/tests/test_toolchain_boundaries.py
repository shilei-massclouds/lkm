from __future__ import annotations

import unittest
from pathlib import Path


class ToolchainBoundaryTests(unittest.TestCase):
    def test_independent_tool_sources_do_not_import_pyveri(self) -> None:
        tools_root = Path(__file__).resolve().parents[2]
        source_roots = [
            tools_root / "common" / "src",
            tools_root / "parse" / "src",
            tools_root / "model" / "src",
            tools_root / "derive" / "src",
            tools_root / "check" / "src",
            tools_root / "view" / "src",
            tools_root / "render" / "src",
        ]

        for source_root in source_roots:
            for path in source_root.rglob("*.py"):
                text = path.read_text(encoding="utf-8")
                self.assertNotIn("from pyveri", text, str(path))
                self.assertNotIn("import pyveri", text, str(path))


if __name__ == "__main__":
    unittest.main()
