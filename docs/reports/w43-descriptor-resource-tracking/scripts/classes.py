#!/usr/bin/env python3
"""Split the arbiter's movement into the card's four classes.

Usage: classes.py BEFORE_DIR CLASS_A_DIR AFTER_DIR ATTRIBUTION.json OUT.json

* (a) vocabulary and gateway, no new declaration: BEFORE vs CLASS_A, all five
  raw artifacts compared byte for byte.
* (b) declarations: every leaf `w43-libc-contracts/scripts/attribute.py`
  assigned (CLASS_A vs AFTER), by callee and shape. A leaf that script leaves
  unattributed is accepted here only when it is the card's own descriptor
  guard (`tracked descriptor ... custody-neutral scalar route`) or the leaf it
  replaced at the same coordinate: the class-(a) rule firing on a name a
  class-(b) declaration made a descriptor. Anything else stays unattributed
  and fails the run.
* (c) wrappers: none declared (recorded, not measured here).
* (d) violation candidates: every added or removed leaf in a safety category
  (`leak`, `invalid_release`, `use_after_release`, `double_release`).

Exits 0 only when every changed leaf is in a class.
"""

import collections
import json
import pathlib
import sys

NAMES = {
    "diagnostics": "resource-diagnostics.json",
    "chunk-contracts": "resource-diagnostics.chunk-contracts.json",
    "contracts": "resource-diagnostics.contracts.json",
    "source-effects": "resource-diagnostics.source-effects.json",
    "source": "monkey-source.json",
}
GUARD = "read by a custody-neutral scalar route is outside the resource projection"
SAFETY = {"leak", "invalid_release", "use_after_release", "double_release"}


def main() -> int:
    before, class_a, after = (pathlib.Path(p) for p in sys.argv[1:4])
    attribution = json.loads(pathlib.Path(sys.argv[4]).read_text())
    record: dict = {"a_vocabulary_and_gateway_alone": {}}
    for label, name in NAMES.items():
        record["a_vocabulary_and_gateway_alone"][label] = (
            (before / name).read_bytes() == (class_a / name).read_bytes()
        )
    leaves = attribution["leaves"]
    guard_coordinates = {
        (leaf["unit"], leaf["function"], leaf["line"])
        for leaf in leaves
        if leaf["class"] is None and GUARD in leaf["description"]
    }
    by_class = collections.Counter()
    unattributed = []
    candidates = []
    for leaf in leaves:
        klass = leaf["class"]
        if klass is None:
            if (leaf["unit"], leaf["function"], leaf["line"]) in guard_coordinates:
                klass = "descriptor-guard"
            else:
                unattributed.append(leaf)
                continue
        by_class[f"{leaf['direction']} {klass}"] += leaf["count"]
        if leaf["category"] in SAFETY:
            candidates.append(
                {
                    key: leaf[key]
                    for key in ("direction", "unit", "function", "origin", "category", "description")
                }
            )
    record["b_declarations"] = {
        "by_class": dict(sorted(by_class.items())),
        "removed_uncontracted_by_callee": attribution["summary"]["removed_uncontracted_by_callee"],
        "status_moves": attribution["summary"]["status_moves"],
        "moved_functions": attribution["summary"]["moved_functions"],
    }
    record["c_wrappers"] = "none declared"
    record["d_violation_candidates"] = candidates
    record["unattributed"] = unattributed
    record["attribution_complete"] = not unattributed and all(
        record["a_vocabulary_and_gateway_alone"].values()
    )
    pathlib.Path(sys.argv[5]).write_text(json.dumps(record, indent=1, sort_keys=True) + "\n")
    print(json.dumps({k: v for k, v in record.items() if k != "b_declarations"}, indent=1)[:3000])
    print(json.dumps(record["b_declarations"], indent=1))
    return 0 if record["attribution_complete"] else 1


if __name__ == "__main__":
    sys.exit(main())
