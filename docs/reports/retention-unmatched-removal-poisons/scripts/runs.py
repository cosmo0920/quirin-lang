#!/usr/bin/env python3
"""RETENTION-UNMATCHED-REMOVAL-POISONS (from FB-EVENT-LOOP-RETENTION-AND-REGISTRATION's
runs.py): artifacts/runs.json, the measured identities (binaries, the corpus
run.json binary check, report digests, row and null totals, wall times inside
the heavy lock) of the BEFORE (main export) and AFTER (tip export) runs."""
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "../../../.."))
P = os.path.join(REPO, "private/rup/")
S = "/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/"


def totals(report, null):
    out = subprocess.run([sys.executable, os.path.join(HERE, "totals.py"), report, null],
                         capture_output=True, text=True, check=True).stdout
    data = json.loads(out)
    data["report"] = data["report"].replace(REPO + "/", "").replace(
        "/home/cosmo/Documents/GitHub/qcore-c/", "")
    return data


def first_word(path):
    with open(path) as f:
        return f.read().split()[0]


def main():
    out = {
        "base": "main 3f8f1a617 (BEFORE binary built from its git archive; main moved to a423fca5d by a "
                "records-only commit, the measured code is identical)",
        "tip_measured": first_word(P + "after-src.head"),
        "binaries": {"before": first_word(P + "bin/before.sha256"), "after": first_word(P + "bin/after.sha256")},
        "corpus_run_binary_sha256": {},
    }
    for label in ("before", "after"):
        with open(P + label + "/run.json") as f:
            out["corpus_run_binary_sha256"][label] = json.load(f)["binary_sha256"]
    runs = {
        "corpus": ((P + "before", P + "before-null.jsonl", P + "before.seconds"),
                   (P + "after", P + "after-null.jsonl", P + "after.seconds")),
        "system": ((P + "before-system", P + "before-system-null.jsonl", P + "before-system.seconds"),
                   (P + "after-system", P + "after-system-null.jsonl", P + "after-system.seconds")),
        "slice": ((S + "rup-before", P + "before-slice-null.jsonl", P + "before-slice.seconds"),
                  (S + "rup-after", P + "after-slice-null.jsonl", P + "after-slice.seconds")),
    }
    for route, (before, after) in runs.items():
        out[route] = {}
        for label, (report, null, wall) in (("before", before), ("after", after)):
            out[route][label] = totals(report, null) | {"seconds": float(first_word(wall))}
    dest = os.path.join(REPO, "docs/reports/retention-unmatched-removal-poisons/artifacts/runs.json")
    with open(dest, "w") as f:
        json.dump(out, f, indent=1)
        f.write("\n")
    print(json.dumps(out, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
