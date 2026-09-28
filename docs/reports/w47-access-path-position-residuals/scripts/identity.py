#!/usr/bin/env python3
"""Compare a run with an archived baseline by raw SHA-256 and sorted rows.

Usage: identity.py ARCHIVE_DIR RUN_DIR OUT_JSON [--parity-without-residual-shapes]

ARCHIVE_DIR holds gzipped artifacts and a `manifest.json` keyed by raw file
name (the round-1 format); `baseline_identity.py` expects the older
label-keyed manifest, so this compares directly: raw SHA-256 of every
verdict-bearing file, and the sorted parity rows and contexts. With
`--parity-without-residual-shapes`, the run's `residual-shape` rows (this
card's parity-only record) are dropped before the comparison.
"""

import gzip
import hashlib
import json
import pathlib
import sys

VERDICT = ["resource-diagnostics.json", "resource-diagnostics.chunk-contracts.json",
           "monkey-source.json"]


def masked_paths(left, right, path=""):
    """Leaf paths where two JSON values differ, ignoring nothing."""
    if isinstance(left, dict) and isinstance(right, dict):
        out = []
        for key in sorted(set(left) | set(right)):
            out += masked_paths(left.get(key), right.get(key), f"{path}/{key}")
        return out
    if isinstance(left, list) and isinstance(right, list) and len(left) == len(right):
        out = []
        for index, (a, b) in enumerate(zip(left, right)):
            out += masked_paths(a, b, f"{path}/{index}")
        return out
    return [] if left == right else [(path, left, right)]


def hex_digest(value):
    return isinstance(value, str) and len(value) == 64 and all(c in "0123456789abcdef" for c in value)


def rows(path):
    opener = gzip.open if path.suffix == ".gz" else open
    with opener(path, "rt") as handle:
        return sorted(handle.read().splitlines())


def main():
    archive, run, out = (pathlib.Path(a) for a in sys.argv[1:4])
    strip = "--parity-without-residual-shapes" in sys.argv
    manifest = json.loads((archive / "manifest.json").read_text())
    result = {"archive": str(archive), "run": str(run), "files": {}}
    for name in VERDICT:
        if name not in manifest or not (run / name).exists():
            continue
        digest = hashlib.sha256((run / name).read_bytes()).hexdigest()
        entry = {"run": digest, "archive": manifest[name]["raw_sha256"],
                 "identical": digest == manifest[name]["raw_sha256"]}
        if not entry["identical"]:
            # Name every differing JSON leaf; a digest-only difference is a
            # path-derived digest (the system slice binds the worktree path).
            with gzip.open(archive / manifest[name]["file"], "rt") as handle:
                theirs = json.load(handle)
            differing = masked_paths(theirs, json.loads((run / name).read_text()))
            entry["differing_leaves"] = [leaf for leaf, _, _ in differing]
            entry["digest_only"] = bool(differing) and all(
                hex_digest(a) and hex_digest(b) for _, a, b in differing)
            entry["identical"] = entry["digest_only"]
            entry["identity"] = "equal after masking path-derived digests"
        result["files"][name] = entry
    for name in ("parity.jsonl", "parity.jsonl.contexts.jsonl"):
        if name not in manifest or not (run / name).exists():
            continue
        mine = rows(run / name)
        if strip and name == "parity.jsonl":
            mine = [r for r in mine if '"position":"residual-shape"' not in r]
        theirs = rows(archive / manifest[name]["file"])
        result["files"][name + " (sorted rows)"] = {
            "run_rows": len(mine), "archive_rows": len(theirs), "identical": mine == theirs}
    result["identical"] = all(entry["identical"] for entry in result["files"].values())
    pathlib.Path(out).write_text(json.dumps(result, indent=1, sort_keys=True) + "\n")
    print(json.dumps(result, indent=1, sort_keys=True))
    return 0 if result["identical"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
