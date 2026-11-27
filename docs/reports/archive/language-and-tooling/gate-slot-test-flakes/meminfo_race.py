#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Mode (c) in isolation (card GATE-SLOT-TEST-FLAKES): a reader polling a fake
meminfo with ``gate_slot.read_meminfo`` while a writer keeps changing the
reading, once with the base helper (``Path.write_text``: truncate, then write)
and once with the fixed one (write aside, then ``os.replace``).  Prints the
reads that ``read_meminfo`` refused for each.

    python3 docs/reports/gate-slot-test-flakes/meminfo_race.py [seconds]
"""
import os
from pathlib import Path
import sys
import tempfile
import threading
import time

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "scripts"))
import gate_slot  # noqa: E402

GIB_KIB = 1024 * 1024


def text(gib):
    return (f"MemTotal:       {60 * GIB_KIB} kB\nMemFree:        1 kB\n"
            f"MemAvailable:   {gib * GIB_KIB} kB\n")


def in_place(path, gib):
    path.write_text(text(gib))


def replaced(path, gib):
    aside = path.with_name("meminfo.new")
    aside.write_text(text(gib))
    os.replace(aside, path)


def trial(writer, seconds):
    with tempfile.TemporaryDirectory() as root:
        path = Path(root) / "meminfo"
        replaced(path, 1)
        stop = threading.Event()

        def write():
            n = 0
            while not stop.is_set():
                writer(path, 1 + n % 8)
                n += 1

        thread = threading.Thread(target=write)
        thread.start()
        reads = refused = 0
        limit = time.monotonic() + seconds
        while time.monotonic() < limit:
            reads += 1
            try:
                gate_slot.read_meminfo(path)
            except gate_slot.Refusal:
                refused += 1
        stop.set()
        thread.join()
        return reads, refused


def main():
    seconds = float(sys.argv[1]) if len(sys.argv) > 1 else 5.0
    for name, writer in (("write_text (base)", in_place), ("os.replace (fix)", replaced)):
        reads, refused = trial(writer, seconds)
        print(f"{name}: {refused} of {reads} reads refused ('has no MemAvailable field')")


if __name__ == "__main__":
    main()
