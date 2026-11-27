#!/usr/bin/env python3
"""CG02-ELABORATION-CORE: census of the elaborator's parity rows.

Usage: parity_census.py PARITY_JSONL [--json OUT]

Reads the `position: "elaboration-*"` rows a `QCORE_TYPED_PATH_PARITY` corpus
run wrote. A CFG is built more than once per function (W4 derivation, the
reported build, validation), so rows are deduplicated by (function, line,
texts, position). Reports per position: consultations, admissions, and the
refusal reasons; and the comma returns (`return (E1), E2`-shaped rows, the
W43-RETURN-COMMA-MACRO population) with their refusal reasons.
"""

import argparse
import collections
import json
import pathlib


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("parity", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    rows = {}
    with args.parity.open() as handle:
        for text in handle:
            row = json.loads(text)
            position = row.get("position", "")
            if not position.startswith("elaboration-"):
                continue
            key = (row["function"], row["line"], tuple(row["texts"]), position,
                   row.get("source_sha256"))
            # One answer per consultation; a later build of the same CFG
            # gives the same answer on the same premises, but keep the
            # admitted one if they differ (reported below).
            previous = rows.get(key)
            if previous is not None and previous["new_admitted"] != row["new_admitted"]:
                row = dict(row, divergent=True)
            rows[key] = row
    census = {}
    for position in sorted({key[3] for key in rows}):
        selected = [row for key, row in rows.items() if key[3] == position]
        reasons = collections.Counter(
            row["new_refusal"] for row in selected if not row["new_admitted"]
        )
        census[position] = {
            "consultations": len(selected),
            "admitted": sum(1 for row in selected if row["new_admitted"]),
            "refused": sum(1 for row in selected if not row["new_admitted"]),
            "divergent": sum(1 for row in selected if row.get("divergent")),
            "refusal_reasons": dict(reasons.most_common()),
            "admitted_functions": len({row["function"] for row in selected if row["new_admitted"]}),
        }

    def top_level_comma(texts):
        depth = 0
        for text in texts:
            if text in "([{":
                depth += 1
            elif text in ")]}":
                depth -= 1
            elif text == "," and depth == 0:
                return True
        return False

    comma_returns = [
        row for key, row in rows.items()
        if key[3] == "elaboration-return" and top_level_comma(row["texts"])
    ]
    census["comma_returns"] = {
        "rows": len(comma_returns),
        "functions": len({row["function"] for row in comma_returns}),
        "admitted": sum(1 for row in comma_returns if row["new_admitted"]),
        "refusal_reasons": dict(collections.Counter(
            row["new_refusal"] for row in comma_returns if not row["new_admitted"]
        ).most_common()),
    }
    print(json.dumps(census, indent=1, ensure_ascii=False))
    if args.json:
        args.json.write_text(json.dumps(census, indent=1, ensure_ascii=False) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
