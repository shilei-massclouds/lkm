#!/usr/bin/env python3
"""Require the BootInitFlow Prepared -> Setup -> Enable runtime order."""

from __future__ import annotations

import os
from pathlib import Path


MARKERS = [
    "checkpoint: BootInitFlow.Prepared",
    "checkpoint: DmaCachePolicy.Ready",
    "checkpoint: CorePreparePhase.Started",
    "checkpoint: CorePreparePhase.Online",
    "checkpoint: MmCoreInitPhase.Online",
    "checkpoint: SchedInitPhase.Online",
    "checkpoint: IrqTimeInitPhase.Online",
    "checkpoint: LocalIrqEnablePhase.Online",
    "checkpoint: IrqOpenPreparePhase.Online",
    "checkpoint: ProcessPreparePhase.Online",
    "checkpoint: BootInitRestInitPhase.Online",
    "checkpoint: BootInitFlow.Ready",
    "checkpoint: BootInitScheduleHandoffPhase.Online",
    "checkpoint: BootInitFlow.Online",
    "checkpoint: Kernel.Online",
]


def main() -> int:
    log_path = Path(os.environ["LKM_TEST_QEMU_LOG"])
    text = log_path.read_text(errors="replace")
    early = "RGTDOI"
    if text.count(early) != 1:
        raise SystemExit(
            f"early Kernel.Enable markers must contain exactly one {early!r} sequence"
        )
    positions: list[int] = []
    for marker in MARKERS:
        count = text.count(marker)
        if count != 1:
            raise SystemExit(f"{marker!r} count is {count}, expected exactly 1")
        positions.append(text.index(marker))
    if any(left >= right for left, right in zip(positions, positions[1:])):
        observed = "\n".join(
            f"0x{position:x} {marker}" for position, marker in zip(positions, MARKERS)
        )
        raise SystemExit(f"BootInitFlow runtime markers are out of order:\n{observed}")
    print(
        "BootInitFlow runtime order verified: Prepared precedes direct setup_arch systems, "
        "CorePrepare and all later leaves, and Kernel.Online; early order is PhysicalDirect -> Started -> InterruptType"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
