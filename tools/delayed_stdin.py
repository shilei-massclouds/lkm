#!/usr/bin/env python3
"""Run a command, write stdin after a stdout marker, and drain output safely."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import os
import selectors
import signal
import subprocess
import sys
import time


@dataclass(frozen=True)
class InputStep:
    marker: bytes
    payload: bytes


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--timeout", required=True, help="timeout in seconds or with an s suffix")
    parser.add_argument("--ready-marker")
    parser.add_argument("--payload")
    parser.add_argument(
        "--input-step",
        nargs=2,
        action="append",
        default=[],
        metavar=("MARKER", "PAYLOAD"),
        help="ordered marker/payload pair; may be repeated",
    )
    parser.add_argument(
        "--success-marker",
        action="append",
        default=[],
        help="ordered marker that makes the command successful and terminates it; may be repeated",
    )
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args(argv)
    command = args.command
    if command and command[0] == "--":
        command = command[1:]
    if not command:
        parser.error("missing command after --")

    if args.success_marker:
        if args.ready_marker is not None or args.payload is not None or args.input_step:
            parser.error("use --success-marker without stdin marker/payload options")
        stdout, returncode, timed_out, observed_markers, terminated_after_success = (
            run_until_success_markers(
                command,
                timeout_seconds=parse_timeout(args.timeout),
                success_markers=[marker.encode() for marker in args.success_marker],
            )
        )
        sys.stdout.write(stdout)
        missing_index = first_missing_step(observed_markers)
        if terminated_after_success and missing_index is None:
            return 0
        if timed_out:
            sys.stdout.write(f"user-boot marker-only command timed out after {args.timeout}\n")
            if missing_index is not None:
                sys.stdout.write(
                    "user-boot pending success marker: "
                    f"step={missing_index + 1} marker={args.success_marker[missing_index]}\n"
                )
            return 124
        if missing_index is not None:
            sys.stdout.write(
                "user-boot success marker missing: "
                f"step={missing_index + 1} marker={args.success_marker[missing_index]}\n"
            )
            return 1 if returncode == 0 else int(returncode or 1)
        return int(returncode or 1)

    if args.input_step:
        if args.ready_marker is not None or args.payload is not None:
            parser.error("use either --input-step or --ready-marker/--payload, not both")
        step_labels = [marker for marker, _payload in args.input_step]
        stdout, returncode, timed_out, sent_steps = run_delayed_steps(
            command,
            timeout_seconds=parse_timeout(args.timeout),
            input_steps=[
                InputStep(marker=marker.encode(), payload=payload.encode())
                for marker, payload in args.input_step
            ],
        )
    else:
        if args.ready_marker is None or args.payload is None:
            parser.error("missing --ready-marker/--payload or at least one --input-step")
        step_labels = [args.ready_marker]
        stdout, returncode, timed_out, stdin_sent = run_delayed(
            command,
            timeout_seconds=parse_timeout(args.timeout),
            ready_marker=args.ready_marker.encode(),
            payload=args.payload.encode(),
        )
        sent_steps = [stdin_sent]

    sys.stdout.write(stdout)
    if timed_out:
        sys.stdout.write(f"user-boot delayed input command timed out after {args.timeout}\n")
        missing_index = first_missing_step(sent_steps)
        if missing_index is not None:
            sys.stdout.write(
                "user-boot pending input step marker: "
                f"step={missing_index + 1} marker={step_labels[missing_index]}\n"
            )
        return 124
    missing_index = first_missing_step(sent_steps)
    if missing_index is not None:
        if len(sent_steps) == 1 and not args.input_step:
            sys.stdout.write(f"user-boot input ready marker missing: {step_labels[missing_index]}\n")
        else:
            sys.stdout.write(
                "user-boot input step marker missing: "
                f"step={missing_index + 1} marker={step_labels[missing_index]}\n"
            )
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


def first_missing_step(sent_steps: list[bool]) -> int | None:
    for index, sent in enumerate(sent_steps):
        if not sent:
            return index
    return None


def run_delayed(
    command: list[str],
    *,
    timeout_seconds: float,
    ready_marker: bytes,
    payload: bytes,
) -> tuple[str, int | None, bool, bool]:
    stdout, returncode, timed_out, sent_steps = run_delayed_steps(
        command,
        timeout_seconds=timeout_seconds,
        input_steps=[InputStep(marker=ready_marker, payload=payload)],
    )
    return stdout, returncode, timed_out, sent_steps[0]


def run_delayed_steps(
    command: list[str],
    *,
    timeout_seconds: float,
    input_steps: list[InputStep],
) -> tuple[str, int | None, bool, list[bool]]:
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
    scan_buffer = b""
    sent_steps = [False for _step in input_steps]
    next_step = 0
    max_scan_buffer = max(4096, max((len(step.marker) for step in input_steps), default=0) * 2)
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
            scan_buffer += chunk
            while next_step < len(input_steps):
                step = input_steps[next_step]
                marker_index = scan_buffer.find(step.marker)
                if marker_index < 0:
                    break
                write_payload(process, step.payload)
                sent_steps[next_step] = True
                scan_buffer = scan_buffer[marker_index + len(step.marker) :]
                next_step += 1
            if len(scan_buffer) > max_scan_buffer:
                scan_buffer = scan_buffer[-max_scan_buffer:]

    try:
        rest, _ = process.communicate(timeout=3)
    except subprocess.TimeoutExpired:
        kill_process_group(process)
        rest, _ = process.communicate()
    if rest:
        stdout_parts.append(rest)
    selector.close()
    stdout = b"".join(stdout_parts).decode("utf-8", errors="replace")
    return stdout, process.returncode, timed_out, sent_steps


def run_until_success_markers(
    command: list[str],
    *,
    timeout_seconds: float,
    success_markers: list[bytes],
) -> tuple[str, int | None, bool, list[bool], bool]:
    process = subprocess.Popen(
        command,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        start_new_session=True,
    )
    assert process.stdout is not None
    selector = selectors.DefaultSelector()
    selector.register(process.stdout, selectors.EVENT_READ)
    stdout_parts: list[bytes] = []
    scan_buffer = b""
    observed_markers = [False for _marker in success_markers]
    next_marker = 0
    max_scan_buffer = max(4096, max((len(marker) for marker in success_markers), default=0) * 2)
    stdout_eof = False
    deadline = time.monotonic() + timeout_seconds
    timed_out = False
    terminated_after_success = False

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
            scan_buffer += chunk
            while next_marker < len(success_markers):
                marker = success_markers[next_marker]
                marker_index = scan_buffer.find(marker)
                if marker_index < 0:
                    break
                observed_markers[next_marker] = True
                scan_buffer = scan_buffer[marker_index + len(marker) :]
                next_marker += 1
            if next_marker == len(success_markers):
                terminated_after_success = True
                kill_process_group(process)
                break
            if len(scan_buffer) > max_scan_buffer:
                scan_buffer = scan_buffer[-max_scan_buffer:]
        if terminated_after_success:
            break

    try:
        rest, _ = process.communicate(timeout=3)
    except subprocess.TimeoutExpired:
        kill_process_group(process)
        rest, _ = process.communicate()
    if rest:
        stdout_parts.append(rest)
    selector.close()
    stdout = b"".join(stdout_parts).decode("utf-8", errors="replace")
    return stdout, process.returncode, timed_out, observed_markers, terminated_after_success


def write_payload(process: subprocess.Popen[bytes], payload: bytes) -> None:
    try:
        assert process.stdin is not None
        process.stdin.write(payload)
        process.stdin.flush()
    except BrokenPipeError:
        pass


def kill_process_group(process: subprocess.Popen[bytes]) -> None:
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass


if __name__ == "__main__":
    raise SystemExit(main())
