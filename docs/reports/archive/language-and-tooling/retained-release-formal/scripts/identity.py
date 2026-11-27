#!/usr/bin/env python3
"""identity.py BEFORE AFTER OUT.json -- RETAINED-RELEASE-OF-A-FORMAL-READ-UNKNOWN:
the byte identity of two measure.sh runs (private/rr/NAME*) on every route:
each route's resource-diagnostics.json, its null-property rows and summary,
and its typed-path parity stream, by sha256, with the custody status counts
and the null summary's verdict counts. Exits 0 when every pair is identical,
1 otherwise. Nothing here reads a verdict to change it."""
import collections
import hashlib
import json
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
PRIVATE = REPO / "private/rr"
SLICES = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")


def paths(name):
    return {
        "corpus": PRIVATE / name / "resource-diagnostics.json",
        "corpus-null": PRIVATE / f"{name}-null.jsonl",
        "corpus-null-summary": PRIVATE / f"{name}-null.summary.json",
        "corpus-parity": PRIVATE / f"{name}-parity.jsonl",
        "system": PRIVATE / f"{name}-system" / "resource-diagnostics.json",
        "system-null": PRIVATE / f"{name}-system-null.jsonl",
        "system-null-summary": PRIVATE / f"{name}-system-null.summary.json",
        "system-parity": PRIVATE / f"{name}-system-parity.jsonl",
        "slice": SLICES / f"rr-{name}" / "resource-diagnostics.json",
        "slice-null": PRIVATE / f"{name}-slice-null.jsonl",
        "slice-null-summary": PRIVATE / f"{name}-slice-null.summary.json",
        "slice-parity": PRIVATE / f"{name}-slice-parity.jsonl",
    }


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest() if path.exists() else None


def statuses(path):
    if not path.exists():
        return None
    report = json.loads(path.read_text())
    return dict(collections.Counter(
        f["status"] for unit in report["units"] for f in unit["functions"]))


def null_counts(path):
    if not path.exists():
        return None
    return json.loads(path.read_text()).get("null")


def main():
    before, after = paths(sys.argv[1]), paths(sys.argv[2])
    out, same = {}, True
    for key in before:
        b, a = digest(before[key]), digest(after[key])
        entry = {"before": b, "after": a, "identical": b is not None and b == a}
        if not key.endswith(("null", "parity", "summary")):
            entry["statuses_before"] = statuses(before[key])
            entry["statuses_after"] = statuses(after[key])
        if key.endswith("summary"):
            entry["null_before"] = null_counts(before[key])
            entry["null_after"] = null_counts(after[key])
        same &= entry["identical"]
        out[key] = entry
    out["all_identical"] = same
    pathlib.Path(sys.argv[3]).write_text(json.dumps(out, indent=1) + "\n")
    for key, entry in out.items():
        if key != "all_identical":
            print(key, "identical" if entry["identical"] else "DIFFERENT",
                  (entry["after"] or "missing")[:8])
    print("all identical" if same else "NOT identical")
    return 0 if same else 1


if __name__ == "__main__":
    sys.exit(main())
