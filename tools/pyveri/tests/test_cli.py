from __future__ import annotations

import contextlib
import io
import tempfile
import unittest
from pathlib import Path

from pyveri.__main__ import main


class CliTests(unittest.TestCase):
    def test_graph_output_file_is_ascii_dot(self) -> None:
        spec = Path(__file__).resolve().parents[3] / "spec" / "entry-prelude-object-model.spec"

        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "object.gv"
            exit_code = main([str(spec), "--graph", "object", "-o", str(output)])

            self.assertEqual(exit_code, 0)
            data = output.read_bytes()
            self.assertTrue(data.startswith(b"digraph ObjectView"))
            data.decode("ascii")

    def test_strict_derivation_fails_for_current_blocked_spec(self) -> None:
        spec = Path(__file__).resolve().parents[3] / "spec" / "entry-prelude-object-model.spec"

        stdout = io.StringIO()
        stderr = io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            exit_code = main([str(spec), "--derive", "--strict"])

        self.assertEqual(exit_code, 1)


if __name__ == "__main__":
    unittest.main()
