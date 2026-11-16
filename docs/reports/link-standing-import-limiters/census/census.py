#!/usr/bin/env python3
"""LINK-STANDING-IMPORT-LIMITERS census (read-only over the linked run's evidence).

Reproduces rule 4c's per-caller scope from each unit's preprocessed text (clang -E
with the unit's own recorded arguments), checks it against the run's progress
lines, and computes: the member-named homes whose name has no token in the unit
(the stray probes), the scope after leaving them out, and per-function home sets
with the report's blockers.
"""
import json, re, subprocess, sys, os, collections
from concurrent.futures import ThreadPoolExecutor

EVID = "/media/Data/GitHub/quirin-lang/private/current-linked-556460e31"
OUT = os.path.dirname(os.path.abspath(__file__))
BUDGET = 24

report = json.load(open(f"{EVID}/reports-89fa31d21/resource-diagnostics.json"))
assumption = json.load(open(f"{OUT}/assumption.json"))
homes = sorted({h["function"] for h in assumption["imported"]})
home_set = set(homes)

scope_lines = {}
for line in open(f"{OUT}/scope-lines.txt"):
    m = re.search(r"cross-unit scope (\S+): (\d+) reachable home\(s\) \((\d+) named only by member facts\), (\d+) held", line)
    scope_lines[m.group(1)] = dict(reachable=int(m.group(2)), named_only=int(m.group(3)), held=int(m.group(4)), exhausted="exhausted" in line)

# The run's member-named homes: the names refused as undeclared, which the
# progress lines count as 23 named-only homes in a unit spelling none.
named = set()
for unit in report["units"]:
    for r in unit.get("cross_unit_link_refusals", []):
        if "undeclared identifier" in r["reason"]:
            named.add(r["function"])

TOKEN = re.compile(r'"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'|[A-Za-z_][A-Za-z0-9_]*|\S')


def preprocess(unit):
    cc = unit["compile_configuration"]
    cti = cc.get("compile_task_identity")
    if cti is None:
        return None
    args = list(cti["arguments"])
    out = []
    skip = False
    for a in args:
        if skip:
            skip = False
            continue
        if a in ("-o",):
            skip = True
            continue
        if a == "-c":
            continue
        out.append(a)
    out += ["-E", "-P", "-w"]
    proc = subprocess.run(out, cwd=cc["directory"],
                          capture_output=True, text=True)
    if proc.returncode != 0:
        return None
    return proc.stdout


def scan(text):
    toks = [t for t in TOKEN.findall(text)]
    present = set(t for t in toks if re.match(r"[A-Za-z_]", t))
    used = set()
    local = set()
    per_fn = {}
    fileinit = set()
    depth = 0
    init = False
    i = 0
    n = len(toks)
    current = None
    while i < n:
        t = toks[i]
        if t == "{":
            if depth == 0:
                # a function definition: identifier ( ... ) {
                j = i - 1
                if j >= 0 and toks[j] == ")":
                    d = 0
                    k = j
                    while k >= 0:
                        if toks[k] == ")":
                            d += 1
                        elif toks[k] == "(":
                            d -= 1
                            if d == 0:
                                break
                        k -= 1
                    if k > 0 and re.match(r"[A-Za-z_]", toks[k - 1]) and toks[k - 1] not in ("if", "while", "for", "switch"):
                        current = toks[k - 1]
                        local.add(current)
                        per_fn.setdefault(current, set())
            depth += 1
        elif t == "}":
            depth = max(0, depth - 1)
            if depth == 0:
                current = None
        elif t == ";" and depth == 0:
            init = False
        elif t == "=" and depth == 0:
            init = True
        else:
            if depth > 0 or init:
                used.add(t)
                if current is not None and depth > 0:
                    per_fn[current].add(t)
                elif depth == 0 or current is None:
                    fileinit.add(t)
        i += 1
    return present, used, local, per_fn, fileinit


def census(unit):
    key = f"{unit['library']}/{unit['file']}"
    text = preprocess(unit)
    if text is None:
        return key, None
    present, used, local, per_fn, fileinit = scan(text)
    reach = sorted(h for h in homes if h not in local and (h in used or h in named))
    named_only = sorted(h for h in reach if h not in used)
    stray = sorted(h for h in named_only if h not in present)
    after = [h for h in reach if h not in stray]
    # per-function home sets, transitive over local calls
    direct = {f: {h for h in ids if h in home_set and h not in local} for f, ids in per_fn.items()}
    calls = {f: {g for g in ids if g in per_fn and g != f} for f, ids in per_fn.items()}
    closure = {}
    for f in per_fn:
        seen = {f}
        stack = [f]
        acc = set()
        while stack:
            g = stack.pop()
            acc |= direct.get(g, set())
            for h in calls.get(g, ()):
                if h not in seen:
                    seen.add(h)
                    stack.append(h)
        closure[f] = acc
    file_homes = sorted(h for h in fileinit if h in home_set and h not in local)
    return key, dict(reach=reach, named_only=named_only, stray=stray, after=after,
                     per_function={f: sorted(s) for f, s in closure.items() if s},
                     file_scope_homes=file_homes)


def main():
    units = report["units"]
    with ThreadPoolExecutor(max_workers=8) as pool:
        results = dict(pool.map(census, units))
    rows = []
    mismatch = []
    for unit in units:
        key = f"{unit['library']}/{unit['file']}"
        res = results.get(key)
        line = scope_lines.get(key)
        fns = {f["function"]: f for f in unit["functions"]}
        if res is None:
            rows.append(dict(unit=key, preprocessed=False))
            continue
        if line and (line["reachable"] != len(res["reach"]) or line["named_only"] != len(res["named_only"])):
            mismatch.append((key, line, len(res["reach"]), len(res["named_only"])))
        # functions whose every diagnostic is an uncontracted call to a run home
        movable = {}
        co = {}
        for name, f in fns.items():
            if f.get("status") != "Unknown":
                continue
            diags = f.get("diagnostics", [])
            callees = set()
            other = False
            for d in diags:
                m = re.match(r"call to uncontracted function `([^`]+)`", d.get("description", ""))
                if m and m.group(1) in home_set:
                    callees.add(m.group(1))
                else:
                    other = True
            if callees and not other:
                movable[name] = sorted(callees)
            elif callees:
                co[name] = sorted(callees)
        homes_sole = sorted({h for v in movable.values() for h in v})
        rows.append(dict(unit=key, preprocessed=True, run_line=line,
                         reachable=len(res["reach"]), named_only=len(res["named_only"]),
                         stray=res["stray"], after=len(res["after"]),
                         over_budget_before=len(res["reach"]) > BUDGET,
                         over_budget_after=len(res["after"]) > BUDGET,
                         reachable_homes=res["reach"],
                         sole_blocked_functions=movable, co_blocked_functions=co,
                         homes_of_sole_blocked=homes_sole,
                         per_function=res["per_function"], file_scope_homes=res["file_scope_homes"]))
    json.dump(dict(named=sorted(named), homes=len(homes), rows=rows, mismatch=mismatch),
              open(f"{OUT}/census.json", "w"), indent=1)
    print("units", len(rows), "preprocessed", sum(r["preprocessed"] for r in rows))
    print("scope mismatches vs run progress lines", len(mismatch))
    for m in mismatch[:10]:
        print("  ", m)
    pre = [r for r in rows if r["preprocessed"]]
    print("over budget before", sum(r["over_budget_before"] for r in pre), "after stray fix", sum(r["over_budget_after"] for r in pre))
    print("units with stray names", sum(1 for r in pre if r["stray"]), "stray probes", sum(len(r["stray"]) for r in pre))
    for r in pre:
        if r["over_budget_after"]:
            print("  still over:", r["unit"], r["after"], "sole-blocked fns", len(r["sole_blocked_functions"]), "homes of them", len(r["homes_of_sole_blocked"]))


main()
