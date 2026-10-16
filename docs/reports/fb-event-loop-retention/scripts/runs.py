#!/usr/bin/env python3
"""FB-EVENT-LOOP-RETENTION-AND-REGISTRATION: artifacts/runs.json, the measured
identities (binaries, report digests, row and null totals, wall times inside
the heavy lock) of the BEFORE (base export) and AFTER (tip export) runs."""
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "../../../.."))
B = os.path.join(REPO, "private/fbel/base-src/private/fbel/")
A = os.path.join(REPO, "private/fbel/after-src/private/fbel/")
S = "/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/"


def totals(report, null):
    out = subprocess.run([sys.executable, os.path.join(HERE, "totals.py"), report, null],
                         capture_output=True, text=True, check=True).stdout
    data = json.loads(out)
    data["report"] = os.path.relpath(data["report"], REPO) if data["report"].startswith(REPO) else \
        data["report"].replace("/home/cosmo/Documents/GitHub/qcore-c/", "")
    return data


def seconds(path):
    with open(path) as f:
        return float(f.read())


def sha(path):
    with open(path) as f:
        return f.read().split()[0]


def main():
    out = {
        "base": "8b360766c (BEFORE binary built from the git archive of 6590dca2f, code identical to main)",
        "tip_measured": open(os.path.join(REPO, "private/fbel/after-src.head")).read().strip(),
        "binaries": {"before": sha(B + "bin/before.sha256"), "after": sha(A + "bin/after.sha256")},
        "note": ("The first BEFORE corpus run (`before`) exited 1 after 763 s: the exported tree's "
                 "third_party/libevent had no .git (`Pin(GitUnavailable)`); kept as a failed attempt, not "
                 "evidence. `before2` reran it with a git clone of the pinned submodule (d82464a2)."),
    }
    runs = {
        "corpus": ((B + "before2", B + "before2-null.jsonl", B + "before2.seconds"),
                   (A + "after", A + "after-null.jsonl", A + "after.seconds")),
        "system": ((B + "before-system", B + "before-system-null.jsonl", B + "before-system.seconds"),
                   (A + "after-system", A + "after-system-null.jsonl", A + "after-system.seconds")),
        "slice": ((S + "fbel-before", B + "before-slice-null.jsonl", B + "before-slice.seconds"),
                  (S + "fbel-after", A + "after-slice-null.jsonl", A + "after-slice.seconds")),
    }
    for route, (before, after) in runs.items():
        out[route] = {}
        for label, (report, null, wall) in (("before", before), ("after", after)):
            out[route][label] = totals(report, null) | {"seconds": seconds(wall)}
    dest = os.path.join(REPO, "docs/reports/fb-event-loop-retention/artifacts/runs.json")
    with open(dest, "w") as f:
        json.dump(out, f, indent=1)
        f.write("\n")
    print(json.dumps(out, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
