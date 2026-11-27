#!/usr/bin/env python3
"""Card CURRENT-ACCESS-ALIAS-CENSUS: rank shapes and run the call-graph closure.

usage: rank.py SHAPE_DIR OUT_JSON

Reads shape_census.py's functions.jsonl.gz. A function row (one per unit
configuration) is *blocked* by its items; an item is removed when every tag it
carries is in the chosen set S. A row is *freed* by S when all its items are
removed; with the closure it is also freed when the only items left are calls
to run-defined callees that are Safe or already freed (rounds counted). The
figures are upper bounds: a freed function is taken Safe (a discharged
obligation, nothing more); the PREDICTION drafts say which are expected Safe.
"""
import collections
import gzip
import json
import pathlib
import sys

shape_dir, out_path = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
rows = []
with gzip.open(shape_dir / "functions.jsonl.gz", "rt") as handle:
    for line in handle:
        rows.append(json.loads(line))

CANDIDATE_FAMILIES = {"IND", "COND", "RET", "ALIAS"}
ACTIVE = set(CANDIDATE_FAMILIES)
COARSE = len(sys.argv) > 3 and sys.argv[3] == "coarse"


def coarse(tag):
    """expr:<construct>:<position>:<roots> -> expr:<statement|cond|return>:<call-arg|direct>."""
    if not tag.startswith("expr:"):
        return tag
    _, construct, position, _roots = tag.split(":", 3)
    kind = "cond" if construct.startswith("cond") else construct
    return f"expr:{kind}:{'call-arg' if 'call-arg' in position else 'direct'}"


if COARSE:
    for r in rows:
        r["items"] = [[fam, sorted({coarse(t) for t in tags}), detail] for fam, tags, detail in r["items"]]


def is_shape_tag(tag):
    return not (tag.startswith(("fix:", "UNC-")) or tag == "unjoined")


# Rows that can be freed: not Safe (Unknown or Unsafe by the report).
live = [r for r in rows if r["status"] != "Safe"]
unknown = [r for r in rows if r["status"] == "Unknown"]


def row_tags(r):
    """list of (family, tags) with tags as frozenset."""
    return [(fam, frozenset(tags)) for fam, tags, _ in r["items"]]


for r in rows:
    r["it"] = row_tags(r)
    r["shape_tags"] = set().union(*[t for f, t in r["it"] if f in CANDIDATE_FAMILIES and all(is_shape_tag(x) for x in t)]) \
        if r["it"] else set()


def freed(S, population):
    return [r for r in population
            if r["it"] and all(f in ACTIVE and t <= S for f, t in r["it"])]


# Tag statistics.
leaf_count = collections.Counter()
func_touch = collections.defaultdict(set)
fam_of = collections.defaultdict(collections.Counter)
for idx, r in enumerate(rows):
    for fam, tags in r["it"]:
        if fam in CANDIDATE_FAMILIES:
            for t in tags:
                leaf_count[t] += 1
                func_touch[t].add(idx)
                fam_of[t][fam] += 1
alltags = [t for t in leaf_count if is_shape_tag(t)]
idx_unknown = {id(r) for r in unknown}
stats = {}
for t in alltags:
    alone = freed({t}, unknown)
    stats[t] = {"family": dict(fam_of[t]), "leaves": leaf_count[t], "functions": len(func_touch[t]),
                "alone": len(alone)}

# Greedy cover by marginal functions freed (ties by leaves removed, then by rows
# one tag away) over Unknown rows.
chosen, steps = [], []
S = set()
remaining = set(alltags)
base = len(freed(S, unknown))
for _ in range(40):
    best = None
    pool = [r for r in unknown if r["it"] and all(f in CANDIDATE_FAMILIES for f, _ in r["it"])]
    for t in sorted(remaining):
        S2 = S | {t}
        gain = len(freed(S2, pool)) - len(freed(S, pool))
        near = sum(1 for r in pool if (set().union(*[x for _, x in r["it"]]) - S) == {t})
        leaves = leaf_count[t]
        key = (gain, near, leaves)
        if best is None or key > best[0]:
            best = (key, t)
    if best is None or (best[0][0] == 0 and best[0][1] == 0):
        break
    S.add(best[1])
    remaining.discard(best[1])
    chosen.append(best[1])
    steps.append({"tag": best[1], "gain": best[0][0], "cumulative": len(freed(S, pool)), "pool": len(pool)})

# Pair table for the top single tags.
top = sorted(alltags, key=lambda t: -stats[t]["alone"])[:12]
pairs = {}
for i, a in enumerate(top):
    for b in top[i + 1:]:
        pairs[a + " + " + b] = len(freed({a, b}, unknown))

# Family-level sets.
fam_tags = collections.defaultdict(set)
for r in rows:
    for fam, tags in r["it"]:
        if fam in CANDIDATE_FAMILIES:
            fam_tags[fam] |= {t for t in tags if is_shape_tag(t)}
family_sets = {
    "IND": fam_tags["IND"], "ALIAS": fam_tags["ALIAS"], "COND": fam_tags["COND"], "RET": fam_tags["RET"],
    "IND+ALIAS": fam_tags["IND"] | fam_tags["ALIAS"],
    "IND+ALIAS+COND+RET": fam_tags["IND"] | fam_tags["ALIAS"] | fam_tags["COND"] | fam_tags["RET"],
}


def freed_by_families(families, population):
    """Rows whose every item is in one of the given families (any tag)."""
    return [r for r in population if r["it"] and all(f in families for f, _ in r["it"])]


family_freed = {name: len(freed_by_families(set(name.split("+")), unknown)) for name in family_sets}

# Units whose whole blocker set a set covers: every non-Safe row of the unit.
by_unit = collections.defaultdict(list)
for r in rows:
    by_unit[r["i"]].append(r)


def units_covered(S):
    count, names = 0, []
    for i, rs in by_unit.items():
        bad = [r for r in rs if r["status"] != "Safe"]
        if bad and all(r["it"] and all(f in ACTIVE and t <= S for f, t in r["it"]) for r in bad):
            count += 1
            names.append(i)
    return count, names


# ----- closure -----
defined_in = collections.defaultdict(set)
for r in rows:
    defined_in[r["function"]].add(r["i"])
unit_file = {r["i"]: r["file"] for r in rows}
files_of = collections.defaultdict(set)
for name, units in defined_in.items():
    files_of[name] = {unit_file[i] for i in units}


def closure(S, mode):
    """mode 'none' (no callee rule), 'same-unit', 'cross-unit'. Returns (rounds list, final freed rows)."""
    key = {(r["i"], r["function"]): r for r in rows}
    good = {k for k, r in key.items() if r["status"] == "Safe"}
    freed_now = set()
    rounds = []
    changed = True
    while changed:
        changed = False
        new = []
        for r in unknown:
            k = (r["i"], r["function"])
            if k in freed_now or not r["it"]:
                continue
            ok = True
            for fam, tags, detail in ((f, t, d) for f, t, d in r["items"]):
                tags = frozenset(tags)
                if fam in ACTIVE and tags <= S:
                    continue
                if fam == "UNC-RUN" and mode != "none":
                    callee = detail
                    if (r["i"], callee) in key:
                        if (r["i"], callee) in good or (r["i"], callee) in freed_now:
                            continue
                    elif mode == "cross-unit" and len(files_of.get(callee, ())) == 1:
                        units = defined_in[callee]
                        if all(((i, callee) in good or (i, callee) in freed_now) for i in units):
                            continue
                ok = False
                break
            if ok:
                new.append(k)
        if new:
            changed = True
            freed_now.update(new)
            rounds.append(len(new))
    return rounds, freed_now


results = {"rows": len(rows), "unknown_rows": len(unknown), "alone_baseline": len(freed(set(), unknown)),
           "tag_stats": stats, "greedy": steps, "pairs": pairs, "family_freed": family_freed,
           "family_tags": {k: sorted(v) for k, v in family_sets.items()}}
results["closure"] = {}
for label, tags in (("greedy-top3", set(chosen[:3])), ("greedy-top6", set(chosen[:6])),
                    ("greedy-all", set(chosen)), ("IND+ALIAS+COND+RET", family_sets["IND+ALIAS+COND+RET"]),
                    ("IND+ALIAS", family_sets["IND+ALIAS"]), ("none", set())):
    entry = {}
    ACTIVE = set(label.split("+")) if label.startswith(("IND", "ALIAS")) else set(CANDIDATE_FAMILIES)
    for mode in ("none", "same-unit", "cross-unit"):
        rounds, got = closure(tags, mode)
        gotset = set(got)
        units_closed = sum(
            1 for i, rs in by_unit.items()
            if any(r["status"] != "Safe" for r in rs)
            and all(r["status"] == "Safe" or (r["i"], r["function"]) in gotset for r in rs))
        entry[mode] = {"rounds": rounds, "freed": len(got), "units_freed": units_closed}
    entry["direct"] = len(freed(tags, unknown))
    count, _ = units_covered(tags)
    entry["units_covered"] = count
    results["closure"][label] = entry
results["units_covered_by_step"] = {}
S = set()
for s in steps[:15]:
    S.add(s["tag"])
    results["units_covered_by_step"][s["tag"]] = units_covered(S)[0]
ACTIVE = set(CANDIDATE_FAMILIES)
if COARSE:
    BUNDLES = {
        "K1 call-actual access (statement, return, condition)":
            {"expr:statement:call-arg", "expr:return:call-arg", "expr:cond:call-arg"},
        "K2 no typed access (conditions, returns, statements)": {"none"},
        "K3 pointer-valued stores (FIELD-CUSTODY)": {"pointer-valued-store"},
        "K4 formal custody aliases": {"alias:formal-handed-uncontracted", "alias:formal-handed-contracted",
                                     "alias:formal-copied", "alias:formal-not-handed"},
        "K5 local-origin aliases": {"alias:derived-local-unguarded", "alias:guarded-other",
                                   "alias:borrowed-path-unguarded", "alias:contracted-result-unguarded",
                                   "alias:uncontracted-result-unguarded", "alias:non-pointer-name",
                                   "alias:statement-expression-unguarded", "alias:poisoned-by-refusal",
                                   "alias:address-taken", "alias:field-store-owner"},
        "K6 direct-position expressible access": {"expr:statement:direct", "expr:return:direct", "expr:cond:direct"},
        "K7 residual refusal causes": {"union", "aggregate-value", "call", "indirect-call", "cast",
                                       "pointer-arithmetic", "static-local", "statement-expression", "pointer-step",
                                       "root-other", "function-designator", "address-of-base", "volatile-atomic",
                                       "pointer-field-step"},
    }
    names = list(BUNDLES)
    bundle_rows = {}
    for name, tags in BUNDLES.items():
        rounds_same, got_same = closure(tags, "same-unit")
        rounds_x, got_x = closure(tags, "cross-unit")
        bundle_rows[name] = {
            "tags": sorted(tags),
            "leaves": sum(leaf_count[t] for t in tags),
            "functions_touched": len(set().union(*[func_touch[t] for t in tags if t in func_touch])),
            "alone": len(freed(tags, unknown)),
            "units_covered": units_covered(tags)[0],
            "closure_same_unit": {"freed": len(got_same), "rounds": rounds_same},
            "closure_cross_unit": {"freed": len(got_x), "rounds": rounds_x},
        }
    union_all = set().union(*BUNDLES.values())
    for name, tags in BUNDLES.items():
        # marginal: functions freed by all bundles but this one vs all bundles
        rest = union_all - tags
        bundle_rows[name]["marginal_last"] = len(freed(union_all, unknown)) - len(freed(rest, unknown))
    pair = {}
    for i, a in enumerate(names):
        for b in names[i + 1:]:
            pair[a.split()[0] + "+" + b.split()[0]] = len(freed(BUNDLES[a] | BUNDLES[b], unknown))
    results["bundles"] = bundle_rows
    results["bundle_pairs"] = pair
    results["all_bundles_alone"] = len(freed(union_all, unknown))
    # Frozen function lists (rows freed alone by each bundle) for the PREDICTION drafts.
    lists = {}
    for name, tags in BUNDLES.items():
        lists[name.split()[0]] = sorted({(r["file"], r["function"]) for r in freed(tags, unknown)})
    results["alone_lists"] = {k: [list(x) for x in v] for k, v in lists.items()}
    results["alone_lists_unique"] = {k: len(v) for k, v in lists.items()}
out_path.write_text(json.dumps(results, indent=1) + "\n")
print(json.dumps({k: v for k, v in results.items() if k not in ("tag_stats", "family_tags")}, indent=1)[:6000])
