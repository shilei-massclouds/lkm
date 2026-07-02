#!/usr/bin/env python3
"""Run a command, write stdin after a stdout marker, and drain output safely."""

from __future__ import annotations

import argparse
import os
import selectors
import signal
import subprocess
import sys
import time


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--timeout", required=True, help="timeout in seconds or with an s suffix")
    parser.add_argument("--ready-marker", required=True)
    parser.add_argument("--payload", required=True)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args(argv)
    command = args.command
    if command and command[0] == "--":
        command = command[1:]
    if not command:
        parser.error("missing command after --")

    stdout, returncode, timed_out, stdin_sent = run_delayed(
        command,
        timeout_seconds=parse_timeout(args.timeout),
        ready_marker=args.ready_marker.encode(),
        payload=args.payload.encode(),
    )
    sys.stdout.write(stdout)
    if timed_out:
        sys.stdout.write(f"user-boot delayed input command timed out after {args.timeout}\n")
        return 124
    if not stdin_sent:
        sys.stdout.write(f"user-boot input ready marker missing: {args.ready_marker}\n")
        return 1 if returncode == 0 else int(returncode or 1)
    if returncode is None:
        return 1
    if returncode != 0:
        sys.stdout.write(f"user-boot delayed input command rc: {returncode}\n")
    return int(returncode)


def parse_timeout(raw: str) -> float:
    value = raw[:-1] if raw.endswith("s") else raw
    timeout = float(value)
    if timeout <= 0:
        raise argparse.ArgumentTypeError("timeout must be positive")
    return timeout


def run_delayed(
    command: list[str],
    *,
    timeout_seconds: float,
    ready_marker: bytes,
    payload: bytes,
) -> tuple[str, int | None, bool, bool]:
    process = subprocess.Popen(
        command,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        start_new_session=True,
    )
    assert process.stdout is not None
    selector = selectors.DefaultSelector()
    selector.register(process.stdout, selectors.EVENT_READ)
    stdout_parts: list[bytes] = []
    tail = b""
    stdin_sent = False
    stdout_eof = False
    deadline = time.monotonic() + timeout_seconds
    timed_out = False

    while True:
        now = time.monotonic()
        if now >= deadline:
            timed_out = True
            kill_process_group(process)
            break
        if stdout_eof and process.poll() is not None:
            break

        wait_time = min(0.25, max(0.0, deadline - now))
        events = selector.select(wait_time)
        if not events:
            continue
        for key, _ in events:
            chunk = os.read(key.fd, 4096)
            if not chunk:
                stdout_eof = True
                continue
            stdout_parts.append(chunk)
            tail = (tail + chunk)[-4096:]
            if not stdin_sent and ready_marker in tail:
                try:
                    assert process.stdin is not None
                    process.stdin.write(payload)
                    process.stdin.flush()
                except BrokenPipeError:
                    pass
                stdin_sent = True

    try:
        rest, _ = process.communicate(timeout=3)
    except subprocess.TimeoutExpired:
        kill_process_group(process)
        rest, _ = process.communicate()
    if rest:
        stdout_parts.append(rest)
    selector.close()
    stdout = b"".join(stdout_parts).decode("utf-8", errors="replace")
    return stdout, process.returncode, timed_out, stdin_sent


def kill_process_group(process: subprocess.Popen[bytes]) -> None:
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass


if __name__ == "__main__":
    raise SystemExit(main())
