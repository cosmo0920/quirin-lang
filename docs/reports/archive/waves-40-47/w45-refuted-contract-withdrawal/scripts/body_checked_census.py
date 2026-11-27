#!/usr/bin/env python3
"""Size the alternative this card declined: `resource_body_checked` as the
default for every declared contract (card W45-REFUTED-CONTRACT-WITHDRAWAL).

Usage: body_checked_census.py RUN_DIRECTORY OUT.json

Reads a corpus run's `resource-diagnostics.json` and its contracts sidecar.
A declared name is every `extern fn NAME` of a selected contract source. For
each declared name it records the units whose projected rows define its body
and the worst status there. Under a default checked specification (§C06.3):

* every unit that does not define the body withdraws the declaration (rule 1,
  `NoBody`), so every cross-unit declaration leaves every caller's unit;
* a defining unit keeps it only if the body establishes it, which needs at
  least a `Safe` row (no diagnostic, fully modelled); a non-`Safe` row is a
  lower bound on `NotEstablished` withdrawals.

It also counts the refutations the rows record (`contract_refutations`),
which is what this card withdraws instead.
"""
import collections
import json
import re
import sys
from pathlib import Path

DECLARATION = re.compile(r"extern\s+fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(")
ORDER = {"Safe": 0, "Pending": 1, "Unknown": 2, "Unsafe": 3}


def main():
    run = Path(sys.argv[1])
    report = json.loads((run / "resource-diagnostics.json").read_text())
    sidecar = json.loads((run / "resource-diagnostics.contracts.json").read_text())
    declared = set()
    for text in sidecar["sources"].values():
        declared.update(DECLARATION.findall(text))
    defining = collections.defaultdict(dict)
    refutations = 0
    for unit in report["units"]:
        for function in unit["functions"]:
            name = function["function"]
            refutations += len(function.get("contract_refutations", []))
            if name in declared:
                previous = defining[name].get(unit["file"])
                status = function["status"]
                if previous is None or ORDER[status] > ORDER[previous]:
                    defining[name][unit["file"]] = status
    with_body = sorted(defining)
    safe_everywhere = [n for n in with_body if all(s == "Safe" for s in defining[n].values())]
    not_safe_somewhere = [n for n in with_body if n not in safe_everywhere]
    result = {
        "declared": len(declared),
        "declared_with_an_analysed_body": len(with_body),
        "declared_without_an_analysed_body": len(declared) - len(with_body),
        "with_body_safe_in_every_defining_unit": len(safe_everywhere),
        "with_body_not_safe_in_some_defining_unit": len(not_safe_somewhere),
        "row_refutations_recorded": refutations,
        "not_safe_names": not_safe_somewhere,
        "meaning": "under a default resource_body_checked, every declaration is withdrawn from every unit that does not define its body (all cross-unit application), and a defining unit keeps it only where the body establishes it, which a non-Safe row cannot; not_safe_names is a lower bound on NotEstablished withdrawals",
    }
    Path(sys.argv[2]).write_text(json.dumps(result, indent=1) + "\n")
    print(json.dumps({k: v for k, v in result.items() if k != "not_safe_names"}, indent=1))


if __name__ == "__main__":
    main()
