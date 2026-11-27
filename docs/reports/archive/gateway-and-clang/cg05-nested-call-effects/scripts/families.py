#!/usr/bin/env python3
"""CG05-NESTED-CALL-EFFECTS: the leaf families the rule-10y consumers are asked
after, on one corpus run.

usage: families.py RUN_DIR

Per family (the nested-call refusal, `resource call with a formal embedded in
an unmodeled expression`, and the return position's refusal): leaves, Unknown
functions, and functions whose only non-alias blockers are that family.
"""

import collections
import json
import pathlib
import sys

FAMILIES = {
    "nested": lambda d: d.startswith("nested call `")
    and d.endswith("` inside a modeled call is outside the resource projection"),
    "formal-embedded": lambda d: d == "resource call with a formal embedded in an unmodeled expression",
    "return": lambda d: d == "unmodeled call, aggregate or indirect access in return expression",
}


def main():
    run = pathlib.Path(sys.argv[1])
    data = json.loads((run / "resource-diagnostics.json").read_text())
    leaves = collections.Counter()
    functions = collections.defaultdict(set)
    sole = collections.defaultdict(set)
    for unit in data["units"]:
        for function in unit["functions"]:
            key = f"{unit['file']}::{function['function']}"
            found = collections.Counter()
            others = 0
            for diagnostic in function.get("diagnostics", []):
                if diagnostic.get("category") == "unknown_alias":
                    continue
                description = diagnostic.get("description") or ""
                family = next((name for name, test in FAMILIES.items() if test(description)), None)
                if family:
                    found[family] += 1
                else:
                    others += 1
            for family, count in found.items():
                leaves[family] += count
                functions[family].add(key)
                if others == 0 and len(found) == 1:
                    sole[family].add(key)
    print(json.dumps({
        family: {
            "leaves": leaves[family],
            "functions": len(functions[family]),
            "sole_blocked": len(sole[family]),
        }
        for family in FAMILIES
    }, indent=1))


if __name__ == "__main__":
    main()
