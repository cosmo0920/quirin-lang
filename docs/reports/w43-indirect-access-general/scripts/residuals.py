#!/usr/bin/env python3
"""Card W43-INDIRECT-ACCESS-GENERAL: size every refused shape at the AFTER tree.

Input: a census run of the card's final tree (`--census` with an
`indirect-census` build). Every leaf of the five census families in an Unknown
function is put in the class of its first statement (in the order below), and
each class reports leaves, Unknown functions and functions whose only blocker
family is the leaf's own family.

Classes, in precedence order:

* `call-argument` - the access sits in a statement whose value or effect is a
  call (`f(p->x)`, `x = f(p->x)`, `p->f = g()`);
* `chain` - a member chain of depth two or more, or a subscript followed by a
  member (`a->b->c`, `a[i].f`);
* `pointer-value-store` - a single access destination whose stored value is a
  pointer, an address or a pointer-typed name (`p->f = q`, `*pp = q`);
* `pointer-increment` - the statement increments the pointer it dereferences
  (`*p++ = c`) or dereferences a computed pointer (`*(p + 1) = c`);
* `untyped-single-access` - a single `*p`, `p[i]`, `p->f` or `s.f` whose base
  or member the declaration reader did not classify as the rule requires;
* `other` - anything else (expression statements, declarations, unexplained).

Usage: residuals.py CENSUS_RUN OUT_JSON OUT_TXT
"""

import collections
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import predict  # noqa: E402
import shape_census as census  # noqa: E402

ORDER = ["call-argument", "chain", "pointer-value-store", "pointer-increment", "untyped-single-access", "other"]


def classify(record):
    shape, value, base = census.classify(record["texts"], record.get("declared", {}))
    if shape in ("call-statement", "assign-call") or value == "call":
        return "call-argument"
    if "chain" in shape:
        return "chain"
    if shape.startswith(("store-postinc", "increment:*(expr)", "store:*(expr)", "compound:*(expr)", "read:*(expr)")):
        return "pointer-increment"
    if shape.startswith(("store:", "compound:")) and value in ("pointer", "address"):
        return "pointer-value-store"
    if any(shape.endswith(form) for form in (":*p", ":p[i]", ":p->f", ":s.f")):
        return "untyped-single-access"
    return "other"


def main():
    run = pathlib.Path(sys.argv[1])
    _, functions, unit_keys = census.load_document(run / "resource-diagnostics.json")
    indirect = census.load_stream(run / "indirect-census.jsonl", lambda record: census.INDIRECT)
    statements = census.load_stream(
        run / "statement-census.jsonl", lambda record: census.outcome_family(record["outcome"])
    )
    assignment = census.match_units([indirect, statements], unit_keys)
    leaves = collections.Counter()
    touched = collections.defaultdict(set)
    sole = collections.defaultdict(set)
    per_family = collections.Counter()
    for fkey, data in functions.items():
        if data["status"] != "Unknown":
            continue
        families = {census.normalize(d) for _, d in data["leaves"]}
        seq = assignment.get(fkey[:2])
        for line, description in data["leaves"]:
            if not predict.intercepted(description):
                continue
            key = (fkey[2], line, description)
            records = []
            if seq is not None:
                records = statements.get(seq, {}).get(key, [])
                if description == census.INDIRECT:
                    records = indirect.get(seq, {}).get(key, []) or records
            classes = {classify(record) for record in records} or {"other"}
            cls = min(classes, key=ORDER.index)
            leaves[cls] += 1
            per_family[(cls, census.normalize(description))] += 1
            touched[cls].add(fkey)
            if len(families) == 1:
                sole[cls].add(fkey)
    result = {
        cls: {
            "leaves": leaves[cls],
            "unknown_functions": len(touched[cls]),
            "sole_blocked_functions": len(sole[cls]),
            "by_family": {fam: n for (c, fam), n in per_family.items() if c == cls},
        }
        for cls in ORDER
    }
    pathlib.Path(sys.argv[2]).write_text(json.dumps(result, indent=1) + "\n")
    lines = [f"{'class':24} {'leaves':>7} {'funcs':>6} {'sole':>5}"]
    for cls in ORDER:
        row = result[cls]
        lines.append(f"{cls:24} {row['leaves']:7} {row['unknown_functions']:6} {row['sole_blocked_functions']:5}")
        for fam, n in sorted(row["by_family"].items(), key=lambda kv: -kv[1]):
            lines.append(f"    {n:6}  {fam}")
    pathlib.Path(sys.argv[3]).write_text("\n".join(lines) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
