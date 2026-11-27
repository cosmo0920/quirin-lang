#!/usr/bin/env python3
"""runs.py OUT.json -- FB-TEARDOWN-DESTROY-DECLARATIONS: the identities of the
measured runs (binary SHA-256, report SHA-256, verdict totals, null tallies,
wall seconds inside the heavy lock). BEFORE is the clean export of main
`8b360766c` (private/ftd/src-main, `scripts/before-clean.sh`); AFTER is this
card's tree with the after1 binary."""
import collections
import hashlib
import json
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
PRIVATE = REPO / "private/ftd"
SRC = PRIVATE / "src-main/private/ftd"
SLICE = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")
RUNS = {
    "before": {
        "binary": SRC / "bin/before3",
        "corpus": (SRC / "before3c", SRC / "before3c-null.summary.json", SRC / "before3c.seconds"),
        "system": (SRC / "before3-system", SRC / "before3-system-null.summary.json",
                   SRC / "before3-system.seconds"),
        "slice": (SLICE / "ftd-before3", SRC / "before3-slice-null.summary.json",
                  SRC / "before3-slice.seconds"),
    },
    "after": {
        "binary": PRIVATE / "bin/after1",
        "corpus": (PRIVATE / "after2", PRIVATE / "after2-null.summary.json", PRIVATE / "after2.seconds"),
        "system": (PRIVATE / "after2-system", PRIVATE / "after2-system-null.summary.json",
                   PRIVATE / "after2-system.seconds"),
        "slice": (SLICE / "ftd-after2", PRIVATE / "after2-slice-null.summary.json",
                  PRIVATE / "after2-slice.seconds"),
    },
}


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    out = {}
    for run, spec in RUNS.items():
        entry = {"binary_sha256": sha(spec["binary"])}
        for route in ("corpus", "system", "slice"):
            directory, null, seconds = spec[route]
            report = directory / "resource-diagnostics.json"
            data = json.loads(report.read_text())
            totals = collections.Counter(f["status"] for u in data["units"] for f in u["functions"])
            entry[route] = {
                "report": str(report),
                "report_sha256": sha(report),
                "statuses": dict(sorted(totals.items())),
                "null": json.loads(null.read_text())["null"],
                "seconds_inside_lock": seconds.read_text().strip() if seconds.exists() else None,
            }
        out[run] = entry
    pathlib.Path(sys.argv[1]).write_text(json.dumps(out, indent=1) + "\n")
    print(json.dumps(out, indent=1))


if __name__ == "__main__":
    main()
