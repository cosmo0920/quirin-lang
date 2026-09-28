#!/usr/bin/env python3
"""W47-CALLEE-CLOSURE-R2: callee closure and external ranking over one corpus run.

Usage: closure_r2.py DIAGNOSTICS(.json|.json.gz) OUT_JSON [--previous RANKING_JSON]

Round 2 of plan P4 (docs/plans/unsupported-coverage/README.md). P1's closure
(w46-access-path-census/scripts/predict.py) ran over a *prediction* of which
access leaves P2 and P3 would remove, so it needed the census streams. After
P3 those leaves are measured, so the closure is recomputed over the measured
diagnostics alone, with P1's callee classes and variants:

* a leaf `call to uncontracted function `N`` or `nested call `N` inside a
  modeled call ...` is a *call blocker* on N; every other leaf, in every
  category, is a *non-call blocker*;
* N is `external` when no unit lists a function of that name (analysed or
  `source_functions`, the W46 test), `same-unit` when the caller's unit
  defines it, `cross-unit` when exactly one other unit defines it, and
  `cross-unit-ambiguous` when several do (a `static` name in several units;
  the diagnostics carry no storage class, so a single definition elsewhere is
  read as the external-linkage body the call binds, a stated premise);
* a function is *call-only* when it is Unknown and every leaf is a call
  blocker.

Closure (P1's variants, over the call-only functions; a `nested call` leaf
is refused for its position, so no summary serves it and it always blocks):

* `same-unit`: a call is served by a same-unit callee that is Safe (the
  existing summary rule, W45-CALLEE-SUMMARY-CASCADE). At a measured baseline
  this should free nothing, and a function it frees is a finding;
* `cross-unit`: also by a Safe callee defined in one other unit (no rule does
  this today, W44-CALLEE-CROSS-UNIT-SUMMARY);
* `ceiling`: `cross-unit`, with every freed function taken Safe in the next
  round (P1's `cross-unit-ceiling`). A freed function's verdict is not
  predicted: a served call imports the callee's custody duties.

Each variant iterates to a fixpoint; round r frees the functions whose every
callee is served by a function Safe at the base or freed in a round < r.

External ranking (W46 rank_external.py, extended):

* `alone`: call-only functions whose every call blocker names N (W46's test,
  now counting a `nested call` leaf naming N as well, the W46 `pack_string`
  miss); `alone_unc` keeps W46's exact test (uncontracted-call leaves only);
* `plus_callee`: call-only functions with exactly two distinct callees, N
  and one other (reported with its class);
* `plus_leaf`: functions whose leaves are call blockers naming N plus exactly
  one non-call leaf (reported with its description template);
* `sites`: uncontracted-call leaves naming N.

`--previous` names a ranking JSON of the same shape (or W46's list form) to
compare the alone column with.
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


def load(path):
    opener = gzip.open if str(path).endswith(".gz") else open
    with opener(path, "rt") as handle:
        return json.load(handle)


def callee_of(leaf):
    if leaf["category"] != "unsupported":
        return None, None
    match = UNC.fullmatch(leaf["description"])
    if match:
        return match.group(1), "unc"
    match = NESTED.fullmatch(leaf["description"])
    if match:
        return match.group(1), "nested"
    return None, None


def analyse(data):
    defined_in = collections.defaultdict(set)
    status = {}
    for unit in data["units"]:
        for entry in unit["functions"]:
            defined_in[entry["function"]].add(unit["file"])
            status[(unit["file"], entry["function"])] = entry["status"]
        for name in unit.get("source_functions") or []:
            defined_in[name].add(unit["file"])

    def classify(file, name):
        homes = defined_in.get(name, set())
        if not homes:
            return "external", None
        if file in homes:
            return "same-unit", (file, name)
        if len(homes) == 1:
            return "cross-unit", (next(iter(homes)), name)
        return "cross-unit-ambiguous", None

    functions = {}
    sites = collections.Counter()
    for unit in data["units"]:
        for entry in unit["functions"]:
            key = (unit["file"], entry["function"])
            calls = collections.defaultdict(lambda: collections.Counter())
            others = []
            for leaf in entry.get("diagnostics") or []:
                name, kind = callee_of(leaf)
                if name is None:
                    others.append(leaf)
                    continue
                calls[name][kind] += 1
                if kind == "unc" and not defined_in.get(name):
                    sites[name] += 1
            functions[key] = {
                "library": unit.get("library"),
                "status": entry["status"],
                "calls": {name: dict(kinds) for name, kinds in calls.items()},
                "others": others,
            }
    return defined_in, status, classify, functions, sites


def closure(functions, status, classify, variant):
    safe = {key for key, value in status.items() if value == "Safe"}
    call_only = [key for key, entry in functions.items()
                 if entry["status"] == "Unknown" and entry["calls"] and not entry["others"]]
    freed = {}
    rounds = []
    round_number = 0
    while True:
        added = []
        for key in call_only:
            if key in freed:
                continue
            served = True
            for name in entry_names(functions[key]):
                if "nested" in functions[key]["calls"][name]:
                    # A nested call is refused for its position; a summary does
                    # not serve it (snappy.c::hash waits on a Safe same-unit
                    # get_unaligned32), only a declared contract can.
                    served = False
                    break
                kind, target = classify(key[0], name)
                if kind == "same-unit" and target in safe:
                    continue
                if kind == "cross-unit" and variant != "same-unit" and target in safe:
                    continue
                served = False
                break
            if served:
                freed[key] = round_number
                added.append(key)
        rounds.append({"round": round_number, "freed": len(added)})
        if not added or variant != "ceiling":
            break
        safe |= set(added)
        round_number += 1
    return freed, rounds


def entry_names(entry):
    return sorted(entry["calls"])


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("diagnostics")
    parser.add_argument("out")
    parser.add_argument("--previous")
    args = parser.parse_args()
    data = load(args.diagnostics)
    defined_in, status, classify, functions, sites = analyse(data)

    # ---- call-only population by callee class --------------------------------
    population = collections.Counter()
    waits = collections.Counter()
    waits_alone = collections.defaultdict(list)
    call_only = {}
    for key, entry in functions.items():
        if entry["status"] != "Unknown" or not entry["calls"] or entry["others"]:
            continue
        classes = sorted({classify(key[0], name)[0] for name in entry["calls"]})
        call_only[key] = classes
        if classes == ["external"]:
            population["external only"] += 1
        elif "external" in classes:
            population["external and in-corpus"] += 1
        else:
            population["in-corpus only"] += 1
        for name in entry["calls"]:
            kind, target = classify(key[0], name)
            if kind != "external":
                waits[(name, kind)] += 1
        if len(entry["calls"]) == 1:
            name = next(iter(entry["calls"]))
            kind, target = classify(key[0], name)
            if kind != "external":
                waits_alone[(name, kind)].append("::".join(key))

    def callee_state(name):
        homes = sorted(defined_in.get(name, set()))
        states = []
        for home in homes:
            entry = functions.get((home, name))
            blockers = collections.Counter()
            if entry is not None:
                for callee, kinds in entry["calls"].items():
                    blockers[f"call {callee}"] += sum(kinds.values())
                for leaf in entry["others"]:
                    blockers[leaf["category"] + ": " + TEMPLATE.sub("`X`", leaf["description"])] += 1
            states.append({"unit": home, "status": status.get((home, name), "not analysed"),
                           "blockers": dict(blockers.most_common(4))})
        return states

    in_corpus_top = []
    for (name, kind), count in sorted(waits_alone.items(), key=lambda item: (-len(item[1]), item[0])):
        in_corpus_top.append({"callee": name, "class": kind, "alone": len(count),
                              "homes": callee_state(name), "functions": sorted(count)})

    variants = {}
    for variant in ("same-unit", "cross-unit", "ceiling"):
        freed, rounds = closure(functions, status, classify, variant)
        by_library = collections.Counter(functions[key]["library"] for key in freed)
        variants[variant] = {
            "freed": len(freed),
            "rounds": rounds,
            "by_library": dict(sorted(by_library.items())),
            "functions": sorted(f"{key[0]}::{key[1]}@{round_number}" for key, round_number in freed.items()),
        }

    # ---- external ranking -----------------------------------------------------
    alone = collections.defaultdict(list)
    alone_unc = collections.defaultdict(list)
    plus_callee = collections.defaultdict(list)
    plus_leaf = collections.defaultdict(list)
    co_blocked = collections.defaultdict(list)
    for key, entry in functions.items():
        if entry["status"] != "Unknown" or not entry["calls"]:
            continue
        label = "::".join(key)
        names = sorted(entry["calls"])
        external = [name for name in names if classify(key[0], name)[0] == "external"]
        if not external:
            continue
        if not entry["others"]:
            if len(names) == 1:
                alone[names[0]].append(label)
                if set(entry["calls"][names[0]]) == {"unc"}:
                    alone_unc[names[0]].append(label)
            elif len(names) == 2:
                for name in external:
                    other = names[1] if names[0] == name else names[0]
                    plus_callee[name].append({"function": label, "other": other,
                                              "other_class": classify(key[0], other)[0]})
            for name in external:
                if len(names) > 1:
                    co_blocked[name].append(label)
        elif len(entry["others"]) == 1 and len(names) == 1:
            leaf = entry["others"][0]
            plus_leaf[names[0]].append({"function": label, "leaf": leaf["category"] + ": "
                                        + TEMPLATE.sub("`X`", leaf["description"]),
                                        "line": leaf["line"]})
    rows = []
    for name in set(sites) | set(alone) | set(plus_callee) | set(plus_leaf):
        if defined_in.get(name):
            continue
        rows.append({
            "callee": name,
            "alone": len(alone[name]),
            "alone_unc": len(alone_unc[name]),
            "plus_callee": len(plus_callee[name]),
            "plus_leaf": len(plus_leaf[name]),
            "co_blocked": len(co_blocked[name]),
            "sites": sites[name],
            "alone_functions": sorted(alone[name]),
            "plus_callee_functions": sorted(plus_callee[name], key=lambda r: r["function"]),
            "plus_leaf_functions": sorted(plus_leaf[name], key=lambda r: r["function"]),
        })
    rows.sort(key=lambda r: (-r["alone"], -(r["plus_callee"] + r["plus_leaf"]), -r["sites"], r["callee"]))

    comparison = None
    if args.previous:
        previous = load(args.previous)
        previous_rows = previous["external_ranking"] if isinstance(previous, dict) else previous
        before = {row["callee"]: row for row in previous_rows}
        comparison = []
        for row in rows:
            old = before.get(row["callee"])
            old_alone = set(old.get("alone_functions", [])) if old else set()
            new_alone = set(row["alone_functions"])
            if old_alone != new_alone:
                comparison.append({"callee": row["callee"], "gained": sorted(new_alone - old_alone),
                                   "lost": sorted(old_alone - new_alone)})
        for name, old in before.items():
            if name not in {row["callee"] for row in rows} and old.get("alone_functions"):
                comparison.append({"callee": name, "gained": [], "lost": sorted(old["alone_functions"])})

    tally = collections.Counter(entry["status"] for entry in functions.values())
    document = {
        "schema": "qcore.w47-callee-closure-r2.v1",
        "functions": dict(tally),
        "call_only_population": dict(population),
        "call_only_total": sum(population.values()),
        "in_corpus_waits_alone": in_corpus_top,
        "in_corpus_waits_any": [{"callee": n, "class": k, "functions": c}
                                for (n, k), c in waits.most_common()],
        "closure": variants,
        "external_callees": len(rows),
        "external_sites": sum(sites.values()),
        "functions_alone_external": sum(r["alone"] for r in rows),
        "functions_alone_unc_external": sum(r["alone_unc"] for r in rows),
        "external_ranking": rows,
        "comparison": comparison,
    }
    pathlib.Path(args.out).write_text(json.dumps(document, indent=1, sort_keys=True) + "\n")

    print(f"functions {dict(tally)}")
    print(f"call-only Unknown functions: {sum(population.values())} {dict(population)}")
    for variant, value in variants.items():
        print(f"closure {variant}: freed {value['freed']} rounds {value['rounds']}")
    print("in-corpus callees blocking call-only functions alone (top 15):")
    for row in in_corpus_top[:15]:
        homes = ", ".join(f"{h['unit']}:{h['status']}" for h in row["homes"])
        print(f"  {row['alone']:3} {row['callee']} [{row['class']}] home {homes}")
    print(f"external callees {len(rows)}, sites {sum(sites.values())}, "
          f"functions alone {document['functions_alone_external']} "
          f"(uncontracted-call leaves only: {document['functions_alone_unc_external']})")
    print(f"{'alone':>5} {'unc':>4} {'+call':>5} {'+leaf':>5} {'co':>4} {'sites':>5}  callee")
    for row in rows[:40]:
        print(f"{row['alone']:5} {row['alone_unc']:4} {row['plus_callee']:5} {row['plus_leaf']:5} "
              f"{row['co_blocked']:4} {row['sites']:5}  {row['callee']}")
    if comparison is not None:
        print(f"alone-set changes against --previous: {len(comparison)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
