#!/usr/bin/env python3
"""Predict function and unit movement from the classification, before any rule.

Usage: predict.py BASELINE_DIAGNOSTICS.json.gz CLASSIFICATION.json OUT.json

Assumes exactly the class (b) leaves are withdrawn and nothing else moves. A
function every one of whose leak leaves is class (b), and which carries no
other accusation, moves provisional Unsafe -> Unknown (it keeps the refusal the
withdrawing rule fired on). A unit moves Unsafe -> Unknown when every one of its
Unsafe functions does and it has no other Unsafe function.
"""

import collections
import gzip
import json
import pathlib
import sys

ACCUSING = {"leak", "double_release", "invalid_release", "use_after_release"}


def main() -> int:
    baseline = json.loads(gzip.decompress(pathlib.Path(sys.argv[1]).read_bytes()))
    classification = json.loads(pathlib.Path(sys.argv[2]).read_text())["classification"]
    withdrawn = collections.Counter(
        (entry["unit"], entry["function"]) for entry in classification if entry["class"] == "b"
    )
    functions = []
    units = []
    for unit in baseline["units"]:
        moved_here = []
        unsafe_here = []
        for function in unit["functions"]:
            if function["status"] != "Unsafe":
                continue
            unsafe_here.append(function["function"])
            accusations = [
                d for d in function.get("diagnostics") or [] if d["category"] in ACCUSING
            ]
            leaks = [d for d in accusations if d["category"] == "leak"]
            removed = withdrawn[(unit["file"], function["function"])]
            if removed and removed == len(leaks) == len(accusations):
                moved_here.append(function["function"])
                functions.append({"unit": unit["file"], "function": function["function"], "leaks": removed})
        if unsafe_here and len(moved_here) == len(unsafe_here):
            units.append(unit["file"])
    document = {
        "leaves_withdrawn": sum(withdrawn.values()),
        "functions_unsafe_to_unknown": functions,
        "units_unsafe_to_unknown": units,
    }
    pathlib.Path(sys.argv[3]).write_text(json.dumps(document, indent=1) + "\n")
    print(json.dumps({"leaves": document["leaves_withdrawn"], "functions": len(functions), "units": units}, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
