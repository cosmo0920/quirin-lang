#!/usr/bin/env python3
"""summarize_runs.py OUT.json NAME=DIR [NAME=DIR ...] -- SUPPLIED-OWNED-RETURN-CODE-HANDOFF (from RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION) (adapted from OP-APP-RK-CALLBACKS-VIA-POLL-CB):
for each named run (DIR holds NAME's corpus and system directories and
their null summaries, as measure.sh writes them; the slice is the shared
prepared attempt's `rkou-NAME`): status counts, report sha256, null tallies,
wall seconds, the binary each run names, and, for the corpus, the member
fold's section (run closures and the pre-pass evidence). Writes OUT.json and
prints it."""
import collections
import hashlib
import json
import pathlib
import sys

SLICE = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")


def route(name, base, suffix, path):
    data = (path / "resource-diagnostics.json").read_bytes()
    report = json.loads(data)
    counts = collections.Counter(f["status"] for u in report["units"] for f in u["functions"])
    entry = {
        "status": dict(counts),
        "report_sha256": hashlib.sha256(data).hexdigest(),
        "null": json.loads((base / f"{name}{suffix}-null.summary.json").read_text())["null"],
        "seconds": float((base / f"{name}{suffix}.seconds").read_text()),
    }
    run = path / "run.json"
    if run.exists():
        entry["run_binary_sha256"] = json.loads(run.read_text()).get("binary_sha256")
    return entry


def main():
    out = {}
    for argument in sys.argv[2:]:
        name, directory = argument.split("=", 1)
        base = pathlib.Path(directory)
        record = {"binary": (base / "bin" / f"{name}.sha256").read_text().split()[0]
                  if (base / "bin" / f"{name}.sha256").exists() else None}
        record["corpus"] = route(name, base, "", base / name)
        record["system"] = route(name, base, "-system", base / f"{name}-system")
        record["slice"] = route(name, base, "-slice", SLICE / f"sorch-{name}")
        sidecar = json.loads((base / name / "resource-diagnostics.contracts.json").read_text())
        members = sidecar.get("callback_members") or {}
        record["callback_members"] = {
            "members": members.get("members"),
            "demoted": len(members.get("demoted") or []),
            "prepass": {key: value for key, value in (members.get("prepass") or {}).items()
                        if key not in ("tokens",)},
            "prepass_tokens": (members.get("prepass") or {}).get("tokens"),
        }
        out[name] = record
    pathlib.Path(sys.argv[1]).write_text(json.dumps(out, indent=1) + "\n")
    print(json.dumps({name: {r: rec[r]["status"] for r in ("corpus", "system", "slice")}
                      for name, rec in out.items()}, indent=1))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
