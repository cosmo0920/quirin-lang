#!/usr/bin/env python3
"""member_counts.py ROWS.jsonl [OUT.json] -- NULL-MEMBER-INVARIANT-SCOPE: the
candidate counts this card reports for one null selection's rows.

* candidates, split into the functions' own checks and the travelled caller
  checks (rule 14v: a check whose subject names a `cross-unit inferred`
  clause), each by strongest source kind (candidates.py's order);
* memory-field candidates on the own checks;
* candidates with a source naming each member this card considers
  (`->rkb_rk`, `->rkbuf_rkb`, `->next`), own and travelled;
* call-precondition checks of scope entries (`nonnull_member_scope`) by outcome;
* checks whose proofs carry a stable member's proof (`result:stable:`)."""
import collections
import json
import re
import sys

ORDER = ["allocation", "lookup", "nullable-result", "null-literal", "uninitialized",
         "uncontracted-result", "formal", "global", "memory-field", "other"]
RANK = {kind: index for index, kind in enumerate(ORDER)}
MEMBERS = ["rkb_rk", "rkbuf_rkb", "next"]


def kind_of(source):
    head = source.split(":", 1)[0]
    return head if head in RANK else "other"


own = collections.Counter()
travelled = collections.Counter()
members = {m: collections.Counter() for m in MEMBERS}
scope_checks = collections.Counter()
stable_proved = collections.Counter()
rows = 0
for line in open(sys.argv[1]):
    if not line.strip():
        continue
    row = json.loads(line)
    rows += 1
    for check in (row.get("null") or {}).get("checks") or []:
        proofs = check.get("proofs") or []
        if any(p.startswith("result:stable:") for p in proofs):
            stable_proved[check["outcome"]] += 1
        if "nonnull_member_scope(" in check.get("subject", ""):
            scope_checks[check["outcome"]] += 1
        if check["outcome"] != "candidate":
            continue
        sources = check.get("sources") or []
        strongest = min((kind_of(s) for s in sources), key=lambda k: RANK[k], default="other")
        side = "travelled" if "cross-unit inferred" in check.get("subject", "") else "own"
        (travelled if side == "travelled" else own)[strongest] += 1
        for member in MEMBERS:
            if any(s.startswith("memory-field:") and re.search(r"(->|\.)%s$" % member, s) for s in sources):
                members[member][side] += 1
record = {
    "rows": rows,
    "candidates": sum(own.values()) + sum(travelled.values()),
    "own": sum(own.values()),
    "travelled": sum(travelled.values()),
    "own_by_strongest_kind": dict(sorted(own.items(), key=lambda kv: RANK[kv[0]])),
    "travelled_by_strongest_kind": dict(sorted(travelled.items(), key=lambda kv: RANK[kv[0]])),
    "memory_field_own": own["memory-field"],
    "member_sourced": {m: dict(c) for m, c in members.items()},
    "scope_entry_checks_by_outcome": dict(scope_checks),
    "checks_with_a_stable_proof_by_outcome": dict(stable_proved),
}
text = json.dumps(record, indent=1, sort_keys=True)
if len(sys.argv) > 2:
    open(sys.argv[2], "w").write(text + "\n")
print(text)
