#!/usr/bin/env python3
"""summary.py BEFORE AFTER OUT.json -- LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER:
per route (corpus, system, slice) the custody status totals and report
digest, the null summary's counts, the null rows' digest, the measured
binary (run.json for the corpus) and the wall time measured inside the shared
lock, for both runs. Writes OUT.json and prints it."""
import collections
import hashlib
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
P = HERE.parents[3] / "private/lppr"
S = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def route(name, kind):
    suffix = {"corpus": "", "system": "-system", "slice": "-slice"}[kind]
    report = {"corpus": P / name, "system": P / f"{name}-system", "slice": S / f"lppr-{name}"}[kind]
    report = report / "resource-diagnostics.json"
    rows = json.loads(report.read_text())
    status = collections.Counter(f["status"] for u in rows["units"] for f in u["functions"])
    null = json.loads((P / f"{name}{suffix}-null.summary.json").read_text())
    out = {
        "custody": dict(sorted(status.items())),
        "report_sha256": sha(report),
        "null_rows_sha256": sha(P / f"{name}{suffix}-null.jsonl"),
        "null_summary": {"functions": null.get("functions"), "null": null.get("null")},
        "seconds": float((P / f"{name}{suffix}.seconds").read_text()),
        "exit": int((P / f"{name}{suffix}.exit").read_text()),
    }
    if kind == "corpus":
        out["run_binary_sha256"] = json.loads((report.parent / "run.json").read_text())["binary_sha256"]
    return out


def main():
    before, after, target = sys.argv[1], sys.argv[2], pathlib.Path(sys.argv[3])
    result = {name: {kind: route(name, kind) for kind in ("corpus", "system", "slice")}
              for name in (before, after)}
    target.write_text(json.dumps(result, indent=1) + "\n")
    print(json.dumps(result, indent=1))


main()
