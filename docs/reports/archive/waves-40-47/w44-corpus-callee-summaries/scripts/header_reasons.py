#!/usr/bin/env python3
"""Why an admitted header definition validated no summary, weighted by leaves.

Usage: header_reasons.py CALLEE_CENSUS.json SCOPE_CENSUS.jsonl[.gz]

For every `in-unit-header-internal:no_summary` leaf of `callee_census.py`, reads
the widened derivation's own analysis of the callee's body in the calling unit
(`body_blockers`, at most six distinct descriptions) and sorts it:

* `bswap-only` -- blocked only by uncontracted `__bswap_*` calls (W44-LIBC-RESIDUALS);
* `cascade-only` -- blocked only by uncontracted calls (to anything);
* `unmodeled` -- carries some other refusal (indirect access, condition, ...).

A body with six distinct blockers may carry more; it is never cascade-only.
"""

import collections
import gzip
import json
import re
import sys

CALL = re.compile(r"call to uncontracted function `([^`]+)`")


def main() -> int:
    census = json.load(gzip.open(sys.argv[1]) if sys.argv[1].endswith(".gz") else open(sys.argv[1]))
    opener = gzip.open if sys.argv[2].endswith(".gz") else open
    bodies = {}
    for line in opener(sys.argv[2], "rt"):
        row = json.loads(line)
        for callee in row["callees"]:
            if callee["outcome"] == "no_summary":
                bodies.setdefault(callee["name"], collections.Counter())
                for description, count in callee["body_blockers"]:
                    bodies[callee["name"]][description] += count
    groups = collections.Counter()
    callees = collections.Counter()
    first = collections.Counter()
    for row in census["callees"]:
        leaves = row["classes"].get("in-unit-header-internal:no_summary")
        if not leaves:
            continue
        blockers = bodies.get(row["callee"], collections.Counter())
        calls = [CALL.fullmatch(d) for d in blockers]
        if blockers and all(calls) and len(blockers) < 6:
            names = {m.group(1) for m in calls}
            group = "bswap-only" if all(n.startswith("__bswap_") for n in names) else "cascade-only"
        else:
            group = "unmodeled"
        groups[group] += leaves
        callees[group] += 1
        if blockers:
            top = blockers.most_common(1)[0][0]
            first[CALL.sub("call to uncontracted function", top)] += leaves
    print(json.dumps({"leaves": dict(groups), "callees": dict(callees),
                      "dominant_blocker_leaves": dict(first.most_common(10))}, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
