#!/usr/bin/env python3
"""Summarise the stress sets (card PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE).

For every private/pfw/stress/*.json written by loadbisect.py, print the binary
digest, the runs, the deviating runs, how many of those logs name ETXTBSY
("Text file busy" / os error 26) and the failing tests with their first panic
line. Read-only; prints Markdown to stdout.
"""

from __future__ import annotations

import json
import sys
from collections import Counter
from pathlib import Path

ETXTBSY = ("Text file busy", "os error 26")


def main() -> int:
    root = Path(sys.argv[1] if len(sys.argv) > 1 else "private/pfw/stress")
    print("| Set | Binary sha256 | Amplifier | Runs | Deviating | With ETXTBSY | Child stalls |")
    print("|---|---|---|---:|---:|---:|---:|")
    details = []
    for path in sorted(root.glob("*.json")):
        data = json.loads(path.read_text())
        env = dict(item.split("=", 1) for item in data["env"])
        for level in data["levels"]:
            runs = level["runs"]
            bad = [run for run in runs if run["exit"] != 0]
            busy = 0
            tests: Counter[str] = Counter()
            other = []
            for run in bad:
                text = Path(run["log"]).read_text(errors="replace")
                if any(marker in text for marker in ETXTBSY):
                    busy += 1
                else:
                    other.append(Path(run["log"]).name)
                for failure in run["failures"]:
                    tests[failure["test"]] += 1
            amplifier = (
                f"close {int(env.get('QCORE_AMPLIFY_CLOSE_US', '0')) // 1000} ms, "
                f"child stall {level['child_stall_ms']} ms"
            )
            print(
                f"| {path.stem} | `{data['binary_sha256'][:12]}…` | {amplifier} | "
                f"{len(runs)} | {len(bad)} | {busy} | "
                f"{sum(run['child_stalls'] for run in runs)} |"
            )
            if bad:
                details.append((path.stem, tests, other))
    for name, tests, other in details:
        print(f"\n{name}: failing tests {dict(tests)}")
        if other:
            print(f"  deviating logs without ETXTBSY: {other}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
