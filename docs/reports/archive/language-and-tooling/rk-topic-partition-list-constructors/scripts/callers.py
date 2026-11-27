#!/usr/bin/env python3
"""callers.py REPORT [OUT.json] -- RK-TOPIC-PARTITION-LIST-CONSTRUCTORS: on one
opt-in `current` report, every row naming one of the card's functions in a
`call to uncontracted function` leaf, with its status, its leaves grouped by
description, and whether the card's functions are its only leaves ("sole").
The card's functions are the four declared constructors/destructor; the
co-blocking uncontracted callees are ranked to show what else the callers
would need."""
import collections
import json
import re
import sys

CARD = {
    "rd_kafka_topic_partition_list_new",
    "rd_kafka_topic_partition_list_copy",
    "rd_kafka_topic_partition_copy",
    "rd_kafka_topic_partition_destroy",
}
UNC = re.compile(r"call to uncontracted function `([^`]+)`")


def main() -> int:
    data = json.load(open(sys.argv[1]))
    rows = []
    co = collections.Counter()
    co_alone = collections.Counter()
    for unit in data["units"]:
        for function in unit["functions"]:
            descriptions = [d["description"] for d in function["diagnostics"]]
            named = {m.group(1) for d in descriptions for m in [UNC.search(d)] if m}
            if not named & CARD:
                continue
            other = [d for d in descriptions
                     if not (UNC.search(d) and UNC.search(d).group(1) in CARD)]
            other_callees = sorted(named - CARD)
            for callee in other_callees:
                co[callee] += 1
            others_all_unc = all(UNC.search(d) for d in other)
            if others_all_unc and len(other_callees) == 1:
                co_alone[other_callees[0]] += 1
            rows.append({
                "unit": unit["file"],
                "function": function["function"],
                "status": function["status"],
                "card_callees": sorted(named & CARD),
                "sole": not other,
                "other_leaves": dict(collections.Counter(
                    re.sub(r"`[^`]*`", "`_`", d)[:90] for d in other)),
                "other_uncontracted": other_callees,
            })
    summary = {
        "rows": len(rows),
        "by_status": dict(collections.Counter(r["status"] for r in rows)),
        "sole": sum(r["sole"] for r in rows),
        "by_card_callee": dict(collections.Counter(c for r in rows for c in r["card_callees"])),
        "co_uncontracted_top": co.most_common(25),
        "co_uncontracted_with_card_alone": co_alone.most_common(15),
    }
    print(json.dumps(summary, indent=1))
    if len(sys.argv) > 2:
        json.dump({"summary": summary, "rows": rows}, open(sys.argv[2], "w"), indent=1)
    return 0


if __name__ == "__main__":
    sys.exit(main())
