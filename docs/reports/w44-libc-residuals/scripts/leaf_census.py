#!/usr/bin/env python3
"""Leaf census of the four W44-LIBC-RESIDUALS families over one corpus run.

Usage: leaf_census.py DIAGNOSTICS(.json|.json.gz) OUT.json [HEADER_INLINE_NAMES]

Families, each keyed by the backticked callee its description names:

* ``uncontracted``  -- ``call to uncontracted function `X```
* ``tail``          -- ``arguments beyond the declared parameters of `X` ...``
* ``nested``        -- ``nested call `X` inside a modeled call ...``
* ``withdrawn``     -- ``call to `X` whose declared contract was withdrawn ... (KEY)``

For every family and callee it counts leaves and functions, split by function
status, and it answers the question the card has to answer before building
any widening: **how many non-Safe functions would move if a family's leaves
were all removed**, and nothing else changed. That is an upper bound on the
function yield of any admission of that family (an admission can also add a
leaf, and can un-poison a binding elsewhere in the body, which this bound does
not model). ``HEADER_INLINE_NAMES`` (one name per line) selects the callees
whose uncontracted leaves are counted as the header-inline family.
"""

import collections
import gzip
import json
import re
import sys

PATTERNS = {
    "uncontracted": re.compile(r"call to uncontracted function `([^`]+)`$"),
    "tail": re.compile(
        r"arguments beyond the declared parameters of `([^`]+)` are outside the resource projection$"
    ),
    "nested": re.compile(r"nested call `([^`]+)` inside a modeled call is outside the resource projection$"),
    "withdrawn": re.compile(
        r"call to `([^`]+)` whose declared contract was withdrawn against its C declaration in this "
        r"translation unit \(([a-z-]+)\)$"
    ),
}
PRINTF = {"printf", "fprintf", "snprintf", "sprintf"}


def load(path):
    opener = gzip.open if str(path).endswith(".gz") else open
    with opener(path, "rt") as handle:
        return json.load(handle)


def classify(description):
    for family, pattern in PATTERNS.items():
        match = pattern.match(description)
        if match:
            return family, match.group(1)
    return None, None


def main() -> int:
    data = load(sys.argv[1])
    out = sys.argv[2]
    inline = set()
    if len(sys.argv) > 3:
        inline = {line.strip() for line in open(sys.argv[3]) if line.strip()}
    leaves = collections.defaultdict(collections.Counter)
    functions = collections.defaultdict(lambda: collections.defaultdict(set))
    by_status = collections.defaultdict(lambda: collections.defaultdict(collections.Counter))
    rows = []
    for unit in data["units"]:
        for entry in unit["functions"]:
            key = (unit.get("library"), unit["file"], entry["function"])
            status = entry["status"]
            families = []
            for diagnostic in entry.get("diagnostics") or []:
                family, callee = classify(diagnostic["description"])
                families.append((family, callee))
                if family is None:
                    continue
                leaves[family][callee] += 1
                functions[family][callee].add(key)
                by_status[family][callee][status] += 1
            rows.append((key, status, families))

    def removable(selector):
        """Non-Safe functions whose every leaf the selector removes."""
        moved = []
        touched = 0
        for key, status, families in rows:
            if status == "Safe" or not families:
                continue
            hits = [selector(f, c) for f, c in families]
            if any(hits):
                touched += 1
                if all(hits):
                    moved.append({"function": key, "status": status, "leaves": len(families)})
        return touched, moved

    bounds = {}
    selectors = {
        "header_inline_uncontracted": lambda f, c: f == "uncontracted" and c in inline,
        "va_list_withdrawn": lambda f, c: f == "withdrawn" and c in {"vsnprintf", "vprintf", "vfprintf", "vsprintf"},
        "printf_tail": lambda f, c: f == "tail" and c in PRINTF,
        "all_tail": lambda f, c: f == "tail",
        "printf_nested": lambda f, c: f == "nested",
        "all_tail_and_nested": lambda f, c: f in {"tail", "nested"},
        "all_four_families": lambda f, c: (f == "uncontracted" and c in inline)
        or (f == "withdrawn" and c in {"vsnprintf", "vprintf", "vfprintf", "vsprintf"})
        or f in {"tail", "nested"},
    }
    for name, selector in selectors.items():
        touched, moved = removable(selector)
        bounds[name] = {"functions_touched_non_safe": touched, "functions_all_leaves_removed": len(moved), "moved": moved}

    result = {"families": {}, "bounds": bounds}
    for family in PATTERNS:
        table = []
        for callee, count in leaves[family].most_common():
            table.append(
                {
                    "callee": callee,
                    "leaves": count,
                    "functions": len(functions[family][callee]),
                    "leaves_by_status": dict(by_status[family][callee]),
                }
            )
        result["families"][family] = {
            "leaves": sum(leaves[family].values()),
            "functions": len(set().union(*functions[family].values())) if functions[family] else 0,
            "callees": table,
        }
    if inline:
        chosen = [row for row in result["families"]["uncontracted"]["callees"] if row["callee"] in inline]
        result["header_inline"] = {
            "names": sorted(inline),
            "leaves": sum(row["leaves"] for row in chosen),
            "functions": len(set().union(*(functions["uncontracted"][n] for n in inline if n in functions["uncontracted"]))),
            "callees": chosen,
        }
    with open(out, "w") as handle:
        json.dump(result, handle, indent=1, sort_keys=True)
        handle.write("\n")
    for family, block in result["families"].items():
        print(family, block["leaves"], "leaves", block["functions"], "functions")
    for name, bound in bounds.items():
        print(name, bound["functions_touched_non_safe"], "touched", bound["functions_all_leaves_removed"], "would move")
    return 0


if __name__ == "__main__":
    sys.exit(main())
