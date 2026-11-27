#!/usr/bin/env python3
"""identity.py BASELINE_DIR RUN_DIR [--ignore-parity-position P ...]

Compare a run against a gzipped, manifest-keyed baseline archive
(`manifest.json` maps each artifact name to its gzip file and the raw
SHA-256). Prints one line per artifact: `identical`, `differs` or `missing`.
The parity stream is compared sorted, optionally without rows whose
`position` is one of the ignored values (a census row this card adds).
Exit 0 only when every verdict-bearing artifact (resource-diagnostics.json,
the chunk contracts and the Monkey source) is byte-identical.
"""
import argparse
import gzip
import hashlib
import json
import pathlib
import sys

VERDICT_BEARING = {
    "resource-diagnostics.json",
    "resource-diagnostics.chunk-contracts.json",
    "monkey-source.json",
}


def sorted_rows(data, ignored):
    rows = []
    for line in data.splitlines():
        if not line:
            continue
        if ignored and json.loads(line).get("position") in ignored:
            continue
        rows.append(line)
    return sorted(rows)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("baseline", type=pathlib.Path)
    parser.add_argument("run", type=pathlib.Path)
    parser.add_argument("--ignore-parity-position", action="append", default=[])
    args = parser.parse_args()
    manifest = json.loads((args.baseline / "manifest.json").read_text())
    ok = True
    report = {}
    for name, entry in sorted(manifest.items()):
        mine = args.run / name
        if not mine.exists():
            report[name] = "missing"
            ok = ok and name not in VERDICT_BEARING
            continue
        data = mine.read_bytes()
        if hashlib.sha256(data).hexdigest() == entry["sha256"]:
            report[name] = "identical"
            continue
        if name.startswith("parity.jsonl"):
            base = gzip.open(args.baseline / entry["file"]).read().decode()
            same = sorted_rows(base, set()) == sorted_rows(
                data.decode(), set(args.ignore_parity_position)
            )
            report[name] = "identical-sorted" if same else "differs"
            continue
        report[name] = "differs"
        if name in VERDICT_BEARING:
            ok = False
    print(json.dumps({"verdict_bearing_identical": ok, "artifacts": report}, indent=2))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
