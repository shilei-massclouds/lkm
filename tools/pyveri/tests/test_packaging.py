from __future__ import annotations

import unittest
from pathlib import Path


class PackagingTests(unittest.TestCase):
    def test_console_script_is_declared(self) -> None:
        pyproject = Path(__file__).resolve().parents[1] / "pyproject.toml"

        text = pyproject.read_text(encoding="utf-8")

        self.assertIn("[project.scripts]", text)
        self.assertIn('pyveri = "pyveri.__main__:main"', text)


if __name__ == "__main__":
    unittest.main()
