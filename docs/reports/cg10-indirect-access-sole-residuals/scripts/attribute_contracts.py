#!/usr/bin/env python3
"""attribute_contracts.py COMPARE.json OUT.json -- card
CG10-INDIRECT-ACCESS-SOLE-RESIDUALS: each moving function of compare.py's
output (rule 12k alone -> rule 12k plus the card's contracts) attributed to
the declaration that freed it.

A move is attributed to a declared callee when a lost leaf names it
(``call to uncontracted function `F` ``, or any leaf text spelling `F` for a
held call inside a condition or return), to `callee-summary:G` when a lost leaf
names a function G that itself moved to Safe in the same comparison (its
validated summary now applies), and to `read` otherwise (attributed by reading
the source, in the report).
"""
import collections
import json
import pathlib
import re
import sys

DECLARED = ["cmt_atomic_load", "cmt_atomic_store", "cmt_atomic_compare_exchange",
            "rd_atomic64_get", "rd_atomic32_add", "rd_atomic64_add", "rd_atomic32_set",
            "cnd_timedwait_ms", "rd_hdr_histogram_record", "rd_avg_add"]


def main() -> int:
    data = json.loads(pathlib.Path(sys.argv[1]).read_text())
    safe_now = {m["function"] for m in data["status_moves"] if m["status"][1] == "Safe"}
    by_label = collections.Counter()
    rows = []
    for move in data["moves"]:
        lost = " ".join(move["lost"])
        labels = sorted({name for name in DECLARED if re.search(r"`%s`" % name, lost)})
        if not labels:
            labels = sorted({"callee-summary:" + name for name in safe_now
                             if name != move["function"] and "`%s`" % name in lost})
        if not labels:
            labels = ["read"]
        if move["status"][0] != move["status"][1]:
            for label in labels:
                by_label[label] += 1
        rows.append({"unit": move["unit"], "function": move["function"],
                     "status": move["status"], "labels": labels,
                     "lost": move["lost"], "gained": move["gained"]})
    record = {"status_moves_by_label": dict(by_label.most_common()), "moves": rows}
    pathlib.Path(sys.argv[2]).write_text(json.dumps(record, indent=1) + "\n")
    print(json.dumps(record["status_moves_by_label"]))
    for row in rows:
        if row["status"][0] != row["status"][1]:
            print("%s::%s %s" % (row["unit"], row["function"], ",".join(row["labels"])))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
