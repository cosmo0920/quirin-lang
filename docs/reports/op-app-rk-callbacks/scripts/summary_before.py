#!/usr/bin/env python3
"""summary_before.py -- OP-APP-RK-CALLBACKS-VIA-POLL-CB: the BEFORE run's
figures on every route (status counts, report sha256, null-property tallies,
wall seconds, the binary each run names), written to
artifacts/summary-before.json and printed."""
import collections
import hashlib
import json
import pathlib

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
PRIVATE = REPO / "private/oprk"
SLICE = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/oprk-before")


def main():
    out = {"binary": (PRIVATE / "bin/before.sha256").read_text().split()[0],
           "base": (PRIVATE / "build-before.head").read_text().strip()}
    for route, path, suffix in [("corpus", PRIVATE / "before", ""),
                                ("system", PRIVATE / "before-system", "-system"),
                                ("slice", SLICE, "-slice")]:
        data = (path / "resource-diagnostics.json").read_bytes()
        report = json.loads(data)
        counts = collections.Counter(f["status"] for u in report["units"] for f in u["functions"])
        entry = {
            "status": dict(counts),
            "report_sha256": hashlib.sha256(data).hexdigest(),
            "null": json.loads((PRIVATE / f"before{suffix}-null.summary.json").read_text())["null"],
            "seconds": float((PRIVATE / f"before{suffix}.seconds").read_text()),
        }
        run = path / "run.json"
        if run.exists():
            entry["run_binary_sha256"] = json.loads(run.read_text()).get("binary_sha256")
        out[route] = entry
    (HERE.parent / "artifacts/summary-before.json").write_text(json.dumps(out, indent=1) + "\n")
    print(json.dumps(out, indent=1))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
