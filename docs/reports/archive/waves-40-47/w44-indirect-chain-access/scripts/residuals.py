#!/usr/bin/env python3
"""Card W44-INDIRECT-CHAIN-ACCESS: size each registered residual of the chain family.

Usage: residuals.py CHAIN_CENSUS_JSON ATTRIBUTION_JSON OUT_JSON OUT_TXT

`CHAIN_CENSUS_JSON` is `chain_census.py`'s output over the AFTER census:
every leaf W43's two chain classes still hold, with the reader outcome of its
hardest record. `ATTRIBUTION_JSON` is `attribute.py`'s output, whose
`added:exchange:*` leaves are the flow half's refusals. Each residual reports
leaves, Unknown functions, and functions whose only blocker family is the
leaf's own (W43's `sole` reading), split by statement and condition family.
"""

import collections
import json
import pathlib
import sys

GROUPS = {
    "W44-CHAIN-NESTED-AGGREGATE": {"dot-link"},
    "W44-CHAIN-TYPEDEF-ROOT": {"root-untyped"},
    "W44-CHAIN-UNDECLARED-ROOT": {"root-undeclared"},
    "W44-CHAIN-SUBSCRIPT": {"subscript"},
    "W44-CHAIN-INCOMPLETE-RECORD": {"incomplete-record"},
    "W44-CHAIN-CALL": {"call"},
    "W44-CHAIN-NON-SCALAR-MEMBER": {"pointer-member", "other-member", "non-pointer-link"},
    "W44-CHAIN-CLASS-NOT-A-CHAIN": {"declarator-not-a-chain", "no-member-chain", "unexplained"},
}


def main():
    census = json.loads(pathlib.Path(sys.argv[1]).read_text())
    attribution = json.loads(pathlib.Path(sys.argv[2]).read_text())
    result = {}
    covered = collections.Counter()
    for name, readers in GROUPS.items():
        leaves = [leaf for leaf in census["family_leaves"] if leaf["reader"] in readers]
        covered.update(leaf["reader"] for leaf in leaves)
        by_family = collections.Counter(leaf["family"] for leaf in leaves)
        result[name] = {
            "leaves": len(leaves),
            "functions": len({leaf["function"] for leaf in leaves}),
            "sole": len({leaf["function"] for leaf in leaves if leaf["sole"]}),
            "statement_leaves": by_family["statement"],
            "condition_leaves": by_family["condition"],
            "by_reader": dict(collections.Counter(leaf["reader"] for leaf in leaves)),
        }
    uncovered = collections.Counter(
        leaf["reader"] for leaf in census["family_leaves"]
    ) - covered
    exchanges = [
        leaf
        for cls, leaves in attribution["leaves"].items()
        if cls.startswith("added:exchange")
        for leaf in leaves
    ]
    result["W44-CHAIN-BOUND-ROOT"] = {
        "leaves": len(exchanges),
        "functions": len({leaf.rsplit(":", 3)[0] for leaf in exchanges}),
        "note": "unresolved resource identity leaves the flow half added at an admitted chain",
    }
    result["family_total"] = census["family"]
    result["uncovered_readers"] = dict(uncovered)
    pathlib.Path(sys.argv[3]).write_text(json.dumps(result, indent=1) + "\n")
    lines = [f"{'residual':32} {'leaves':>7} {'funcs':>6} {'sole':>5} {'stmt':>6} {'cond':>6}"]
    for name in list(GROUPS) + ["W44-CHAIN-BOUND-ROOT"]:
        row = result[name]
        lines.append(
            f"{name:32} {row['leaves']:7} {row['functions']:6} {row.get('sole', '-'):>5} "
            f"{row.get('statement_leaves', '-'):>6} {row.get('condition_leaves', '-'):>6}"
        )
    lines.append(f"family at this census: {census['family']}")
    lines.append(f"readers no residual covers: {dict(uncovered)}")
    pathlib.Path(sys.argv[4]).write_text("\n".join(lines) + "\n")
    print("\n".join(lines))
    return 0


if __name__ == "__main__":
    sys.exit(main())
