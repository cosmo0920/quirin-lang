#!/usr/bin/env python3
"""Wave-17 consolidation: check the two recorded residue classes leaf by leaf.

Read-only, and reproducible from committed bytes: both inputs are archived
artifact sets, so this reproduces `residues.txt` without re-running the audit.

1. `source-effects.json` — every changed leaf must be an identity digest, a key
   derived from one, a package-identity field, or a value carrying this
   worktree's path; and no result's status may move (package identity).
2. `source.json` — every changed leaf must be a path string literal or a
   `char[N]` extent derived from one (path-literal provenance).

    python3 -B docs/reports/wave17-corpus/residues.py
"""
from __future__ import annotations

import collections
import gzip
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[3]
BASELINE = ROOT / "docs/reports/wave16-corpus/artifacts"
COMPOSED = ROOT / "docs/reports/wave17-corpus/artifacts"

# Identity fields and keys derived from them. `paths`, `kind` and
# `observed_runs_only` appear only inside a `bounds` map that is re-keyed by a
# new obligation id: both the old and the new key's subtree count as changed.
IDENTITY_TERMS = (
    "digest", "obligation", "task", "package", "provider_version", "bounds",
    "coverage", "configuration",
)


def load(directory: pathlib.Path, name: str) -> dict:
    with gzip.open(directory / f"{name}.json.gz", "rt") as handle:
        return json.load(handle)


def flatten(node, path=()):
    if isinstance(node, dict):
        for key, value in node.items():
            yield from flatten(value, path + (str(key),))
    elif isinstance(node, list):
        for index, value in enumerate(node):
            yield from flatten(value, path + (str(index),))
    else:
        yield path, node


def changed_leaves(before, after) -> dict:
    b = dict(flatten(before))
    a = dict(flatten(after))
    return {k: (b.get(k), a.get(k)) for k in set(b) | set(a) if b.get(k) != a.get(k)}


def main() -> int:
    ok = True

    print("=== source-effects.json (package identity)")
    before = load(BASELINE, "source-effects")
    after = load(COMPOSED, "source-effects")
    changed = changed_leaves(before, after)
    unexplained = {}
    for key, pair in changed.items():
        joined = "/".join(key)
        if any(term in joined for term in IDENTITY_TERMS):
            continue
        if "/worktrees/" in f"{pair[0]}|{pair[1]}":
            continue
        unexplained[joined] = pair
    fields = collections.Counter(key[-1] for key in changed)
    print(f"changed leaves: {len(changed)}; by field: {dict(sorted(fields.items()))}")
    print(f"outside the identity/path classes: {len(unexplained)}")
    for key, pair in sorted(unexplained.items()):
        print(f"  UNEXPLAINED {key} -> {str(pair)[:200]}")
        ok = False
    statuses_before = [(r.get("function"), r.get("status")) for r in before["results"]]
    statuses_after = [(r.get("function"), r.get("status")) for r in after["results"]]
    print(f"status split before: {dict(collections.Counter(s for _, s in statuses_before))}")
    print(f"status split after:  {dict(collections.Counter(s for _, s in statuses_after))}")
    print(f"per-result statuses equal: {statuses_before == statuses_after}")
    ok = ok and statuses_before == statuses_after

    print("=== source.json (path-literal provenance)")
    before = load(BASELINE, "source")
    after = load(COMPOSED, "source")
    changed = changed_leaves(before, after)
    literals = extents = 0
    other = {}
    for key, pair in changed.items():
        text = f"{pair[0]}|{pair[1]}"
        if "/worktrees/" in text or "/build/cycle19/" in text:
            literals += 1
        elif "char[" in text:
            extents += 1
        else:
            other["/".join(key)] = pair
    print(f"changed leaves: {len(changed)}; path literals: {literals}; "
          f"char[N] extents: {extents}; other: {len(other)}")
    for key, pair in sorted(other.items()):
        print(f"  OTHER {key} -> {str(pair)[:200]}")
        ok = False

    print(f"RESULT: {'both residue classes hold' if ok else 'UNEXPLAINED LEAVES'}")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
