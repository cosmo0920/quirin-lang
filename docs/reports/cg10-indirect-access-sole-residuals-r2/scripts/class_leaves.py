#!/usr/bin/env python3
"""class_leaves.py CLASSIFY.json RUN -- card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS:
what the leaves of the `uncontracted-callee` / `return-uncontracted-callee`
classes (keyed by unit, function and line) read as in RUN's report."""
import collections
import json
import pathlib
import sys

TARGET = {"uncontracted-callee", "return-uncontracted-callee"}


def main() -> int:
    classes = json.loads(pathlib.Path(sys.argv[1]).read_text())
    keys = {(leaf["unit"], leaf["function"], leaf["line"])
            for leaf in classes["leaves"] if leaf["class"] in TARGET}
    report = json.loads((pathlib.Path(sys.argv[2]) / "resource-diagnostics.json").read_text())
    counts = collections.Counter()
    for unit in report["units"]:
        for function in unit["functions"]:
            for leaf in function.get("diagnostics") or []:
                if (unit["file"], function["function"], leaf["line"]) not in keys:
                    continue
                text = leaf.get("description", "")
                if text.startswith("call to uncontracted function"):
                    counts["uncontracted call"] += 1
                elif text.startswith("indirect memory access"):
                    counts["access"] += 1
                else:
                    counts["other"] += 1
    print(json.dumps({"class_lines": len(keys), "leaves": dict(counts)}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
