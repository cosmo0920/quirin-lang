#!/usr/bin/env python3
"""W48-BOTTOM-UP-CALLEE-BODIES: why each return-expression leaf refuses.

Usage: return_reasons.py DIAGNOSTICS PARITY.jsonl FAMILY_JSON OUT_JSON

For every `unmodeled call, aggregate or indirect access in return
expression` leaf of an Unknown function, the parity rows written at the
same (function, line): the elaborator's refusal (`elaboration-return`
new_refusal), the residual-shape features and the return route's own row.
Rows are keyed by (function name, line) because parity rows carry the
source digest, not the unit path: a static name defined in two units at the
same line would merge (reported as `ambiguous`). Leaves of functions the
family closure frees (FAMILY_JSON's cross_unit_freed for the return family)
are marked `decisive`.
"""
import collections
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import closure_r3  # noqa: E402

FAMILY = "unsupported: unmodeled call, aggregate or indirect access in return expression"


def main() -> int:
    diagnostics, parity, family_json, out = sys.argv[1:5]
    data = closure_r3.load(diagnostics)
    freed = set()
    for row in json.loads(pathlib.Path(family_json).read_text()):
        if row["family"] == FAMILY:
            freed = {f.rsplit("@", 1)[0] for f in row["cross_unit_freed"]}
    leaves = {}
    for unit in data["units"]:
        for entry in unit["functions"]:
            if entry["status"] != "Unknown":
                continue
            for leaf in entry.get("diagnostics") or []:
                if closure_r3.family(leaf) == FAMILY:
                    key = (entry["function"], leaf.get("line"))
                    leaves.setdefault(key, []).append(f"{unit['file']}::{entry['function']}")
    found = collections.defaultdict(dict)
    with open(parity) as handle:
        for line in handle:
            if '"elaboration-return"' not in line and '"residual-shape"' not in line \
                    and '"return"' not in line and '"value-connective-return"' not in line:
                continue
            row = json.loads(line)
            key = (row.get("function"), row.get("line"))
            if key not in leaves:
                continue
            position = row.get("position")
            cell = found[key]
            if position == "elaboration-return":
                cell["elaboration"] = row.get("new_refusal")
                cell["texts"] = " ".join(row.get("texts") or [])
            elif position == "residual-shape" and row.get("of_position") == "return":
                cell["features"] = ",".join(sorted(row.get("features") or []))
            elif position == "value-connective-return":
                cell["connective"] = row.get("new_refusal") or row.get("refusal")
    rows = []
    reasons = collections.Counter()
    decisive = collections.Counter()
    for key, owners in sorted(leaves.items(), key=lambda kv: (kv[0][0], kv[0][1] or 0)):
        cell = found.get(key, {})
        reason = cell.get("elaboration") or "(no elaboration row)"
        is_decisive = any(owner in freed for owner in owners)
        rows.append({"function": key[0], "line": key[1], "owners": owners,
                     "ambiguous": len(set(owners)) > 1, "decisive": is_decisive,
                     "elaboration": reason, "features": cell.get("features"),
                     "texts": cell.get("texts")})
        label = f"{reason} | {cell.get('features')}"
        reasons[label] += 1
        if is_decisive:
            decisive[label] += 1
    pathlib.Path(out).write_text(json.dumps(rows, indent=1) + "\n")
    print(f"return leaves: {len(leaves)} keys; decisive functions: {len(freed)}")
    for label, count in reasons.most_common():
        print(f"{count:4} {decisive[label]:4}  {label}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
