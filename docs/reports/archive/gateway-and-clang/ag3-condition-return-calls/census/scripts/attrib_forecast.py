"""Forecast rule 63 (2) on the user's `current` report (lexical approximation).

A condition site is a candidate when the first direct call (textual order,
which is C's evaluation order over && || ?:) of its expression names a callee
the same report refuses elsewhere as `call to uncontracted function`. This is
an upper bound on which conditions re-attribute (the gateway also requires
the refusal to be the plan's terminal one, no descriptor and no switch).
"""
import json, re, sys
from collections import Counter, defaultdict

report, sites_path = sys.argv[1], sys.argv[2]
d = json.load(open(report))
unc = set()
for u in d["units"]:
    for f in u.get("functions", []):
        for x in f.get("diagnostics", []):
            m = re.fullmatch(r"call to uncontracted function `([^`]+)`", x.get("description", ""))
            if m:
                unc.add(m.group(1))
sites = json.load(open(sites_path))
KW = {"sizeof", "unlikely", "likely", "__builtin_expect", "if", "while", "for", "switch", "return"}
cand = defaultdict(list)
rows_all, rows_sole = set(), set()
callees = Counter()
for s in sites:
    if s["position"] in ("return expression", "switch expression", "do-while condition") or not s["expr"]:
        continue
    toks = re.findall(r"[A-Za-z_]\w*|->|\.|\(", s["expr"])
    first = None
    for i in range(len(toks) - 1):
        if toks[i + 1] == "(" and re.fullmatch(r"[A-Za-z_]\w*", toks[i]) and toks[i] not in KW \
                and not (i > 0 and toks[i - 1] in ("->", ".")):
            first = toks[i]
            break
    if first and first in unc:
        key = (s["function"], s["origin"].rsplit(":", 1)[0])
        rows_all.add(key)
        callees[first] += 1
        if s["sole"]:
            rows_sole.add(key)
            cand[key].append(first)
print("uncontracted names in report:", len(unc))
print("candidate condition sites:", sum(callees.values()), "rows", len(rows_all), "sole rows", len(rows_sole))
print("top callees:", callees.most_common(15))
single = {k: v for k, v in cand.items() if len(set(v)) == 1}
print("sole rows whose candidate sites name one callee:", len(single))
