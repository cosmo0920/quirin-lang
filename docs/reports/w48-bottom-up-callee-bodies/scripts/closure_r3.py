#!/usr/bin/env python3
"""W48-CALLEE-CLOSURE-R3: rank every callee that blocks an Unknown function.

Usage: closure_r3.py DIAGNOSTICS OUT_JSON [--route NAME] [--decisions DECISIONS_JSON]

Round 3 of plan P4 (docs/plans/unsupported-coverage/README.md), extending
round 2's closure_r2.py from the external callees to every callee:

* a leaf `call to uncontracted function `N`` or `nested call `N` inside a
  modeled call ...` is a *call blocker* on N; every other leaf, of every
  category, is a *non-call blocker*. A function's rows (the system and slice
  routes carry one row per compile configuration) are merged by
  (unit file, function): the union of their leaves, the join of their status.
* N's class, relative to the calling unit (round 2's test, W46's for
  `external`):
  - (i)  `same-unit`: the calling unit defines N (its own row's status and
         blockers are reported);
  - (ii) `cross-unit`: N is defined in exactly one other unit of the route
         (`cross-unit-ambiguous` when several: a `static` name defined in
         several units, which no cross-unit route could serve);
  - (iii) `external`: no unit of the route defines N;
  - (iv) `decision`: N has no body in the route and is named in
         DECISIONS_JSON, the eight classes of W47-CALLEE-CLOSURE-R2 §6 (a
         name with a body in the route keeps its location class: its body,
         not a declaration, is what would decide it);
  - (v)  `builtin`: N is a compiler builtin (`__builtin_*`, `__atomic_*`,
         `__sync_*`, `__c11_atomic_*`).
* `alone`: Unknown functions whose every blocker is a call blocker naming N;
  `plus_callee`: whose blockers are call blockers naming N and exactly one
  other callee; `plus_leaf`: call blockers naming N and exactly one non-call
  leaf; `sites`: call-blocker leaves naming N; `functions`: Unknown functions
  with any call blocker naming N.

Closure over the call-only Unknown functions (every blocker a call blocker),
iterated to a fixpoint (round r frees the functions every callee of which is
served by a function Safe at the base or freed in a round < r). Since
CG05-NESTED-CALL-EFFECTS (gateway 10y) a nested call is applied through any
contract in force, a validated summary included, so a `nested call` leaf is
served exactly as an uncontracted-call leaf is (round 2 read it as always
blocking, which was right before 10y):

* `same-unit`: the existing summary rule, a Safe same-unit callee;
* `cross-unit`: also a Safe callee defined in one other unit;
* `ceiling`: `cross-unit`, every freed function taken Safe next round;
* `class-i-safe`: `ceiling`, with every class-(i) and class-(ii) callee's own
  body taken Safe from the start (the brief's hypothetical: how far the
  bottom-up closure reaches if the in-corpus callees' own blockers went).
  Its complement names, per in-corpus callee, the leaves that keep its own
  body Unknown.
"""

import argparse
import collections
import gzip
import json
import pathlib
import re
import sys

UNC = re.compile(r"call to uncontracted function `([^`]+)`")
NESTED = re.compile(r"nested call `([^`]+)` inside a modeled call is outside the resource projection")
TEMPLATE = re.compile(r"`[^`]*`")
BUILTIN = re.compile(r"^(__builtin_|__atomic_|__sync_|__c11_atomic_)")
ORDER = {"Safe": 0, "Pending": 1, "Unknown": 2, "Unsafe": 3}


def load(path):
    path = pathlib.Path(path)
    if path.is_dir():
        path = path / "resource-diagnostics.json"
    opener = gzip.open if str(path).endswith(".gz") else open
    with opener(path, "rt") as handle:
        return json.load(handle)


def callee_of(leaf):
    if leaf.get("category") != "unsupported":
        return None, None
    match = UNC.fullmatch(leaf["description"])
    if match:
        return match.group(1), "unc"
    match = NESTED.fullmatch(leaf["description"])
    if match:
        return match.group(1), "nested"
    return None, None


def family(leaf):
    return leaf["category"] + ": " + TEMPLATE.sub("`X`", leaf.get("description") or "")


def analyse(data):
    defined_in = collections.defaultdict(set)
    functions = {}
    for unit in data["units"]:
        for name in unit.get("source_functions") or []:
            defined_in[name].add(unit["file"])
        for entry in unit["functions"]:
            key = (unit["file"], entry["function"])
            defined_in[entry["function"]].add(unit["file"])
            record = functions.setdefault(key, {
                "library": unit.get("library"), "status": entry["status"],
                "calls": collections.defaultdict(collections.Counter), "others": []})
            if ORDER.get(entry["status"], 2) > ORDER.get(record["status"], 2):
                record["status"] = entry["status"]
            for leaf in entry.get("diagnostics") or []:
                name, kind = callee_of(leaf)
                if name is None:
                    record["others"].append(leaf)
                else:
                    record["calls"][name][kind] += 1
    return defined_in, functions


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("diagnostics")
    parser.add_argument("out")
    parser.add_argument("--route", default="corpus")
    parser.add_argument("--decisions")
    args = parser.parse_args()
    data = load(args.diagnostics)
    defined_in, functions = analyse(data)
    decisions, prefixes = {}, []
    if args.decisions:
        for klass, spec in json.loads(pathlib.Path(args.decisions).read_text())["classes"].items():
            for name in spec["callees"]:
                decisions[name] = klass
            prefixes += [(prefix, klass) for prefix in spec.get("prefixes", [])]
    for name in list(defined_in) + [n for f in functions.values() for n in f["calls"]]:
        if name not in decisions and not defined_in.get(name):
            for prefix, klass in prefixes:
                if name.startswith(prefix):
                    decisions[name] = klass
    status = {key: value["status"] for key, value in functions.items()}

    def location(file, name):
        homes = defined_in.get(name, set())
        if not homes:
            return "external", None
        if file in homes:
            return "same-unit", (file, name)
        if len(homes) == 1:
            return "cross-unit", (next(iter(homes)), name)
        return "cross-unit-ambiguous", None

    def klass(file, name):
        where, _ = location(file, name)
        if BUILTIN.match(name):
            return "v-builtin"
        if name in decisions and where == "external":
            return "iv-decision"
        return {"same-unit": "i-same-unit", "cross-unit": "ii-cross-unit",
                "cross-unit-ambiguous": "ii-cross-unit-ambiguous",
                "external": "iii-external"}[where]

    unknown = {k: v for k, v in functions.items() if v["status"] == "Unknown"}

    # ---- per-callee ranking ---------------------------------------------------
    rows = {}

    def row(name, file):
        where, target = location(file, name)
        record = rows.get(name)
        if record is None:
            record = rows[name] = {
                "callee": name, "classes": collections.Counter(), "locations": collections.Counter(),
                "decision": decisions.get(name), "alone": [], "plus_callee": [], "plus_leaf": [],
                "functions": 0, "sites": 0, "nested_sites": 0}
        record["classes"][klass(file, name)] += 1
        record["locations"][where] += 1
        return record

    for key, entry in unknown.items():
        label = "::".join(key)
        names = sorted(entry["calls"])
        for name in names:
            record = row(name, key[0])
            record["functions"] += 1
            record["sites"] += entry["calls"][name].get("unc", 0)
            record["nested_sites"] += entry["calls"][name].get("nested", 0)
        if not names:
            continue
        if not entry["others"]:
            if len(names) == 1:
                rows[names[0]]["alone"].append(label)
            elif len(names) == 2:
                for name in names:
                    other = names[1] if names[0] == name else names[0]
                    rows[name]["plus_callee"].append({"function": label, "other": other,
                                                      "other_class": klass(key[0], other)})
        elif len(entry["others"]) == 1 and len(names) == 1:
            leaf = entry["others"][0]
            rows[names[0]]["plus_leaf"].append({"function": label, "leaf": family(leaf),
                                                "line": leaf.get("line"),
                                                "origin": leaf.get("origin")})

    def home_state(name):
        states = []
        for home in sorted(defined_in.get(name, set())):
            entry = functions.get((home, name))
            blockers = collections.Counter()
            if entry is not None:
                for callee, kinds in entry["calls"].items():
                    blockers[f"call {callee}"] += sum(kinds.values())
                for leaf in entry["others"]:
                    blockers[family(leaf)] += 1
            states.append({"unit": home,
                           "status": entry["status"] if entry else "not analysed",
                           "blockers": dict(blockers.most_common(6)),
                           "blocker_leaves": sum(blockers.values())})
        return states

    ranking = []
    for name, record in rows.items():
        klasses = record["classes"]
        main_class = klasses.most_common(1)[0][0]
        ranking.append({
            "callee": name, "class": main_class, "classes": dict(klasses),
            "locations": dict(record["locations"]), "decision": record["decision"],
            "alone": len(record["alone"]), "plus_callee": len(record["plus_callee"]),
            "plus_leaf": len(record["plus_leaf"]),
            "freed_with_one_other": len(record["alone"]) + len(record["plus_callee"]) + len(record["plus_leaf"]),
            "functions": record["functions"], "sites": record["sites"],
            "nested_sites": record["nested_sites"],
            "homes": home_state(name) if record["locations"].keys() - {"external"} else [],
            "alone_functions": sorted(record["alone"]),
            "plus_callee_functions": sorted(record["plus_callee"], key=lambda r: r["function"]),
            "plus_leaf_functions": sorted(record["plus_leaf"], key=lambda r: r["function"]),
        })
    ranking.sort(key=lambda r: (-r["alone"], -(r["plus_callee"] + r["plus_leaf"]), -r["functions"], r["callee"]))

    by_class = collections.defaultdict(lambda: collections.Counter())
    for r in ranking:
        c = by_class[r["class"]]
        c["callees"] += 1
        c["alone"] += r["alone"]
        c["plus_one"] += r["plus_callee"] + r["plus_leaf"]
        c["sites"] += r["sites"] + r["nested_sites"]
        c["callees_alone_ge_2"] += r["alone"] >= 2
        c["callees_alone_ge_1"] += r["alone"] >= 1

    # ---- call-only population -------------------------------------------------
    population = collections.Counter()
    for key, entry in unknown.items():
        if not entry["calls"] or entry["others"]:
            continue
        classes = sorted({klass(key[0], name) for name in entry["calls"]})
        population[" + ".join(classes)] += 1

    # ---- closure --------------------------------------------------------------
    call_only = [key for key, entry in unknown.items() if entry["calls"] and not entry["others"]]

    def closure(variant):
        safe = {key for key, value in status.items() if value == "Safe"}
        freed, rounds, number = {}, [], 0
        while True:
            added = []
            for key in call_only:
                if key in freed:
                    continue
                served = True
                for name, kinds in functions[key]["calls"].items():
                    where, target = location(key[0], name)
                    if variant == "class-i-safe" and where in ("same-unit", "cross-unit") \
                            and klass(key[0], name) in ("i-same-unit", "ii-cross-unit"):
                        continue
                    if where == "same-unit" and target in safe:
                        continue
                    if where == "cross-unit" and variant != "same-unit" and target in safe:
                        continue
                    served = False
                    break
                if served:
                    freed[key] = number
                    added.append(key)
            rounds.append(len(added))
            if not added or variant not in ("ceiling", "class-i-safe"):
                break
            safe |= set(added)
            number += 1
        return {"freed": len(freed), "rounds": rounds,
                "functions": sorted(f"{k[0]}::{k[1]}@{r}" for k, r in freed.items())}

    variants = {v: closure(v) for v in ("same-unit", "cross-unit", "ceiling", "class-i-safe")}

    # ---- the eight decision classes (class (v) is class 6) ---------------------
    decision_table = collections.defaultdict(lambda: {"callees": [], "alone": set(), "plus_one": set(),
                                                     "functions": set(), "sites": 0})
    for key, entry in unknown.items():
        label = "::".join(key)
        for name, kinds in entry["calls"].items():
            k = klass(key[0], name)
            if k == "v-builtin":
                group = "6-compiler-builtins-without-an-owning-crate"
            elif k == "iv-decision":
                group = decisions[name]
            else:
                continue
            cell = decision_table[group]
            if name not in cell["callees"]:
                cell["callees"].append(name)
            cell["functions"].add(label)
            cell["sites"] += sum(kinds.values())
            if not entry["others"] and len(entry["calls"]) == 1:
                cell["alone"].add(label)
            elif (not entry["others"] and len(entry["calls"]) == 2) or \
                    (len(entry["others"]) == 1 and len(entry["calls"]) == 1):
                cell["plus_one"].add(label)
    # Functions every one of whose blockers is a call to some callee of the class.
    for group, cell in decision_table.items():
        members = set(cell["callees"])
        cell["class_alone"] = sorted("::".join(key) for key, entry in unknown.items()
                                     if entry["calls"] and not entry["others"]
                                     and set(entry["calls"]) <= members)
    decision_table = {group: {"callees": sorted(cell["callees"]), "alone": sorted(cell["alone"]),
                              "plus_one": sorted(cell["plus_one"]), "class_alone": cell["class_alone"],
                              "functions": len(cell["functions"]), "sites": cell["sites"]}
                      for group, cell in sorted(decision_table.items())}

    tally = collections.Counter(entry["status"] for entry in functions.values())
    document = {
        "schema": "qcore.w48-callee-closure-r3.v1",
        "route": args.route,
        "functions": dict(tally),
        "unknown_with_call_blocker": sum(1 for e in unknown.values() if e["calls"]),
        "call_only_total": len(call_only),
        "call_only_population": dict(population.most_common()),
        "by_class": {k: dict(v) for k, v in sorted(by_class.items())},
        "closure": variants,
        "decision_table": decision_table,
        "ranking": ranking,
    }
    pathlib.Path(args.out).write_text(json.dumps(document, indent=1, sort_keys=True) + "\n")

    print(f"route {args.route}: functions {dict(tally)}")
    print(f"Unknown with a call blocker: {document['unknown_with_call_blocker']}; call-only: {len(call_only)}")
    for label, count in population.most_common():
        print(f"  {count:5}  {label}")
    for variant, value in variants.items():
        print(f"closure {variant}: freed {value['freed']} rounds {value['rounds']}")
    print("by class: callees / alone / plus-one / sites / callees alone>=2")
    for k, v in sorted(by_class.items()):
        print(f"  {k:26} {v['callees']:5} {v['alone']:5} {v['plus_one']:5} {v['sites']:6} {v['callees_alone_ge_2']:4}")
    print("decision classes: callees / alone (one callee) / freed by the whole class / plus-one / functions / sites")
    for group, cell in decision_table.items():
        print(f"  {group:48} {len(cell['callees']):4} {len(cell['alone']):4} {len(cell['class_alone']):4} "
              f"{len(cell['plus_one']):4} {cell['functions']:5} {cell['sites']:5}")
    print(f"{'alone':>5} {'+call':>5} {'+leaf':>5} {'fns':>4} {'sites':>5}  class  callee  [home status]")
    for r in ranking[:60]:
        homes = ",".join(f"{h['status']}" for h in r["homes"])
        print(f"{r['alone']:5} {r['plus_callee']:5} {r['plus_leaf']:5} {r['functions']:4} "
              f"{r['sites'] + r['nested_sites']:5}  {r['class']:14} {r['callee']}  [{homes}]")
    return 0


if __name__ == "__main__":
    sys.exit(main())
