#!/usr/bin/env python3
"""rank.py REPORT.json OUT.json [--source-root DIR] [--top N] -- card
REFCOUNT-RESOURCE-KIND (adapted from SLOT-MOVE-AND-INTERIOR-BORROW, from W43-ABI-ENUM-AND-CONST-PP, from CALLEE-RETENTION-CLASS1), task 1: the Unknown functions blocked by
uncontracted calls alone, and the callees ranked by functions decided alone.

Population (CG10-INDIRECT-ACCESS-SOLE-RESIDUALS' `decides.py`, corpus arm):
every `Unknown` row of REPORT whose every leaf reads exactly
``call to uncontracted function `F` ``. A function that also keeps any other
leaf (an access refusal over an actual neither reader recognises, an
unresolved identity, a branch condition, ...) is not in it, so a function
that also needs an unread actual is never counted (rule 12k names a callee
only when the borrowed reading of every actual admits the statement).

Per callee: `alone` (the function's only blocking callee), `pair` (the
callee plus exactly one other), `touched`; the fixed point when callees are
lifted cumulatively in rank order (the ceiling a contract per callee
reaches, every other obligation of the freed statements assumed to
discharge); and where the callee's body is (`kind`): `same-unit` / `cross-unit`
(a reported row of that name in a calling unit / only in another unit, with
its status and leading leaf families), `header-inline` (a static or inline
header definition under DIR, lexical), else `external`.
"""
import argparse
import collections
import json
import pathlib
import re
import subprocess

UNCONTRACTED = re.compile(r"^call to uncontracted function `([^`]*)`$")
TEMPLATE = re.compile(r"`[^`]*`")


def callees_of(leaves):
    names = set()
    for leaf in leaves:
        match = UNCONTRACTED.match(leaf.get("description", ""))
        if not match:
            return None
        names.add(match.group(1))
    return names


def header_definitions(root, name):
    if root is None:
        return []
    pattern = r"^[^;#/]*\b%s\s*\([^;]*$" % re.escape(name)
    output = subprocess.run(["grep", "-rnE", "--include=*.h", "-e", pattern, str(root)],
                            capture_output=True, text=True, check=False).stdout
    found = []
    for line in output.splitlines():
        path, number, text = line.split(":", 2)
        if "static" in text or "inline" in text.lower():
            found.append("%s:%s" % (path[len(str(root)) + 1:], number))
    return found


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("report")
    parser.add_argument("out")
    parser.add_argument("--source-root")
    parser.add_argument("--top", type=int, default=80)
    args = parser.parse_args()
    report = json.loads(pathlib.Path(args.report).read_text())
    pure, homes = {}, collections.defaultdict(list)
    statuses = collections.Counter()
    for unit in report["units"]:
        for function in unit["functions"]:
            statuses[function["status"]] += 1
            leaves = function.get("diagnostics") or []
            families = collections.Counter(
                TEMPLATE.sub("`_`", d.get("description", "")) for d in leaves)
            homes[function["function"]].append({
                "unit": unit["file"], "status": function["status"],
                "families": dict(families.most_common(4))})
            if function["status"] != "Unknown" or not leaves:
                continue
            names = callees_of(leaves)
            if names:
                pure[(unit["file"], function["function"])] = names
    alone, pair, touched = collections.Counter(), collections.Counter(), collections.Counter()
    units, freed = collections.defaultdict(set), collections.defaultdict(list)
    for key, names in pure.items():
        for name in names:
            touched[name] += 1
            units[name].add(key[0])
            if len(names) == 2:
                pair[name] += 1
        if len(names) == 1:
            name = next(iter(names))
            alone[name] += 1
            freed[name].append("%s::%s" % key)
    order = sorted(touched, key=lambda n: (-alone[n], -touched[n], n))
    lifted, rows = set(), []
    for name in order:
        lifted.add(name)
        rows.append({"callee": name, "alone": alone[name], "pair": pair[name],
                     "touched": touched[name],
                     "cumulative": sum(1 for names in pure.values() if names <= lifted),
                     "freed": sorted(freed[name])})
    root = pathlib.Path(args.source_root) if args.source_root else None
    for row in rows[: args.top]:
        name = row["callee"]
        found = homes.get(name, [])
        if found:
            kind = "same-unit" if {h["unit"] for h in found} & units[name] else "cross-unit"
        else:
            headers = header_definitions(root, name)
            kind = "header-inline" if headers else "external"
            found = [{"header": h} for h in headers[:3]]
        row["kind"] = kind
        row["homes"] = found[:4]
    record = {
        "report": args.report,
        "statuses": dict(statuses),
        "population": len(pure),
        "alone_total": sum(alone.values()),
        "ranking": rows,
    }
    pathlib.Path(args.out).write_text(json.dumps(record, indent=1) + "\n")
    print("statuses %s; population %d; decided alone by one callee %d" % (
        dict(statuses), len(pure), record["alone_total"]))
    for row in rows[: args.top]:
        status = ",".join(sorted({h.get("status", "-") for h in row.get("homes", [])})) or "-"
        print("  %-46s %3d %3d %3d %4d %-13s %s" % (row["callee"], row["alone"], row["pair"],
                                                   row["touched"], row["cumulative"],
                                                   row.get("kind", ""), status))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
