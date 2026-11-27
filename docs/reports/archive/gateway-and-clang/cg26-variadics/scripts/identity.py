#!/usr/bin/env python3
"""identity.py NAME_A NAME_B -- card CG26-VARIADICS (from CG05-ELABORATOR-CALL-FOOTPRINTS): compare two
measurement runs of scripts/measure.sh on all three routes (corpus, system,
tail-pause slice): digests, deduplicated totals, moved functions and
same-status functions whose leaves changed.
"""
import hashlib
import json
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
PRIVATE = REPO / "private/cg26va"
SLICES = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")


def routes(name):
    return {
        "corpus": PRIVATE / name,
        "system": PRIVATE / f"{name}-system",
        "slice": SLICES / f"cg26va-{name}",
    }


def main() -> int:
    a, b = sys.argv[1], sys.argv[2]
    record = {}
    for route, before in routes(a).items():
        after = routes(b)[route]
        compare = json.loads(subprocess.run(
            [sys.executable, str(HERE / "compare.py"), str(before), str(after)],
            check=True, capture_output=True, text=True).stdout)
        digest = lambda run: hashlib.sha256(
            (run / "resource-diagnostics.json").read_bytes()).hexdigest()
        record[route] = {
            "before": {"digest": digest(before), "totals": compare["totals"]["before"]},
            "after": {"digest": digest(after), "totals": compare["totals"]["after"]},
            "move_counts": compare["move_counts"],
            "same_status_functions_with_changed_diagnostics":
                compare["same_status_functions_with_changed_diagnostics"],
        }
    print(json.dumps(record, indent=1))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
