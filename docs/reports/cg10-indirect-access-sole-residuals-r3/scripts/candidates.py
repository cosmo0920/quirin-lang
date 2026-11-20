#!/usr/bin/env python3
"""candidates.py PREMISES.jsonl.gz CONTRACTED.json UNITS_DIR -- CG10 r3: the census
rows the two predicted readings could reach.

* rule 67: a leaf group whose statement holds two or more calls, none
  indirect (the AST census's `calls`), by whether every callee is
  contracted in the run (CONTRACTED: the set of declaration names any row's
  `contract_dependencies` lists) and by statement position.
* rule 68: a leaf group with a path through a pointer member whose Clang
  type spelling has no `*` (a member declared through a pointer typedef),
  by the path's access and root kind.

Each count is given in leaf groups, Unknown functions mentioning, and
sole-blocker functions (only refusal family the access refusal) every one
of whose access leaf groups is in the class (blocked by that class alone,
as far as the AST census can tell; the reading may still decline)."""
import collections
import gzip
import json
import re
import sys

IND = "indirect memory access is outside the resource projection"


UNITS = None
SPELLED = {}


def unit_tables(unit):
    """(pointer typedef names, {struct tag: body text}) of one analysed unit."""
    if unit not in SPELLED:
        text = open(f"{UNITS}/unit-{unit}.c", errors="replace").read()
        names = set(re.findall(r"typedef\s+(?:const\s+)?(?:struct\s+)?\w+\s*\*\s*(\w+)\s*;", text))
        bodies = {}
        for match in re.finditer(r"struct\s+(\w+)\s*\{", text):
            depth, i = 1, match.end()
            while depth and i < len(text):
                depth += {"{": 1, "}": -1}.get(text[i], 0)
                i += 1
            bodies.setdefault(match.group(1), text[match.end():i])
        SPELLED[unit] = (names, bodies)
    return SPELLED[unit]


def typedef_pointer_paths(row):
    """Paths through a pointer member whose declaration spells a pointer
    typedef of the unit (`flb_sds_t name;`); the Clang types the census
    records are desugared, so the spelling is read from the analysed text."""
    names, bodies = unit_tables(row["unit"])
    out = []
    for path in row["paths"]:
        owner = path.get("root_type") or ""
        for step in path["steps"]:
            tag = re.sub(r"^(const\s+)?struct\s+", "", owner).replace("*", "").strip()
            member = step.get("member")
            if step.get("value") == "pointer" and member and tag in bodies:
                spelled = re.search(r"(\w+)\s+" + re.escape(member) + r"\s*;", bodies[tag])
                if spelled and spelled.group(1) in names:
                    out.append((path, step, spelled.group(1)))
            owner = step.get("type") or ""
    return out


def main() -> int:
    rows = [json.loads(line) for line in gzip.open(sys.argv[1], "rt")]
    contracted = set(json.load(open(sys.argv[2])))
    global UNITS
    UNITS = sys.argv[3]
    by_function = collections.defaultdict(list)
    for row in rows:
        by_function[(row["file"], row["function"])].append(row)
    sole = {k for k, v in by_function.items()
            if v[0]["status"] == "Unknown" and set(v[0]["families"]) <= {IND}}

    def table(name, predicate, label):
        groups = collections.Counter()
        functions = collections.defaultdict(set)
        alone = collections.defaultdict(set)
        for key, leaves in by_function.items():
            labels = [label(r) if predicate(r) else None for r in leaves]
            for r, l in zip(leaves, labels):
                if l:
                    groups[l] += 1
                    functions[l].add(key)
            if key in sole and all(labels) and len(set(labels)) == 1:
                alone[labels[0]].add(key)
        print(f"## {name}")
        print("| class | leaf groups | functions mentioning | sole functions alone | samples |")
        print("|---|---:|---:|---:|---|")
        for l, n in groups.most_common():
            sample = ", ".join(sorted(f for _, f in alone[l])[:4])
            print(f"| {l} | {n} | {len(functions[l])} | {len(alone[l])} | {sample} |")
        any_alone = {k for k, leaves in by_function.items()
                     if k in sole and all(predicate(r) for r in leaves)}
        print(f"\nsole functions all of whose leaves are in some class: {len(any_alone)}: "
              + ", ".join(sorted(f for _, f in any_alone)) + "\n")

    def two_calls(r):
        calls = r["calls"]
        return len(calls) >= 2 and "<indirect>" not in calls

    def two_label(r):
        status = "all contracted" if all(c in contracted for c in r["calls"]) else "some uncontracted"
        kinds = ",".join(r["construct_kinds"])
        return f"{status} / {kinds}"

    table("rule 67 (two or more direct calls in the statement)", two_calls, two_label)

    def td(r):
        return bool(typedef_pointer_paths(r))

    def td_label(r):
        path, step, name = typedef_pointer_paths(r)[0]
        return f"{path['access']} via {name} / root {path['root_kind']} / {r['premise']}"

    table("rule 68 (a pointer member spelled through a typedef)", td, td_label)
    return 0


if __name__ == "__main__":
    sys.exit(main())
