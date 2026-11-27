#!/usr/bin/env python3
"""Copy a gate run's exits into the report's gate/ and print its figures.

Usage: gate_record.py LOG_DIR...

Copies every `*.exit` of each LOG_DIR (and the Clang recording, when the
records phase ran) into docs/reports/w47-loop-owner-instance-flow/gate/, and
prints the figures the logs carry: the Rust suite's totals line, the Lean
audit count, the provider test totals, the Python suite counts, and every
nonzero exit. Read-only on the logs.
"""

import pathlib
import re
import shutil
import sys

GATE = pathlib.Path(__file__).resolve().parents[1] / "gate"


def main():
    GATE.mkdir(exist_ok=True)
    for directory in map(pathlib.Path, sys.argv[1:]):
        for exit_file in sorted(directory.glob("*.exit")):
            shutil.copyfile(exit_file, GATE / exit_file.name)
            code = exit_file.read_text().strip()
            if code != "0":
                print(f"NONZERO {exit_file.name}: {code}")
        for log in sorted(directory.glob("*.log")):
            text = log.read_text(errors="replace")
            for pattern in (r"test binaries: .*", r"\d+ audits? passed.*", r"^Ran \d+ tests.*",
                            r"^(OK|FAILED).*", r"test result: .*"):
                for match in re.findall(pattern, text, re.M)[-1:]:
                    print(f"{log.name}: {match}")
        clang = directory / "12-clang.log"
        if clang.exists():
            shutil.copyfile(clang, GATE / "clang-version.txt")
            shutil.copyfile(directory / "12-clang.exit", GATE / "clang-version.exit")


if __name__ == "__main__":
    main()
