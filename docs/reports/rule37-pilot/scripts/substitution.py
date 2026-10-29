#!/usr/bin/env python3
"""RULE37-PILOT: check that report artifacts of two sides are equal up to a
checked one-to-one substitution of the analyzer revision and of the digests
derived from it.

Usage: substitution.py BEFORE_REV AFTER_REV BEFORE_DIR AFTER_DIR FILE [FILE ...]

Each FILE (JSON, or JSON lines when it ends in `.jsonl`) is read on both
sides. Step 1: every string has BEFORE_REV (resp. AFTER_REV) replaced by
`<rev>` and every run of 24 to 64 hex digits (the reports' digests and
24-digit obligation ids) replaced by `<digest>`, in values and keys, and every
list is read as a multiset; the two normalized documents must be equal.
Step 2: list items are aligned by normalized form (tied items, equal after
normalization, are left unpaired and counted) and dict keys by normalized key,
and every pair of differing scalars and keys is recorded; across all files the
pairs must form a bijection (no value maps to two, no two values map to one).
Values that differ in any other way fail step 1.
The output is one JSON object; exit 0 only when both steps pass.

Lists are compared as multisets because the reports sort evidence by digest
identifiers, and those identifiers move with the revision
(MEASURE-CORPUS-SIDE-CWD-IN-BUILD-IDENTITY is the same class of movement).
"""
import json
import re
import sys
from pathlib import Path

DIGEST = re.compile(r"[0-9a-f]{24,64}")


def load(path):
    text = path.read_text(encoding="utf-8")
    if path.suffix == ".jsonl":
        return [json.loads(line) for line in text.splitlines() if line.strip()]
    return json.loads(text)


def normalize(value, rev):
    if isinstance(value, dict):
        # A multiset of (key, value) pairs: two keys that normalize alike must
        # both be kept, never merged.
        return ["<dict>"] + sorted(
            ([normalize(k, rev), normalize(v, rev)] for k, v in value.items()),
            key=lambda v: json.dumps(v, sort_keys=True))
    if isinstance(value, list):
        return sorted((normalize(v, rev) for v in value), key=lambda v: json.dumps(v, sort_keys=True))
    if isinstance(value, str):
        return DIGEST.sub("<digest>", value.replace(rev, "<rev>"))
    return value


def keyed(value, rev):
    return json.dumps(normalize(value, rev), sort_keys=True)


def pairs(before, after, before_rev, after_rev, out, skipped):
    """Pair the differing scalars (and dict keys) of two documents equal after
    normalization. List items are aligned by normalized form; items whose
    normalized form repeats in their list (ties) are not paired, since any
    alignment of them is equally consistent, and are counted in `skipped`."""
    if isinstance(before, dict):
        by_key = {}
        for key in after:
            by_key.setdefault(normalize(key, after_rev), []).append(key)
        for key in before:
            candidates = by_key.get(normalize(key, before_rev), [])
            if len(candidates) != 1:
                skipped[0] += 1
                continue
            if key != candidates[0]:
                out.append((key, candidates[0]))
            pairs(before[key], after[candidates[0]], before_rev, after_rev, out, skipped)
    elif isinstance(before, list):
        groups_before, groups_after = {}, {}
        for item in before:
            groups_before.setdefault(keyed(item, before_rev), []).append(item)
        for item in after:
            groups_after.setdefault(keyed(item, after_rev), []).append(item)
        for key, items in groups_before.items():
            if len(items) == 1 and len(groups_after.get(key, [])) == 1:
                pairs(items[0], groups_after[key][0], before_rev, after_rev, out, skipped)
            else:
                skipped[0] += len(items)
    elif before != after:
        out.append((before, after))


def main():
    before_rev, after_rev, before_dir, after_dir, *files = sys.argv[1:]
    report, forward, backward = {}, {}, {}
    ok = True
    for name in files:
        before = load(Path(before_dir) / name)
        after = load(Path(after_dir) / name)
        equal = normalize(before, before_rev) == normalize(after, after_rev)
        found, skipped = [], [0]
        if equal:
            pairs(before, after, before_rev, after_rev, found, skipped)
        for b, a in found:
            forward.setdefault(json.dumps(b), set()).add(json.dumps(a))
            backward.setdefault(json.dumps(a), set()).add(json.dumps(b))
        report[name] = {"equal_after_normalization": equal, "differing_scalars": len(found),
                        "tied_items_not_paired": skipped[0]}
        ok = ok and equal
    bijective = all(len(v) == 1 for v in forward.values()) and all(
        len(v) == 1 for v in backward.values())
    revision_pairs = sum(1 for k in forward if before_rev in k)
    print(json.dumps({
        "ok": ok and bijective,
        "bijective": bijective,
        "substituted_values": len(forward),
        "values_naming_the_revision": revision_pairs,
        "files": report,
        "not_one_to_one": sorted(k for k, v in forward.items() if len(v) > 1)[:5]
        + sorted(k for k, v in backward.items() if len(v) > 1)[:5],
    }, indent=1))
    return 0 if ok and bijective else 1


if __name__ == "__main__":
    sys.exit(main())
