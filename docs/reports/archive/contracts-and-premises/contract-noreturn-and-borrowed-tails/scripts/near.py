#!/usr/bin/env python3
"""CONTRACT-NORETURN-AND-BORROWED-TAILS: what else blocks the functions the
card's leaves touch.

Usage: near.py DIAGNOSTICS OUT.json

For every non-Safe function holding a card leaf (`call to uncontracted
function `rd_kafka_crash``, or the tail refusal of printf, fprintf,
snprintf, mk_print, flb_log_print): the card leaves it holds and the other
leaves, grouped by family (category + description with backquoted names
masked). `remaining` histograms the number of other leaves; `one_other` lists
the functions with exactly one.
"""
import collections
import json
import pathlib
import re
import sys

CARD = re.compile(r"^(call to uncontracted function `rd_kafka_crash`|arguments beyond the declared "
                  r"parameters of `(printf|fprintf|snprintf|mk_print|flb_log_print)` are outside the "
                  r"resource projection)$")
TEMPLATE = re.compile(r"`[^`]*`")


def main() -> int:
    path = pathlib.Path(sys.argv[1])
    if path.is_dir():
        path = path / "resource-diagnostics.json"
    data = json.loads(path.read_text())
    rows = []
    hist = collections.Counter()
    families = collections.Counter()
    for unit in data["units"]:
        for entry in unit["functions"]:
            leaves = entry.get("diagnostics") or []
            card = [d for d in leaves if CARD.match(d.get("description", ""))]
            if not card or entry["status"] == "Safe":
                continue
            other = [d for d in leaves if not CARD.match(d.get("description", ""))]
            hist[min(len(other), 6)] += 1
            for d in other:
                families[d["category"] + ": " + TEMPLATE.sub("`X`", d.get("description", ""))] += 1
            rows.append({
                "function": f"{unit['file']}::{entry['function']}",
                "status": entry["status"],
                "card": collections.Counter(d["description"] for d in card),
                "other": [f"{d.get('line')}: {d['category']}: {d.get('description', '')}" for d in other],
            })
    one = [r for r in rows if len(r["other"]) == 1]
    out = {"functions": len(rows), "remaining": {str(k): v for k, v in sorted(hist.items())},
           "other_families": dict(families.most_common(25)),
           "one_other": one, "rows": rows}
    pathlib.Path(sys.argv[2]).write_text(json.dumps(out, indent=1, ensure_ascii=False) + "\n")
    print(json.dumps({k: out[k] for k in ("functions", "remaining")}))
    for r in one:
        print(r["status"], r["function"], dict(r["card"]), r["other"])
    return 0


if __name__ == "__main__":
    sys.exit(main())
