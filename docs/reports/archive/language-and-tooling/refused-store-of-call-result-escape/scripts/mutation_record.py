#!/usr/bin/env python3
"""Card REFUSED-STORE-OF-CALL-RESULT-ESCAPE: record the guard-mutation control.

Usage: mutation_record.py CENSUS_DIR OUT.json

CENSUS_DIR holds `cmt_map.i`, `cmt_map-unguarded.i` (line 5875 of
`cmt_map_destroy`, `if (map->metric_static_set) {`, replaced by `if (1) {`)
and `unguarded.log`, the token-level harness's output over the mutated text
with `QCORE_LEAK_WITNESS=CENSUS_DIR/unguarded-witness.jsonl`.
"""

import difflib
import hashlib
import json
import os
import re
import sys


def functions(path):
    found = {}
    current = None
    for line in open(path, encoding="utf-8").read().splitlines():
        match = re.match(r"== (\S+) (\S+)$", line)
        if match:
            current = match.group(1)
            found[current] = {"status": match.group(2), "diagnostics": []}
            continue
        match = re.match(r"   (\d+) (.*)$", line)
        if match and current:
            found[current]["diagnostics"].append({"line": int(match.group(1)), "kind": match.group(2)})
    return found


def main() -> int:
    census = sys.argv[1]
    original = open(os.path.join(census, "cmt_map.i"), encoding="utf-8").read().splitlines()
    mutated_bytes = open(os.path.join(census, "cmt_map-unguarded.i"), "rb").read()
    mutated = mutated_bytes.decode("utf-8").splitlines()
    diff = [line for line in difflib.unified_diff(original, mutated, lineterm="", n=0)][2:]
    after = functions(os.path.join(census, "unguarded.log"))
    before = functions(os.path.join(census, "explore.log"))
    witness = os.path.join(census, "unguarded-witness.jsonl")
    changed = sorted(
        name for name in set(before) | set(after) if before.get(name) != after.get(name)
    )
    document = {
        "schema": "refused-store-mutation/1",
        "card": "REFUSED-STORE-OF-CALL-RESULT-ESCAPE",
        "mutation": diff,
        "mutated_sha256": hashlib.sha256(mutated_bytes).hexdigest(),
        "witness_lines": (
            len(open(witness, encoding="utf-8").read().splitlines()) if os.path.exists(witness) else 0
        ),
        "leaks_in_unit": sum(
            1 for record in after.values() for d in record["diagnostics"] if d["kind"].startswith("Leak")
        ),
        "functions_changed": {
            name: {"before": before.get(name), "after": after.get(name)} for name in changed
        },
    }
    with open(sys.argv[2], "w", encoding="utf-8") as out:
        json.dump(document, out, indent=1)
        out.write("\n")
    print(json.dumps({k: v for k, v in document.items() if k != "functions_changed"}, indent=1))
    print("changed:", changed)
    return 0


if __name__ == "__main__":
    sys.exit(main())
