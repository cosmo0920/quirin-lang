#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Card PRIORITY0-ATTRIBUTION: the README tables of evidence_classes.py's
output (read-only).

    python3 evidence_summary.py EVIDENCE_CLASSES.json
"""

import json
import re
import sys
from collections import Counter


def short(cause):
    cause = re.sub(r" \[landed in ([0-9a-f]{9}) [^\]]*\]", r" [\1]", cause)
    return cause


def main():
    d = json.load(open(sys.argv[1]))
    print("| Route | Step | Rows with an evidence change | Attributed | Unattributed |")
    print("|---|---|---|---|---|")
    for route, v in d["routes"].items():
        for step in ("1", "2"):
            total = v["rows_with_evidence_change"][step]
            un = v["causes"][step].get("unattributed", 0)
            print(f"| {route} | {'prev->common' if step == '1' else 'common->tip'} | {total} | {total - un} | {un} |")
    print()
    for step in ("1", "2"):
        print(f"Causes, step {step} (rows; a row may have several):")
        print()
        print("| Cause | corpus | twelve-home | system | slice |")
        print("|---|---|---|---|---|")
        causes = sorted({c for v in d["routes"].values() for c in v["causes"][step]})
        for c in sorted(causes, key=lambda c: -d["routes"]["corpus"]["causes"][step].get(c, 0)):
            cells = [str(d["routes"][r]["causes"][step].get(c, 0)) for r in ("corpus", "twelve-home", "system", "slice")]
            print(f"| {short(c)} | " + " | ".join(cells) + " |")
        print()
    print("Classes of the unattributed rows (rows per class):")
    print()
    print("| Route | Step | Class | Rows |")
    print("|---|---|---|---|")
    for route, v in d["routes"].items():
        for step in (1, 2):
            k = Counter()
            for u in v["unattributed"]:
                if u["step"] == step:
                    for c in u["classes"]:
                        k[c] += 1
            for c, n in k.most_common(6):
                print(f"| {route} | {step} | {c} | {n} |")


if __name__ == "__main__":
    main()
