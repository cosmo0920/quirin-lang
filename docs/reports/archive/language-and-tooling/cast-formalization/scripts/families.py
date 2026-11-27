#!/usr/bin/env python3
"""CAST-FORMALIZATION: the sizes of the absorbed follow-ups' cast families,
from one route's report and its parity stream (usage: families.py DIAGNOSTICS
PARITY.jsonl OUT.json).

Each family counts leaves (and Unknown functions) on a line holding an
explicit cast of the given classes at the given positions
(`cast-semantics-census` rows, line or statement line), with the given
refusal descriptions:

* `cg08-b`: `unsupported assignment expression` on a line with a pointer
  conversion (CG08-CAST-AND-QUERY-RESIDUALS (b), measured there by spelling);
* `cast-root-store`: `unrecognized assignment destination` or an indirect
  access on a line with a pointer conversion at a member root
  (W45-SUB-OBJECT-STORE-PREFIX's cast root);
* `cast-step`: any refusal on a line with a pointer conversion at a pointer
  step (W43-INDEX-TRAVERSAL-STEP-COMPUTED's cast);
* `tail-cast`: the tail refusal on a line with a cast in a variadic tail
  (CG26-TAIL-RESIDUAL-ACTUALS' cast leaves).
"""
import collections
import json
import sys

POINTER = {"byte-view", "typed-view", "punned-view", "qualifier", "integer-to-pointer",
           "null-pointer", "function-object", "function-pointer"}
FAMILIES = {
    "cg08-b": (POINTER, None, lambda d: d == "unsupported assignment expression"),
    "cast-root-store": (POINTER, {"member-root"}, lambda d: d in (
        "unrecognized assignment destination",
        "indirect memory access is outside the resource projection")),
    "cast-step": (POINTER, {"pointer-step"}, lambda d: True),
    "tail-cast": (None, {"variadic-tail"},
                  lambda d: d.startswith("arguments beyond the declared parameters of")),
}


def main():
    data = json.load(open(sys.argv[1] + "/resource-diagnostics.json"))
    lines = collections.defaultdict(lambda: collections.defaultdict(set))
    seen = set()
    for line in open(sys.argv[2], encoding="utf-8"):
        if '"cast-semantics-census"' not in line:
            continue
        row = json.loads(line)
        key = (row["source_sha256"], row["function"], row["line"], row["statement_line"],
               tuple(row.get("texts") or ()))
        if key in seen:
            continue
        seen.add(key)
        for at in (row["line"], row["statement_line"]):
            lines[row["function"]][at].add((row["class"], row["cast_position"]))
    out = {}
    for name, (classes, positions, refusal) in FAMILIES.items():
        leaves, functions, unknown = 0, set(), set()
        for unit in data["units"]:
            for entry in unit["functions"]:
                cells = lines.get(entry["function"])
                if not cells:
                    continue
                for leaf in entry.get("diagnostics") or []:
                    if leaf.get("category") != "unsupported" or not refusal(leaf["description"]):
                        continue
                    if any((classes is None or c in classes) and (positions is None or p in positions)
                           for c, p in cells.get(leaf.get("line"), ())):
                        leaves += 1
                        functions.add((unit["file"], entry["function"]))
                        if entry["status"] == "Unknown":
                            unknown.add((unit["file"], entry["function"]))
        out[name] = {"leaves": leaves, "functions": len(functions), "unknown_functions": len(unknown)}
    json.dump(out, open(sys.argv[3], "w"), indent=1)
    print(json.dumps(out))


if __name__ == "__main__":
    main()
