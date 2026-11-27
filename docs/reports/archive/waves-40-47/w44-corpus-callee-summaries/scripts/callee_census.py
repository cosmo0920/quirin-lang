#!/usr/bin/env python3
"""Census every uncontracted corpus callee by why no summary reaches its calls.

Usage: callee_census.py DIAGNOSTICS.json[.gz] DEFINITION_SITES.json OUT.json [SCOPE_CENSUS.jsonl]

Reads the `call to uncontracted function` leaves of every Unknown function (the
population the card brief counts: leaves, Unknown functions, functions blocked
only by uncontracted calls). Each leaf is classified by where the callee's body
is, as seen from the **calling** unit (`definition_sites.py`), first rule wins:

* `same-unit-selected:<status>` -- a main-source definition of the calling unit;
  W4 already tries it, so it yields no summary because its own body does not
  validate (its status is given);
* `in-unit-header-internal` / `in-unit-header-external` -- a definition the
  translation unit carries from a header (not a system header), with or without
  `static`: outside the derivation scope before this card;
* `in-unit-system-header` -- a system-header inline body (`__bswap_*`, owned by
  W44-LIBC-RESIDUALS);
* `other-unit:<statuses>` -- no body in the calling unit, a main-source definition
  in another pinned unit; no cross-unit summary route exists;
* `no-corpus-body` -- no definition in any pinned unit (for example the cfl
  library, which is not a pinned unit).

With SCOPE_CENSUS (the `QCORE_CALLEE_SCOPE_CENSUS` stream of an
`indirect-census` build) the header rows are split further by what the widened
derivation decides for that callee in that unit: `validated`, `no_summary`
(with the body's own status and first blockers), a declared outcome, or
`refused_linkage`.

For each callee the census reports leaves, Unknown functions, functions whose
only blocker is that one callee (`sole`), and functions blocked only by
uncontracted calls whose every uncontracted callee is in the same class
(`class_only`).
"""

import collections
import gzip
import json
import re
import sys

CALL = re.compile(r"call to uncontracted function `([^`]+)`")


def load(path):
    opener = gzip.open if path.endswith(".gz") else open
    return json.load(opener(path))


def main() -> int:
    diagnostics, sites_path, out = sys.argv[1:4]
    scope_path = sys.argv[4] if len(sys.argv) > 4 else None
    data = load(diagnostics)
    sites = load(sites_path)["units"]
    homes = collections.defaultdict(list)
    for unit in data["units"]:
        for entry in unit["functions"]:
            homes[entry["function"]].append((unit["library"] + "/" + unit["file"], entry["status"]))
    scope = {}
    if scope_path:
        seen = set()
        rows = [json.loads(line) for line in open(scope_path)]
        by_selected = {}
        for row in rows:
            key = tuple(sorted(row["selected"]))
            if key in seen:
                continue
            seen.add(key)
            by_selected[key] = row
        for unit in data["units"]:
            key = tuple(sorted(entry["function"] for entry in unit["functions"]))
            row = by_selected.get(key)
            if row is None:
                continue
            scope[unit["library"] + "/" + unit["file"]] = {c["name"]: c for c in row["callees"]}

    def classify(unit_key, callee):
        local = sites.get(unit_key, {}).get("definitions", {})
        for home, status in homes.get(callee, []):
            if home == unit_key:
                return f"same-unit-selected:{status}", None
        site = local.get(callee)
        if site is not None and site["file"] != "<stdin>":
            if site["system"]:
                return "in-unit-system-header", None
            base = "in-unit-header-internal" if site["static"] and not site["extern"] else "in-unit-header-external"
            decided = scope.get(unit_key, {}).get(callee)
            return base, decided
        if callee in homes:
            statuses = "/".join(sorted({s for _, s in homes[callee]}))
            return f"other-unit:{statuses}", None
        return "no-corpus-body", None

    leaves = collections.Counter()
    functions = collections.defaultdict(set)
    sole = collections.Counter()
    callee_rows = {}
    class_leaves = collections.Counter()
    class_callees = collections.defaultdict(set)
    class_functions = collections.defaultdict(set)
    class_sole = collections.Counter()
    class_only = collections.Counter()
    decisions = collections.defaultdict(collections.Counter)
    blocker_reasons = collections.defaultdict(collections.Counter)
    unknown = 0
    only_calls = 0
    total = 0
    for unit in data["units"]:
        key = unit["library"] + "/" + unit["file"]
        for entry in unit["functions"]:
            if entry["status"] != "Unknown":
                continue
            descriptions = [d["description"] for d in entry.get("diagnostics") or []]
            matches = [CALL.fullmatch(d) for d in descriptions]
            callees = {m.group(1) for m in matches if m}
            if not callees:
                continue
            unknown += 1
            classes = set()
            for match in matches:
                if not match:
                    continue
                callee = match.group(1)
                total += 1
                kind, decided = classify(key, callee)
                label = kind
                if decided is not None:
                    label = kind + ":" + decided["outcome"]
                    decisions[callee][decided["outcome"]] += 1
                    if decided["outcome"] == "no_summary" and decided.get("body_blockers"):
                        blocker_reasons[callee][decided["body_blockers"][0][0]] += 1
                classes.add(label)
                leaves[callee] += 1
                functions[callee].add(key + ":" + entry["function"])
                class_leaves[label] += 1
                class_callees[label].add(callee)
                class_functions[label].add(key + ":" + entry["function"])
                callee_rows.setdefault(callee, {"classes": collections.Counter()})
                callee_rows[callee]["classes"][label] += 1
            if all(matches):
                only_calls += 1
                if len(callees) == 1:
                    sole[next(iter(callees))] += 1
                    class_sole[next(iter(classes))] += 1
                if len(classes) == 1:
                    class_only[next(iter(classes))] += 1
    rows = []
    for callee, count in leaves.most_common():
        rows.append({
            "callee": callee,
            "leaves": count,
            "unknown_functions": len(functions[callee]),
            "sole": sole[callee],
            "classes": dict(callee_rows[callee]["classes"].most_common()),
            "homes": homes.get(callee, []),
            "widened_decisions": dict(decisions[callee]),
            "no_summary_first_blockers": dict(blocker_reasons[callee].most_common(3)),
        })
    classes = []
    for label in sorted(class_leaves, key=lambda k: -class_leaves[k]):
        classes.append({
            "class": label,
            "leaves": class_leaves[label],
            "callees": len(class_callees[label]),
            "unknown_functions": len(class_functions[label]),
            "sole": class_sole[label],
            "class_only": class_only[label],
        })
    record = {
        "diagnostics": diagnostics,
        "totals": {"leaves": total, "unknown_functions": unknown,
                   "blocked_only_by_uncontracted_calls": only_calls,
                   "sole_one_callee": sum(sole.values()), "callees": len(leaves)},
        "classes": classes,
        "callees": rows,
    }
    with open(out, "w") as handle:
        json.dump(record, handle, indent=1)
        handle.write("\n")
    print(json.dumps(record["totals"], indent=1))
    for row in classes:
        print(f"{row['class']:52} leaves {row['leaves']:5} callees {row['callees']:5} "
              f"fns {row['unknown_functions']:5} sole {row['sole']:3} class_only {row['class_only']:3}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
