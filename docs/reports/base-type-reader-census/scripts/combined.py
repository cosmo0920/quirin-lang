#!/usr/bin/env python3
"""Card BASE-TYPE-READER-CENSUS: the two readers changed together.

`discount.py` shows the typedef-aggregate cause's cleared set falling from 30 to
12 once the member is required to classify. Five of the eighteen that fall do so
only because the record body is refused whole -- which is residual
`RECORD-BODY-TOTAL-OR-REJECT`, a separate reader. This measures the pair: the
base-type reader resolving `typedef struct TAG { ... } t;` AND a per-member
record-body reader, landed together.

Usage: python3 combined.py <run-dir>
"""
import collections
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from base_causes import (  # noqa: E402
    INDIRECT, cause_key, cause_payload, load_census, load_document, match_units,
)

CAUSE = "DeclaredOpaque/TypedefDefinesAggregate"


def main():
    run = pathlib.Path(sys.argv[1])
    blockers, unit_leaves, leaf_functions = load_document(run / "resource-diagnostics.json")
    census, _ = load_census(run / "indirect-leaves.jsonl")
    assign = match_units(census, unit_leaves)
    bodies = {}
    for line in open(run / "record-bodies.jsonl"):
        note = json.loads(line)
        bodies.setdefault(note["tag"], note)

    statements = {}
    for ukey, keys in unit_leaves.items():
        seq = assign.get(ukey)
        recs = census.get(seq, {}) if seq is not None else {}
        for key in keys:
            statements[ukey + key] = recs.get(key, [])

    own = collections.defaultdict(list)
    touched = set()
    for key, records in statements.items():
        f = leaf_functions[key]
        own[f].extend(records)
        for record in records:
            if any(cause_key(c) == CAUSE for c in record["chains"]):
                touched.add(f)

    def exchanges(record):
        return bool(record["contains_call"]) and not record.get("call_contracted")

    cleared = []
    for f in touched:
        if set(blockers[f]) != {INDIRECT}:
            continue
        records = own[f]
        if not records:
            continue
        if any(not all(cause_key(c) in ("Committed", CAUSE) for c in r["chains"])
               for r in records):
            continue
        if any(exchanges(r) for r in records):
            continue
        cleared.append(f)

    def names_a_known_tag(chain):
        head = (cause_payload(chain) or "").split()
        return len(head) == 2 and head[0] == "struct" and head[1] in bodies

    combined = []
    premise = collections.Counter()
    for f in sorted(cleared):
        chains = [c for r in own[f] for c in r["chains"] if cause_key(c) == CAUSE]
        if all(names_a_known_tag(c) for c in chains):
            combined.append(f)
            premise["premise HOLDS (every blocked base never-bound)"
                    if not any(c["base_may_bind"] for c in chains)
                    else "premise UNAVAILABLE (a blocked base may bind)"] += 1

    print("first discount  (base reader alone, member assumed to classify): %d" % len(cleared))
    print("second discount (member must actually classify):                 "
          "see discount.py")
    print("BOTH READERS    (typedef-aggregate resolution + per-member body): %d"
          % len(combined))
    for name, count in premise.most_common():
        print("   %4d  %s" % (count, name))
    print()
    for f in combined:
        print("    %s" % "/".join(str(x) for x in f))
    return 0


if __name__ == "__main__":
    sys.exit(main())
