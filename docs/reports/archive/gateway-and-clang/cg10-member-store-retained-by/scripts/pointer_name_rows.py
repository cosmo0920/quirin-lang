#!/usr/bin/env python3
"""pointer_name_rows.py PREMISES.jsonl.gz PPDIR -- CG10-MEMBER-STORE-RETAINED-BY census.

Uses round 3's classifier (`refine.py`'s `shape`, imported from the round-3
report's scripts) over a census `premises.jsonl.gz`, keeps the sole-blocker
functions (every leaf family the access refusal, status Unknown) with at least
one `pointer-name-into-member` leaf, and prints, per function, each such leaf's
statement, the stored name, its root kind and type, the holder place, and the
function's other sub-shapes. JSON lines on stdout."""
import collections
import gzip
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
R3 = os.path.join(HERE, "..", "..", "cg10-indirect-access-sole-residuals-r3", "scripts")
sys.path.insert(0, R3)
from leaves import statement  # noqa: E402
from refine import IND, shape  # noqa: E402


def main() -> int:
    rows = [json.loads(line) for line in gzip.open(sys.argv[1], "rt")]
    ppdir = sys.argv[2]
    by = collections.defaultdict(list)
    for row in rows:
        if set(row["families"]) <= {IND} and row["status"] == "Unknown":
            by[(row["file"], row["function"])].append(row)
    cache = {}
    for key, leaves in sorted(by.items()):
        path = os.path.join(ppdir, f"unit-{leaves[0]['unit']}.c")
        if path not in cache:
            cache[path] = open(path, errors="replace").read().split("\n") if os.path.exists(path) else []
        lines = cache[path]
        shaped = []
        for leaf in leaves:
            text = statement(lines, leaf["line"] - 1)[0] if lines else leaf["origin_text"]
            shaped.append((shape(leaf, text)[0], leaf, text))
        names = {s for s, _, _ in shaped}
        if "pointer-name-into-member" not in names:
            continue
        stores = []
        for s, leaf, text in shaped:
            if s != "pointer-name-into-member":
                continue
            for p in leaf["paths"]:
                if p["access"] != "store":
                    continue
                stores.append({
                    "origin": leaf["origin"],
                    "statement": text[:200],
                    "holder_root": p["root"],
                    "holder_root_kind": p["root_kind"],
                    "holder_steps": [(st["op"], st["member"], st["type"]) for st in p["steps"]],
                    "rhs": p.get("rhs"),
                    "rhs_kind": p.get("rhs_kind"),
                    "rhs_type": p.get("rhs_type"),
                })
        print(json.dumps({
            "file": key[0],
            "function": key[1],
            "alone": names == {"pointer-name-into-member"},
            "other_shapes": sorted(names - {"pointer-name-into-member"}),
            "other_leaves": [(s, l["origin_text"][:120]) for s, l, _ in shaped if s != "pointer-name-into-member"],
            "stores": stores,
        }))
    return 0


if __name__ == "__main__":
    sys.exit(main())
