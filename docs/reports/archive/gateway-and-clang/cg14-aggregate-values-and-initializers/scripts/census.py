#!/usr/bin/env python3
"""census.py PARITY_JSONL -- card CG14-AGGREGATE-VALUES-AND-INITIALIZERS:
rule 11f's census from one run's `QCORE_TYPED_PATH_PARITY` stream.

Reads every `position: "aggregate-value"` row and every rule-10v flow row
whose place is an initializer guard (`… (initializer)`), deduplicated per
(function, line, texts, form) over analyses, and prints a JSON record:
per form, the statements admitted in some analysis and in every analysis,
the refusal codes, the classes and leaves, the functions, and the flow
guard's decisions. Admission here is at classification; the flow guard and
the flow's own obligations decide afterwards.
"""

import collections
import json
import sys


def main() -> int:
    rows = collections.defaultdict(list)
    guard = collections.Counter()
    guard_functions = collections.defaultdict(set)
    with open(sys.argv[1], encoding="utf-8") as stream:
        for line in stream:
            if '"aggregate-value"' not in line and "(initializer)" not in line:
                continue
            row = json.loads(line)
            if row.get("position") == "aggregate-value":
                key = (row["function"], row["line"], tuple(row["texts"]), row["form"])
                rows[key].append(row)
            elif row.get("position") == "pointer-store-flow" and row.get("place", "").endswith(
                "(initializer)"
            ):
                guard[row["decision"]] += 1
                guard_functions[row["decision"]].add(row["function"])
    forms = {}
    for (function, _, _, form), analyses in rows.items():
        entry = forms.setdefault(
            form,
            {
                "statements": 0,
                "admitted_some": 0,
                "admitted_every": 0,
                "refusals": collections.Counter(),
                "classes": collections.Counter(),
                "leaves": collections.Counter(),
                "functions_admitted": set(),
                "functions": set(),
            },
        )
        entry["statements"] += 1
        entry["functions"].add(function)
        admitted = [row["admitted"] for row in analyses]
        if any(admitted):
            entry["admitted_some"] += 1
            entry["functions_admitted"].add(function)
        if all(admitted):
            entry["admitted_every"] += 1
        first = analyses[0]
        if not first["admitted"]:
            entry["refusals"][first["refusal"]] += 1
        detail = first.get("detail") or {}
        if isinstance(detail, dict):
            if "class" in detail:
                entry["classes"][detail["class"]] += 1
            for leaf in set(detail.get("leaves", [])):
                entry["leaves"][leaf] += 1
    record = {
        form: {
            "statements": entry["statements"],
            "admitted_in_some_analysis": entry["admitted_some"],
            "admitted_in_every_analysis": entry["admitted_every"],
            "functions": len(entry["functions"]),
            "functions_with_an_admission": len(entry["functions_admitted"]),
            "refusals": dict(entry["refusals"].most_common()),
            "classes": dict(entry["classes"].most_common()),
            "statements_with_leaf": dict(entry["leaves"].most_common()),
        }
        for form, entry in sorted(forms.items())
    }
    record["initializer-guard-flow"] = {
        "decisions": dict(guard.most_common()),
        "functions": {decision: len(names) for decision, names in guard_functions.items()},
    }
    print(json.dumps(record, indent=1))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
