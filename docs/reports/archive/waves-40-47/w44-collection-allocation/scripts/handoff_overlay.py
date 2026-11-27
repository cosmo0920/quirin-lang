#!/usr/bin/env python3
"""Card W44-COLLECTION-ALLOCATION on main 70ce6b44: which of the card's leaks
the refused hand-on rule withdraws, and which it leaves.

Usage: handoff_overlay.py BEFORE_RUN AFTER_RUN RULE_OFF_JOINED.json OUT_JSON OUT_TXT

BEFORE_RUN is main's run (the combined baseline), AFTER_RUN this card merged
with main (the rule on). RULE_OFF_JOINED.json is W44-LEAK-PIN-AUDIT's
`join_witnesses.py` output over a census run of this merged tree with the
refused hand-on rule's `apply` disabled (`rule_off.py`, the audit's
evidence/census-rule-off.patch). A rule-off leaf is **this card's** when the
BEFORE run holds, in the same function, the `indirect memory access` refusal
whose origin coordinate is the leaf's acquisition coordinate: the allocation
this card admitted. Main's own rule-off leaves are not this card's and are
counted only as a total.

Each of the card's rule-off leaves gets the audit's overlay class:

* `withdrawn`: a handed path exists in every joined invocation (the leaked
  identity is named in a refused statement on every analysis of the body);
* `mentioned`: a handed path exists in some invocation only;
* `not-mentioned`: none.

and whether the leaf is present in AFTER (the rule on). A leaf the rule
withdraws must be absent from AFTER and a leaf it cannot withdraw present,
which the script checks and reports.
"""

import collections
import json
import pathlib
import re
import sys

ACCESS = "indirect memory access is outside the resource projection"
LEAK = re.compile(r"live obligation acquired at (\S+)")


def leak_leaves(run):
    document = json.loads((run / "resource-diagnostics.json").read_text())
    rows = collections.Counter()
    access_origins = collections.defaultdict(set)
    for unit in document["units"]:
        for function in unit.get("functions", []):
            for leaf in function.get("diagnostics", []):
                if leaf["category"] == "leak":
                    rows[(unit["file"], function["function"], leaf["line"], leaf["description"])] += 1
                elif leaf["description"] == ACCESS and leaf.get("origin"):
                    access_origins[(unit["file"], function["function"])].add(leaf["origin"])
    return rows, access_origins


def main() -> int:
    before_leaks, before_access = leak_leaves(pathlib.Path(sys.argv[1]))
    after_leaks, _ = leak_leaves(pathlib.Path(sys.argv[2]))
    joined = json.loads(pathlib.Path(sys.argv[3]).read_text())["joined"]
    out_json, out_txt = pathlib.Path(sys.argv[4]), pathlib.Path(sys.argv[5])
    remaining = collections.Counter(after_leaks)
    rows = []
    mains = 0
    for leaf in joined:
        origin = LEAK.search(leaf["description"]).group(1)
        # The unit key of the diagnostics is `file`; the joined record's is `unit`.
        key = (leaf["unit"], leaf["function"])
        if origin not in before_access.get(key, set()):
            mains += 1
            continue
        witness = leaf["witness"]
        if witness["handed_path_in_every_invocation"]:
            overlay = "withdrawn"
        elif witness["handed_path_in_some_invocation"]:
            overlay = "mentioned"
        else:
            overlay = "not-mentioned"
        signature = (leaf["unit"], leaf["function"], leaf["line"], leaf["description"])
        present = remaining[signature] > 0
        if present:
            remaining[signature] -= 1
        rows.append({
            "unit": leaf["unit"], "function": leaf["function"], "line": leaf["line"],
            "acquisition": origin, "overlay": overlay, "present_with_rule_on": present,
            "exit_unmodeled": witness.get("exit_unmodeled"),
        })
    consistent = all((r["overlay"] == "withdrawn") != r["present_with_rule_on"] for r in rows)
    added_after = sum((after_leaks - before_leaks).values())
    summary = {
        "rule_off_leaves": len(joined),
        "rule_off_leaves_not_this_card": mains,
        "this_card_rule_off_leaves": len(rows),
        "this_card_functions": len({(r["unit"], r["function"]) for r in rows}),
        "by_overlay": dict(collections.Counter(r["overlay"] for r in rows).most_common()),
        "present_with_rule_on": sum(r["present_with_rule_on"] for r in rows),
        "leaves_after_adds_over_before": added_after,
        "overlay_agrees_with_rule_on_run": consistent,
    }
    out_json.write_text(json.dumps({"summary": summary, "leaves": rows}, indent=1) + "\n")
    text = ["W44-COLLECTION-ALLOCATION: the card's leaks under the refused hand-on rule (main 70ce6b44)", ""]
    text += [f"{k}: {v}" for k, v in summary.items()]
    text.append("")
    text.append("leaves the rule leaves standing:")
    for r in rows:
        if r["present_with_rule_on"] or r["overlay"] != "withdrawn":
            text.append(f"  [{r['overlay']}] {r['unit']}::{r['function']} L{r['line']} ({r['acquisition']}) present={r['present_with_rule_on']}")
    out_txt.write_text("\n".join(text) + "\n")
    print("\n".join(text))
    return 0


if __name__ == "__main__":
    sys.exit(main())
