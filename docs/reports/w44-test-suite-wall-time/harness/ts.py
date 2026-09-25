#!/usr/bin/env python3
"""Prefix each stdin line with the seconds elapsed since start (monotonic).

Measurement helper for card W44-TEST-SUITE-WALL-TIME: the merged output of a
plain `cargo test` run is piped through this so the per-binary durations can be
recovered from the gaps between cargo's `Running ...` lines.
"""

import sys
import time


def main() -> None:
    t0 = time.monotonic()
    out = sys.stdout
    for raw in sys.stdin.buffer:
        out.write("%10.3f " % (time.monotonic() - t0))
        out.write(raw.decode("utf-8", "replace"))
        out.flush()


if __name__ == "__main__":
    main()
