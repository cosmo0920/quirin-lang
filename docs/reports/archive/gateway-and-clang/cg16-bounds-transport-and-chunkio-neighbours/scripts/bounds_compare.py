#!/usr/bin/env python3
"""CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS: attribute every difference between two
bounds-selected corpus runs (NAME-bounds.jsonl under
private/cg16-bounds-transport-and-chunkio-neighbours/), row by row.

Usage: bounds_compare.py BEFORE AFTER [OUT.json]

Rows are keyed (library, file, function, occurrence). For each row: verdict,
custody and combined moves; the bounds reasons added and removed; the
judgements (accesses) whose `actual` changed, with their kind, subject and
line; and whether the identity (`contracts_sha256` and the rest) changed.
Rows that differ only in the identity are counted, not listed."""
import collections
import json
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
OUT = REPO / "private/cg16-bounds-transport-and-chunkio-neighbours"


def rows(name):
    seen = collections.Counter()
    result = {}
    for line in (OUT / f"{name}-bounds.jsonl").read_text().splitlines():
        row = json.loads(line)
        key = (row["library"], row["file"], row["function"])
        seen[key] += 1
        result[key + (seen[key],)] = row
    return result


def accesses(row):
    bounds = row.get("bounds") or {}
    return [(a.get("kind"), a.get("subject"), a.get("line"), a.get("actual"))
            for a in bounds.get("accesses", [])]


def main():
    before, after = rows(sys.argv[1]), rows(sys.argv[2])
    assert before.keys() == after.keys(), "row sets differ"
    identity_only = 0
    identity_changed = 0
    moved = []
    changed = []
    contracts_sha256 = collections.Counter()
    for key in sorted(before):
        b, a = before[key], after[key]
        if b.get("identity") != a.get("identity"):
            identity_changed += 1
        contracts_sha256[((b.get("identity") or {}).get("contracts_sha256"),
                          (a.get("identity") or {}).get("contracts_sha256"))] += 1
        bb, ab = b.get("bounds") or {}, a.get("bounds") or {}
        same_rest = (b["custody"] == a["custody"] and b["combined"] == a["combined"]
                     and bb.get("verdict") == ab.get("verdict")
                     and bb.get("reasons") == ab.get("reasons")
                     and accesses(b) == accesses(a))
        if same_rest:
            identity_only += b.get("identity") != a.get("identity")
            continue
        record = {
            "row": list(key),
            "bounds": [bb.get("verdict"), ab.get("verdict")],
            "custody": [b["custody"], a["custody"]],
            "combined": [b["combined"], a["combined"]],
            "reasons_removed": sorted(set(bb.get("reasons", [])) - set(ab.get("reasons", []))),
            "reasons_added": sorted(set(ab.get("reasons", [])) - set(bb.get("reasons", []))),
            "judgements_changed": [
                {"before": list(x), "after": list(y)}
                for x, y in zip(accesses(b), accesses(a)) if x != y
            ],
            "judgement_count": [len(accesses(b)), len(accesses(a))],
        }
        (moved if record["bounds"][0] != record["bounds"][1]
         or record["combined"][0] != record["combined"][1] else changed).append(record)
    result = {
        "before": sys.argv[1], "after": sys.argv[2], "rows": len(before),
        "identity_changed_rows": identity_changed,
        "identity_only_rows": identity_only,
        "contracts_sha256_pairs": [[k[0], k[1], v] for k, v in contracts_sha256.items()],
        "verdict_moves": moved,
        "same_verdict_other_changes": changed,
    }
    text = json.dumps(result, indent=1) + "\n"
    if len(sys.argv) > 3:
        pathlib.Path(sys.argv[3]).write_text(text)
    print(json.dumps({k: (len(v) if isinstance(v, list) else v) for k, v in result.items()},
                     indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
