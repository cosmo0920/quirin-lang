#!/usr/bin/env python3
"""Card W47-ALIAS-RESIDUALS-CENSUS: which Unknown functions each alias class blocks.

usage: functions.py DIAGNOSTICS.json CENSUS_OUT OUT_JSON [--system] [--key class|group]

`--key` selects the leaf label the figures are grouped by: `class` (the
leaf's origin class, the default) or `group` (the discharge group one card
would discharge together).

Reads alias_census.py's `leaves.jsonl.gz` and `unit-map.json` beside the
report. Every other leaf is put in the family vocabulary of P1 and the STATUS
rows (`IND`, `COND`, `RET`, `UNC`, `OTHER`, plus the non-unsupported
categories `leak`, `summary_conflict`, `counted_user`). Per Unknown function:

* **alone**: every leaf is `unknown_alias`; grouped by the set of alias
  classes (one class: that class discharges it alone);
* **alias + one family**: every leaf is `unknown_alias` or in exactly one
  other family; grouped by (alias class set, family);
* **closure** (P1's rule, W46 predict.py): with the class discharged, a
  function whose leaves are all that class's aliases, or uncontracted calls
  to callees now Safe, is freed and taken Safe (the upper bound: a discharged
  obligation, nothing more). A callee serves a same-unit caller (the existing
  summary rule, `same-unit`), or also a caller in another unit that defines no
  such function (`cross-unit`, no rule does this today). Rounds are counted.

With `--system` functions are joined over configurations by (file, function)
(Safe < Pending < Unknown < Unsafe), as the system slice is counted, and the
closure is not computed (the slice has no uncontracted-call callee oracle
beyond the report itself).
"""

import collections
import gzip
import json
import pathlib
import re
import sys

UNC = re.compile(r"call to uncontracted function `([^`]*)`")
ORDER = {"Safe": 0, "Pending": 1, "Unknown": 2, "Unsafe": 3}


def family(leaf):
    category = leaf["category"]
    if category != "unsupported":
        return category
    description = leaf["description"]
    if description == "indirect memory access is outside the resource projection":
        return "IND"
    if description.startswith("call to uncontracted function"):
        return "UNC"
    if description == "unmodeled call, aggregate or indirect access in return expression":
        return "RET"
    if description.startswith("unmodeled call, aggregate or indirect access in"):
        return "COND"
    return "OTHER"


def jsonl(path):
    with gzip.open(path, "rt") as handle:
        rows = [json.loads(line) for line in handle]
    return rows[1:] if rows and "schema" in rows[0] else rows


def main():
    diagnostics = json.load(open(sys.argv[1]))
    census = pathlib.Path(sys.argv[2])
    out = pathlib.Path(sys.argv[3])
    system = "--system" in sys.argv
    leaves = jsonl(census / "leaves.jsonl.gz")
    unit_map = json.loads((census / "unit-map.json").read_text())
    key_name = "class"
    if "--key" in sys.argv:
        key_name = sys.argv[sys.argv.index("--key") + 1]
    alias_class = {}
    for row in leaves:
        alias_class[(row["file"], row["function"], row["line"], row["variable"])] = row[key_name]

    functions = {}
    for unit in diagnostics["units"]:
        for function in unit["functions"]:
            key = (unit["file"], function["function"])
            entry = functions.setdefault(key, {"status": function["status"], "library": unit.get("library"),
                                               "aliases": [], "families": collections.Counter(),
                                               "callees": set()})
            if ORDER[function["status"]] > ORDER[entry["status"]]:
                entry["status"] = function["status"]
            for leaf in function["diagnostics"]:
                if leaf["category"] == "unknown_alias":
                    variable = leaf["description"].split("`", 1)[1].rsplit("`", 1)[0]
                    entry["aliases"].append(alias_class.get((unit["file"], function["function"],
                                                             leaf["line"], variable), "unclassified"))
                    continue
                name = family(leaf)
                entry["families"][name] += 1
                if name == "UNC":
                    entry["callees"].add(UNC.match(leaf["description"]).group(1))

    unknown = {k: v for k, v in functions.items() if v["status"] == "Unknown"}
    with_alias = {k: v for k, v in unknown.items() if v["aliases"]}
    alone = collections.Counter()
    plus_one = collections.Counter()
    alone_lists = collections.defaultdict(list)
    per_class_functions = collections.Counter()
    for key, entry in with_alias.items():
        classes = frozenset(entry["aliases"])
        for name in classes:
            per_class_functions[name] += 1
        label = "+".join(sorted(classes))
        if not entry["families"]:
            alone[label] += 1
            alone_lists[label].append(f"{key[0]}:{key[1]}")
        elif len(entry["families"]) == 1:
            plus_one[(label, next(iter(entry["families"])))] += 1

    # Per single label: functions whose every alias leaf carries it
    # (`exclusive`, whatever else blocks them), those it blocks alone, and
    # those it blocks with exactly one other family (by family).
    ranking = collections.defaultdict(lambda: collections.Counter())
    for key, entry in with_alias.items():
        labels = set(entry["aliases"])
        if len(labels) != 1:
            continue
        label = next(iter(labels))
        ranking[label]["exclusive"] += 1
        if not entry["families"]:
            ranking[label]["alone"] += 1
        elif len(entry["families"]) == 1:
            ranking[label]["plus_one"] += 1
            ranking[label]["plus_" + next(iter(entry["families"]))] += 1
    result_ranking = {label: dict(counter) for label, counter in
                      sorted(ranking.items(), key=lambda item: (-item[1]["alone"], -item[1]["plus_one"],
                                                                -item[1]["exclusive"], item[0]))}

    result = {
        "key": key_name,
        "ranking": result_ranking,
        "unknown_functions": len(unknown),
        "unknown_with_alias": len(with_alias),
        "functions_per_class": dict(per_class_functions.most_common()),
        "alone": dict(alone.most_common()),
        "alone_total": sum(alone.values()),
        "alias_plus_one_family": {f"{a} + {f}": n for (a, f), n in plus_one.most_common()},
        "alias_plus_one_family_by_family": dict(sum(
            (collections.Counter({f: n}) for (_, f), n in plus_one.items()), collections.Counter()).most_common()),
        "alone_functions": {k: sorted(v) for k, v in sorted(alone_lists.items())},
    }

    if not system:
        definitions = unit_map["definitions"]
        defined_in = collections.defaultdict(list)
        for file, names in definitions.items():
            for name, storage in names.items():
                if (file, name) in functions and storage != "static":
                    defined_in[name].append(file)

        def closure(discharged, variant):
            safe = {k for k, v in functions.items() if v["status"] == "Safe"}
            freed = {}
            round_number = 0
            rounds = []
            while True:
                added = []
                for key, entry in unknown.items():
                    if key in freed:
                        continue
                    if any(name not in discharged for name in entry["aliases"]):
                        continue
                    if set(entry["families"]) - {"UNC"}:
                        continue
                    served = True
                    for callee in entry["callees"]:
                        if (key[0], callee) in safe:
                            continue
                        if variant == "cross-unit" and len(defined_in.get(callee, [])) == 1 \
                                and (defined_in[callee][0], callee) in safe:
                            continue
                        served = False
                        break
                    if served:
                        added.append(key)
                for key in added:
                    freed[key] = round_number
                rounds.append(len(added))
                if not added:
                    break
                safe |= set(added)
                round_number += 1
            return freed, rounds

        classes = sorted(per_class_functions)
        gains = {}
        # The baseline closure (nothing discharged) frees functions whose only
        # blockers are uncontracted calls to already-Safe callees in another
        # unit (the cross-unit variant); a class's figures are what it frees
        # beyond that baseline. `direct`: freed in round 0 (every leaf is an
        # alias of the class, or a call to a callee Safe before this class);
        # `closure_gain`: freed in a later round because a callee was freed.
        baseline = {variant: set(closure(set(), variant)[0]) for variant in ("same-unit", "cross-unit")}
        result["closure_baseline"] = {variant: len(keys) for variant, keys in baseline.items()}
        for name in classes + ["<every class>"]:
            discharged = set(classes) | {"unclassified"} if name == "<every class>" else {name}
            entry = {}
            for variant in ("same-unit", "cross-unit"):
                freed, rounds = closure(discharged, variant)
                new = {key: round_number for key, round_number in freed.items() if key not in baseline[variant]}
                direct = sum(1 for round_number in new.values() if round_number == 0)
                entry[variant] = {"freed": len(new), "direct": direct, "closure_gain": len(new) - direct,
                                  "rounds": rounds}
            gains[name] = entry
        result["closure"] = gains
    out.write_text(json.dumps(result, indent=1, sort_keys=False) + "\n")
    printable = {k: v for k, v in result.items() if k != "alone_functions"}
    print(json.dumps(printable, indent=1))


if __name__ == "__main__":
    main()
