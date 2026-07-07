from __future__ import annotations

import contextlib
import importlib.util
import io
from pathlib import Path
import sys
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "delayed_stdin.py"
SPEC = importlib.util.spec_from_file_location("delayed_stdin", SCRIPT)
assert SPEC is not None
delayed_stdin = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = delayed_stdin
assert SPEC.loader is not None
SPEC.loader.exec_module(delayed_stdin)


class DelayedStdinTest(unittest.TestCase):
    def test_legacy_ready_marker_payload_cli_still_writes_after_marker(self) -> None:
        script = (
            "import sys\n"
            "print('ready', flush=True)\n"
            "line = sys.stdin.readline().strip()\n"
            "print('got=' + line, flush=True)\n"
        )
        stdout = io.StringIO()
        with contextlib.redirect_stdout(stdout):
            rc = delayed_stdin.main(
                [
                    "--timeout",
                    "5",
                    "--ready-marker",
                    "ready",
                    "--payload",
                    "legacy\n",
                    "--",
                    sys.executable,
                    "-c",
                    script,
                ]
            )

        self.assertEqual(rc, 0)
        self.assertIn("got=legacy", stdout.getvalue())

    def test_input_steps_write_payloads_in_marker_order(self) -> None:
        script = (
            "import sys\n"
            "print('login:', flush=True)\n"
            "user = sys.stdin.readline().strip()\n"
            "print('user=' + user, flush=True)\n"
            "print('Password:', flush=True)\n"
            "password = sys.stdin.readline().strip()\n"
            "print('password=' + password, flush=True)\n"
            "print('/ $', flush=True)\n"
            "cmd = sys.stdin.readline().strip()\n"
            "exit_cmd = sys.stdin.readline().strip()\n"
            "print('cmd=' + cmd, flush=True)\n"
            "print('exit=' + exit_cmd, flush=True)\n"
        )

        stdout, returncode, timed_out, sent_steps = delayed_stdin.run_delayed_steps(
            [sys.executable, "-c", script],
            timeout_seconds=5,
            input_steps=[
                delayed_stdin.InputStep(b"login:", b"test\n"),
                delayed_stdin.InputStep(b"Password:", b"\n"),
                delayed_stdin.InputStep(b"/ $", b"/bin/ls\nexit\n"),
            ],
        )

        self.assertEqual(returncode, 0)
        self.assertFalse(timed_out)
        self.assertEqual(sent_steps, [True, True, True])
        self.assertIn("user=test", stdout)
        self.assertIn("cmd=/bin/ls", stdout)
        self.assertIn("exit=exit", stdout)

    def test_missing_later_marker_reports_unsent_step(self) -> None:
        script = (
            "import sys\n"
            "print('login:', flush=True)\n"
            "sys.stdin.readline()\n"
            "print('done', flush=True)\n"
        )

        stdout, returncode, timed_out, sent_steps = delayed_stdin.run_delayed_steps(
            [sys.executable, "-c", script],
            timeout_seconds=5,
            input_steps=[
                delayed_stdin.InputStep(b"login:", b"test\n"),
                delayed_stdin.InputStep(b"Password:", b"\n"),
            ],
        )

        self.assertEqual(returncode, 0)
        self.assertFalse(timed_out)
        self.assertEqual(sent_steps, [True, False])
        self.assertEqual(delayed_stdin.first_missing_step(sent_steps), 1)
        self.assertIn("done", stdout)

    def test_timeout_keeps_unsent_step_diagnostic_state(self) -> None:
        script = "import time\ntime.sleep(10)\n"

        _stdout, _returncode, timed_out, sent_steps = delayed_stdin.run_delayed_steps(
            [sys.executable, "-c", script],
            timeout_seconds=0.2,
            input_steps=[delayed_stdin.InputStep(b"never", b"payload\n")],
        )

        self.assertTrue(timed_out)
        self.assertEqual(sent_steps, [False])

    def test_repeated_marker_sends_step_only_once(self) -> None:
        script = (
            "import os, select, sys\n"
            "print('READY', flush=True)\n"
            "print('READY', flush=True)\n"
            "line = sys.stdin.readline().strip()\n"
            "ready, _, _ = select.select([sys.stdin.buffer], [], [], 0.2)\n"
            "extra = os.read(0, 100).decode() if ready else ''\n"
            "print(f'line={line} extra={extra!r}', flush=True)\n"
        )

        stdout, returncode, timed_out, sent_steps = delayed_stdin.run_delayed_steps(
            [sys.executable, "-c", script],
            timeout_seconds=5,
            input_steps=[delayed_stdin.InputStep(b"READY", b"one\n")],
        )

        self.assertEqual(returncode, 0)
        self.assertFalse(timed_out)
        self.assertEqual(sent_steps, [True])
        self.assertIn("line=one extra=''", stdout)


if __name__ == "__main__":
    unittest.main()
