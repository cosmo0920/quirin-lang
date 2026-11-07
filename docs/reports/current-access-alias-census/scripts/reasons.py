#!/usr/bin/env python3
"""Card CURRENT-ACCESS-ALIAS-CENSUS: why the typed layer still refuses expressible leaves.

usage: reasons.py RUN_DIR SHAPE_DIR OUT_JSON

For every IND/COND/RET leaf group whose typed path is expressible (tag `expr:*`)
the typed-path parity stream (QCORE_TYPED_PATH_PARITY) holds the refusal reason
of the rows at the leaf's (function, line). The reasons are normalised (quoted
names, backticked text and numbers erased) and counted per coarse shape, with
the facts shape_census.py recorded (root kinds, value kinds, accesses, hops)
and, for `CallRefused`, the callee. The stream keys rows by function name and
line only, so a name shared by two files merges their reasons; the counts are
a description of the refusal, not a verdict input.
"""
import collections
import gzip
import json
import pathlib
import re
import sys

run, shape_dir, out = (pathlib.Path(a) for a in sys.argv[1:4])
parity = collections.defaultdict(set)
callee_of = collections.defaultdict(set)
with open(run / "parity.jsonl") as handle:
    for line in handle:
        row = json.loads(line)
        refusal = row.get("new_refusal")
        if row.get("new_admitted") is False and refusal and row.get("line"):
            text = str(refusal)
            key = (row["function"], row["line"])
            parity[key].add(re.sub(r'"[^"]*"|`[^`]*`|\d+', "X", text)[:80])
            match = re.match(r'CallRefused\("([^"]*)"', text)
            if match:
                callee_of[key].add(match.group(1))


def coarse(tag):
    _, construct, position, _roots = tag.split(":", 3)
    kind = "cond" if construct.startswith("cond") else construct
    return f"expr:{kind}:{'call-arg' if 'call-arg' in position else 'direct'}"


by_shape = collections.defaultdict(lambda: {"leaves": 0, "no_parity_row": 0, "reasons": collections.Counter(),
                                            "roots": collections.Counter(), "values": collections.Counter(),
                                            "accesses": collections.Counter(), "hops": collections.Counter(),
                                            "callees": collections.Counter()})
with gzip.open(shape_dir / "leaves.jsonl.gz", "rt") as handle:
    handle.readline()
    for line in handle:
        row = json.loads(line)
        exprs = [t for t in row["tags"] if t.startswith("expr:")]
        if not exprs:
            continue
        shape = by_shape[coarse(exprs[0])]
        n = row["leaves"]
        shape["leaves"] += n
        key = (row["function"], row["line"])
        reasons = parity.get(key)
        if not reasons:
            shape["no_parity_row"] += n
        for reason in reasons or ():
            shape["reasons"][reason] += n
        for callee in callee_of.get(key, ()):
            shape["callees"][callee] += n
        facts = row["facts"]
        for root in facts["roots"]:
            shape["roots"][root] += n
        for value in facts["values"]:
            shape["values"][value] += n
        for access in facts["accesses"]:
            shape["accesses"][access] += n
        shape["hops"][str(min(facts["max_hops"], 4))] += n
result = {name: {k: (dict(v.most_common(14)) if isinstance(v, collections.Counter) else v) for k, v in s.items()}
          for name, s in by_shape.items()}
out.write_text(json.dumps(result, indent=1) + "\n")
for name, s in sorted(result.items()):
    print(name, s["leaves"], "no-parity-row", s["no_parity_row"])
    print("   reasons", dict(list(s["reasons"].items())[:6]))
    print("   roots", s["roots"], "values", s["values"], "accesses", s["accesses"], "hops", s["hops"])
    print("   callees", dict(list(s["callees"].items())[:8]))
