#!/usr/bin/env python3
"""Explain the added tail and nested-call refusals, and bound what widening
the tail rule could move.

Usage: exchange.py ATTRIBUTION.json CALL_SHAPES.json(.gz) AFTER_DIR OUT.json

For every ADDED `arguments beyond the declared parameters of `X`` leaf (X a
family member), the baseline call-shape rows of the same unit, function and
analysed line whose callee is X give the lexical tail(s) of the refused call(s)
on that line. The leaf is labelled:

* `literal-tail`: every such call's tail is literals only. `is_data_only_actual`
  admits a literal under T1-T4 whatever the declarations say, so the only
  refusal left is `tail_arguments_are_data_with_nested`'s first premise: the
  function's typed declaration facts are unreadable (`FunctionTypes::is_readable`
  is false, the total-or-reject local reader). This is certain.
* `in-unreadable-function`: the tail carries a name, and the same function has a
  `literal-tail` leaf, so the function is unreadable and that alone refuses it.
* `name-or-unreadable`: the tail carries a name or an expression and nothing
  above decides which premise refused it.

Then the bound: over the AFTER run, every non-Safe function whose remaining
leaves are ALL family tail or nested-call refusals. Those are the only functions
any widening of the tail rule, or of nested-call admission, could move by
itself.
"""

import collections
import gzip
import json
import pathlib
import re
import sys

FAMILY = {"printf", "fprintf", "snprintf", "sprintf"}
TAIL = re.compile(r"arguments beyond the declared parameters of `([^`]+)` are outside the resource projection")
NESTED = re.compile(r"nested call `([^`]+)` inside a modeled call is outside the resource projection")


def load(path):
    opener = gzip.open if str(path).endswith(".gz") else open
    with opener(path, "rt") as handle:
        return json.load(handle)


def main() -> int:
    attribution = load(sys.argv[1])
    shapes = load(sys.argv[2])
    after = load(pathlib.Path(sys.argv[3]) / "resource-diagnostics.json")
    out = pathlib.Path(sys.argv[4])
    by_line = collections.defaultdict(list)
    for row in shapes["rows"]:
        if row.get("shape") == "read":
            by_line[(row["unit"], row["function"], row["line"], row["callee"])].append(row)
    added = [r for r in attribution["leaves"] if r["direction"] == "added"]
    labels = []
    for r in added:
        match = TAIL.fullmatch(r["description"])
        if not match or match.group(1) not in FAMILY:
            continue
        calls = by_line.get((f"{r['library']}/{r['unit']}", r["function"], r["line"], match.group(1)), [])
        tails = [c["tail"] for c in calls]
        literal = bool(tails) and all(t and all(x == "literal" for x in t) for t in tails if t) and any(tails)
        labels.append({"row": r, "tails": tails, "literal": literal})
    unreadable = {(l["row"]["library"], l["row"]["unit"], l["row"]["function"]) for l in labels if l["literal"]}
    counts = collections.Counter()
    per_callee = collections.Counter()
    for l in labels:
        key = (l["row"]["library"], l["row"]["unit"], l["row"]["function"])
        if l["literal"]:
            label = "literal-tail"
        elif key in unreadable:
            label = "in-unreadable-function"
        else:
            label = "name-or-unreadable"
        l["label"] = label
        counts[label] += l["row"]["count"]
        per_callee[(TAIL.fullmatch(l["row"]["description"]).group(1), label)] += l["row"]["count"]
    nested = collections.Counter()
    for r in added:
        match = NESTED.fullmatch(r["description"])
        if match:
            nested[match.group(1)] += r["count"]
    bound = []
    for unit in after["units"]:
        for function in unit["functions"]:
            if function["status"] == "Safe":
                continue
            leaves = function.get("diagnostics") or []
            if not leaves:
                continue
            only = True
            for leaf in leaves:
                tail = TAIL.fullmatch(leaf["description"])
                if tail and tail.group(1) in FAMILY:
                    continue
                if NESTED.fullmatch(leaf["description"]):
                    continue
                only = False
                break
            if only:
                bound.append({"unit": f"{unit['library']}/{unit['file']}", "function": function["function"],
                              "status": function["status"],
                              "leaves": dict(collections.Counter(l["description"] for l in leaves))})
    record = {
        "tail_refusals_added": sum(counts.values()),
        "tail_refusals_by_label": dict(counts),
        "tail_refusals_by_callee_and_label": {f"{a} {b}": n for (a, b), n in sorted(per_callee.items())},
        "functions_proved_unreadable": len(unreadable),
        "nested_call_refusals_added_by_inner_callee": dict(nested.most_common()),
        "nested_call_refusals_added": sum(nested.values()),
        "functions_blocked_only_by_family_tail_or_nested_refusals": bound,
    }
    out.write_text(json.dumps(record, indent=1) + "\n")
    brief = dict(record)
    brief["nested_call_refusals_added_by_inner_callee"] = dict(list(nested.most_common())[:15])
    print(json.dumps(brief, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
