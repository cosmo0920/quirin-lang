#!/usr/bin/env python3
"""Card FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS: the identity and totals of one
measurement NAME (scripts/measure.sh): per route the exit, wall seconds, the
report's sha256, the custody counts and the null property's counts, read from
the `*-null.summary.json` the null property writes beside its stream.

usage: runs.py NAME [SLICE_RUN_DIR] > artifacts/runs-NAME.json
"""
import hashlib
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
OUT = ROOT / "private" / "fblb"
SLICE = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read(path):
    return path.read_text().strip() if path.exists() else None


def route(name, suffix, report):
    summary = OUT / f"{name}{suffix}-null.summary.json"
    data = json.loads(summary.read_text()) if summary.exists() else {}
    seconds = read(OUT / f"{name}{suffix}.seconds")
    return {
        "exit": read(OUT / f"{name}{suffix}.exit"),
        "seconds": float(seconds) if seconds else None,
        "report": str(report.relative_to(ROOT)) if report.is_relative_to(ROOT) else str(report),
        "report_sha256": sha(report) if report.exists() else None,
        "custody": data.get("custody"),
        "null": data.get("null"),
        "null_stream_sha256": sha(OUT / f"{name}{suffix}-null.jsonl")
        if (OUT / f"{name}{suffix}-null.jsonl").exists() else None,
    }


def main():
    name = sys.argv[1]
    binary = read(OUT / f"{name}.binary.sha256")
    run = OUT / name / "run.json"
    result = {
        "card": "FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS",
        "name": name,
        "binary_sha256": binary.split()[0] if binary else None,
        "corpus_run_json_binary_sha256": json.loads(run.read_text())["binary_sha256"]
        if run.exists() else None,
        "routes": {
            "corpus": route(name, "", OUT / name / "resource-diagnostics.json"),
            "system": route(name, "-system", OUT / f"{name}-system" / "resource-diagnostics.json"),
            "slice": route(name, "-slice", SLICE / f"fblb-{name}" / "resource-diagnostics.json"),
        },
    }
    json.dump(result, sys.stdout, indent=1)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
