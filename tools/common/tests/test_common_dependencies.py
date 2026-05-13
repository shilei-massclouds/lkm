from __future__ import annotations

import unittest
from pathlib import Path


class CommonDependencyTests(unittest.TestCase):
    def test_model_contract_modules_do_not_import_pyveri(self) -> None:
        source_root = Path(__file__).resolve().parents[1] / "src" / "common"
        paths = [
            source_root / "spec_ast.py",
            source_root / "model_types.py",
            source_root / "model_json.py",
            source_root / "derive_types.py",
            source_root / "defaults.py",
        ]

        for path in paths:
            text = path.read_text(encoding="utf-8")
            self.assertNotIn("from pyveri", text, str(path))
            self.assertNotIn("import pyveri", text, str(path))


if __name__ == "__main__":
    unittest.main()
