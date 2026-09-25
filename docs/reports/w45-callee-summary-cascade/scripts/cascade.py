#!/usr/bin/env python3
"""Cascade census of uncontracted-call leaves on Unknown functions.

Usage: cascade.py DIAGNOSTICS.json[.gz] DEFINITION_SITES.json[.gz] OUT.json [SCOPE_CENSUS.jsonl.gz]

Nodes are reported rows (unit, function). An uncontracted-call leaf in unit U
naming callee c resolves, first rule wins, to: the same-unit reported row
(U, c); a header definition carried by U (internal or external linkage, from
DEFINITION_SITES); another pinned unit's unique reported row; several homes;
or no pinned body.

The hypothetical "c validates" makes the body of the resolved node clean. A
function becomes clean when every one of its diagnostics is an uncontracted
call to a clean node, iterated to a fixed point. Only same-unit resolutions
propagate (the only summary route that exists); for an other-unit callee the
figure is reported separately as `unblocked_cross`, an upper bound for a
cross-unit route.
"""
import collections
import gzip
import json
import re
import sys

CALL = re.compile(r"call to uncontracted function `([^`]+)`")
NORM = re.compile(r"`[^`]*`")


def load(path):
    opener = gzip.open if path.endswith(".gz") else open
    with opener(path, "rt") as handle:
        return json.load(handle)


def family(description):
    return NORM.sub("`X`", description)


def main():
    diagnostics, sites_path, out = sys.argv[1:4]
    scope_path = sys.argv[4] if len(sys.argv) > 4 else None
    data = load(diagnostics)
    sites = load(sites_path)["units"]
    nodes = {}
    homes = collections.defaultdict(list)
    for unit in data["units"]:
        key = unit["library"] + "/" + unit["file"]
        for entry in unit["functions"]:
            node = (key, entry["function"])
            nodes[node] = {
                "status": entry["status"],
                "diags": [(d["category"], d["description"]) for d in entry.get("diagnostics") or []],
            }
            homes[entry["function"]].append(node)
    header_body = {}
    if scope_path:
        with gzip.open(scope_path, "rt") as handle:
            rows = [json.loads(line) for line in handle]
        by_selected = {}
        for row in rows:
            by_selected.setdefault(tuple(sorted(row["selected"])), row)
        for unit in data["units"]:
            key = unit["library"] + "/" + unit["file"]
            row = by_selected.get(tuple(sorted(e["function"] for e in unit["functions"])))
            if row:
                for callee in row["callees"]:
                    header_body[("hdr:" + key, callee["name"])] = callee

    def resolve(unit, callee):
        if (unit, callee) in nodes:
            return "same-unit", (unit, callee)
        site = sites.get(unit, {}).get("definitions", {}).get(callee)
        if site is not None and site["file"] != "<stdin>":
            if site["system"]:
                return "system-header", ("hdr:" + unit, callee)
            linkage = "header-internal" if site["static"] and not site["extern"] else "header-external"
            return linkage, ("hdr:" + unit, callee)
        if callee in homes:
            if len(homes[callee]) == 1:
                return "other-unit", homes[callee][0]
            return "other-unit-multi", None
        return "no-body", None

    call_edges = {}
    for node, info in nodes.items():
        edges = []
        for _, description in info["diags"]:
            match = CALL.fullmatch(description)
            if match:
                cls, target = resolve(node[0], match.group(1))
                edges.append((match.group(1), cls, target))
        call_edges[node] = edges

    def propagates(cls, cross):
        return cls == "same-unit" or cls == "header-internal" or (cross and cls == "other-unit")

    def fixed_point(extra_clean, removed_family=None, cross=False, family_scope=None):
        clean = set(n for n, i in nodes.items() if i["status"] == "Safe") | set(extra_clean)
        changed = True
        while changed:
            changed = False
            for node, info in nodes.items():
                if node in clean or info["status"] != "Unknown":
                    continue
                ok = True
                edges = iter(call_edges[node])
                for _, description in info["diags"]:
                    if CALL.fullmatch(description):
                        _, cls, target = next(edges)
                        if target in clean and propagates(cls, cross):
                            continue
                        ok = False
                        break
                    if removed_family is not None and family(description) == removed_family and (
                        family_scope is None or node in family_scope
                    ):
                        continue
                    ok = False
                    break
                if ok:
                    clean.add(node)
                    changed = True
        return clean

    base_clean = fixed_point(())
    unknown = [n for n, i in nodes.items() if i["status"] == "Unknown"]
    anomalies = [n for n in unknown if n in base_clean]

    callee_leaves = collections.Counter()
    callee_callers = collections.defaultdict(set)
    class_of = {}
    for node in unknown:
        for name, cls, target in call_edges[node]:
            key = target if target is not None else (cls + ":" + (node[0] if cls == "no-body" else "*"), name)
            callee_leaves[key] += 1
            callee_callers[key].add(node)
            class_of[key] = cls
    only_calls = [n for n in unknown if call_edges[n] and all(CALL.fullmatch(d) for _, d in nodes[n]["diags"])]

    def body_blockers(key):
        if key in nodes:
            return collections.Counter(family(d) for _, d in nodes[key]["diags"])
        if key in header_body:
            row = header_body[key]
            counter = collections.Counter()
            for blocker in row.get("body_blockers") or []:
                counter[family(blocker[0])] += blocker[1] if len(blocker) > 1 and isinstance(blocker[1], int) else 1
            if not counter:
                counter["<header " + str(row.get("outcome")) + ">"] = 1
            return counter
        return collections.Counter()

    def lost_last_call(clean, cross=False):
        count = 0
        for node in unknown:
            if node in clean or not call_edges[node]:
                continue
            if all(t in clean and propagates(c, cross) for _, c, t in call_edges[node]):
                count += 1
        return count

    base_lost = lost_last_call(base_clean)
    base_clean_cross = fixed_point((), cross=True)
    base_lost_cross = lost_last_call(base_clean_cross, True)
    rows = []
    for key, leaves in callee_leaves.items():
        cls = class_of[key]
        blockers = body_blockers(key)
        row = {
            "callee": key[1],
            "home": key[0],
            "class": cls,
            "leaves": leaves,
            "callers": len(callee_callers[key]),
            "home_status": nodes[key]["status"] if key in nodes else None,
            "body_blockers": dict(blockers.most_common()),
            "body_families": len(blockers),
            "body_leaves": sum(blockers.values()),
            "unblocked": 0,
            "unblocked_cross": 0,
            "lost_last_call": 0,
        }
        if cls in ("same-unit", "header-internal", "other-unit"):
            cross = cls == "other-unit"
            clean = fixed_point([key], cross=cross)
            reference = base_clean_cross if cross else base_clean
            unblocked = len([n for n in unknown if n in clean and n not in reference and n != key])
            row["unblocked_cross" if cross else "unblocked"] = unblocked
            row["lost_last_call"] = lost_last_call(clean, cross) - (base_lost_cross if cross else base_lost)
        rows.append(row)
    rows.sort(key=lambda r: (-(r["unblocked"] + r["unblocked_cross"]), -r["leaves"]))

    callee_bodies = set(k for k in callee_leaves if k in nodes)
    fam_total = collections.Counter()
    for node in unknown:
        for _, d in nodes[node]["diags"]:
            fam_total[family(d)] += 1
    families = []
    for fam, count in fam_total.most_common(40):
        if fam.startswith("call to uncontracted function"):
            continue
        everywhere = fixed_point((), removed_family=fam)
        in_callees = fixed_point((), removed_family=fam, family_scope=callee_bodies)
        families.append({
            "family": fam,
            "leaves_on_unknown": count,
            "leaves_in_callee_bodies": sum(1 for k in callee_bodies for _, d in nodes[k]["diags"] if family(d) == fam),
            "unblocked_removed_everywhere": len([x for x in unknown if x in everywhere and x not in base_clean]),
            "unblocked_removed_in_callee_bodies": len([x for x in unknown if x in in_callees and x not in base_clean]),
        })
    families.sort(key=lambda r: -r["unblocked_removed_in_callee_bodies"])
    classes = collections.Counter()
    for node in unknown:
        for _, cls, _ in call_edges[node]:
            classes[cls] += 1
    result = {
        "unknown_functions": len(unknown),
        "unknown_with_call_leaves": sum(1 for n in unknown if call_edges[n]),
        "only_calls": len(only_calls),
        "only_one_callee": sum(1 for n in only_calls if len({c for c, _, _ in call_edges[n]}) == 1),
        "leaves": sum(len(call_edges[n]) for n in unknown),
        "leaf_classes": dict(classes.most_common()),
        "anomalies_clean_at_base": [list(a) for a in anomalies],
        "cross_unit_route_existing_safe_bodies": {
            "unblocked": len([n for n in unknown if n in base_clean_cross and n not in base_clean]),
            "lost_last_call": base_lost_cross - base_lost,
            "functions": sorted("%s:%s" % n for n in unknown if n in base_clean_cross and n not in base_clean),
        },
        "families": families,
        "callees": rows,
    }
    with open(out, "w") as handle:
        json.dump(result, handle, indent=1)
    print(json.dumps({k: v for k, v in result.items() if k not in ("callees", "families")}, indent=1))
    for f in families[:20]:
        print(f)
    for r in rows[:50]:
        print(r["unblocked"], r["unblocked_cross"], r["lost_last_call"], r["leaves"], r["class"], r["home"],
              r["callee"], r["body_leaves"], list(r["body_blockers"].items())[:4])


if __name__ == "__main__":
    main()
