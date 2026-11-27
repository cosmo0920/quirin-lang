#!/usr/bin/env python3
"""Archive CONTRACT-INHERIT's arbiter records beside the report.

Usage: archive_arbiter.py BEFORE_RUN AFTER_RUN IDENTITY_JSON NONVERDICT_JSON

Copies both runs' `run.json`, the AFTER-vs-W43 identity record and the
non-verdict attribution into `arbiter/`, and writes `before-vs-after.json`:
the raw SHA-256 of each of the five artifacts in both same-checkout runs.
"""
import hashlib
import json
import pathlib
import sys

REPORT = pathlib.Path(__file__).resolve().parents[1]
NAMES = [
    "resource-diagnostics.json",
    "resource-diagnostics.chunk-contracts.json",
    "resource-diagnostics.contracts.json",
    "resource-diagnostics.source-effects.json",
    "monkey-source.json",
]


def main() -> int:
    before, after = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    out = REPORT / "arbiter"
    out.mkdir(exist_ok=True)
    (out / "before-run.json").write_bytes((before / "run.json").read_bytes())
    (out / "after-run.json").write_bytes((after / "run.json").read_bytes())
    (out / "after-vs-w43.json").write_bytes(pathlib.Path(sys.argv[3]).read_bytes())
    (out / "nonverdict-parity.json").write_bytes(pathlib.Path(sys.argv[4]).read_bytes())
    rows = {}
    for name in NAMES:
        b = hashlib.sha256((before / name).read_bytes()).hexdigest()
        a = hashlib.sha256((after / name).read_bytes()).hexdigest()
        rows[name] = {"before_sha256": b, "after_sha256": a, "byte_identical": a == b}
    doc = {
        "before_commit": json.loads((before / "run.json").read_text())["source_commit"],
        "after_commit": json.loads((after / "run.json").read_text())["source_commit"],
        "checkout": "same worktree, same pinned inputs, fresh release builds",
        "artifacts": rows,
        "all_identical": all(row["byte_identical"] for row in rows.values()),
    }
    (out / "before-vs-after.json").write_text(json.dumps(doc, indent=2) + "\n")
    print(json.dumps(doc, indent=2))
    return 0 if doc["all_identical"] else 1


if __name__ == "__main__":
    sys.exit(main())
