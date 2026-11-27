#!/usr/bin/env python3
"""identity.py NAME_A NAME_B OUT_JSON -- W48-CALLEE-CLOSURE-R3: compare two
scripts/measure.sh runs on all three routes (corpus, system, tail-pause
slice): raw digests, the digest with this worktree's path replaced by
`WORKTREE` (the system and slice reports embed the profile directory), the
deduplicated totals, every status move and the count of same-status
functions whose leaves changed (scripts/compare.py).
"""
import hashlib
import json
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
PRIVATE = REPO / "private/w48cc"
SLICES = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")


def routes(name):
    return {"corpus": PRIVATE / name, "system": PRIVATE / f"{name}-system",
            "slice": SLICES / f"w48cc-{name}"}


def digests(run):
    raw = (run / "resource-diagnostics.json").read_bytes()
    normal = raw.replace(str(REPO).encode(), b"WORKTREE")
    return hashlib.sha256(raw).hexdigest(), hashlib.sha256(normal).hexdigest()


def main() -> int:
    a, b, out = sys.argv[1], sys.argv[2], pathlib.Path(sys.argv[3])
    record = {}
    for route, before in routes(a).items():
        after = routes(b)[route]
        compare = json.loads(subprocess.run(
            [sys.executable, str(HERE / "compare.py"), str(before), str(after)],
            check=True, capture_output=True, text=True).stdout)
        record[route] = {
            "before": dict(zip(("digest", "normalised"), digests(before)),
                           totals=compare["totals"]["before"]),
            "after": dict(zip(("digest", "normalised"), digests(after)),
                          totals=compare["totals"]["after"]),
            "moves": compare["moves"],
            "move_counts": compare["move_counts"],
            "same_status_functions_with_changed_diagnostics":
                compare["same_status_functions_with_changed_diagnostics"],
            "leaf_delta": compare["leaf_delta"],
        }
    out.write_text(json.dumps(record, indent=1, ensure_ascii=False) + "\n")
    for route, value in record.items():
        same = value["before"]["normalised"] == value["after"]["normalised"]
        print(f"{route}: {value['before']['totals']} -> {value['after']['totals']} "
              f"moves {value['move_counts']} leaves-changed "
              f"{value['same_status_functions_with_changed_diagnostics']} identical={same}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
