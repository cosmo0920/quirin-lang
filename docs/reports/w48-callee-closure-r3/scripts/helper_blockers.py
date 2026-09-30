#!/usr/bin/env python3
"""W48-CALLEE-CLOSURE-R3: why a class-(iii) header callee's body publishes no
W4 summary.

Usage: helper_blockers.py RANKING_JSON PARITY_JSONL OUT_JSON [--top N]

A class-(iii) callee that a unit carries as a `static` header definition is
inside W4's derivation scope (gateway 4a) but is not a reported row, so the
diagnostics never say why its body fails to validate. The typed-path parity
stream (QCORE_TYPED_PATH_PARITY) does record every statement the body's
analysis consulted. For each external callee of the ranking (top N by
functions blocked), this collects the distinct refusals the parity stream
records for that function name: `elaboration-statement` rows whose old and
new routes both refused, and `nested-call` rows that refused, keyed by
(line, refusal). A body with no parity row is not analysed in any unit (no
definition carried, or out of W4 scope). This is a reading aid, not a
verdict: a statement another route admitted can still appear here, so each
named blocker is read in the source before anything is declared.
"""
import argparse
import collections
import json
import pathlib


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("ranking")
    parser.add_argument("parity")
    parser.add_argument("out")
    parser.add_argument("--top", type=int, default=60)
    args = parser.parse_args()
    ranking = json.loads(pathlib.Path(args.ranking).read_text())["ranking"]
    external = [r for r in ranking if r["class"] == "iii-external"]
    external.sort(key=lambda r: (-r["functions"], r["callee"]))
    names = {r["callee"]: r for r in external[: args.top]}
    refusals = collections.defaultdict(set)
    seen = collections.Counter()
    with open(args.parity) as handle:
        for line in handle:
            if '"function":"' not in line:
                continue
            row = json.loads(line)
            name = row.get("function")
            if name not in names:
                continue
            seen[name] += 1
            position = row.get("position")
            if position == "elaboration-statement" and not row.get("old_admitted") \
                    and not row.get("new_admitted") and row.get("old_refusal"):
                refusals[name].add((row.get("line"), row["old_refusal"]))
            elif position == "nested-call" and row.get("refusal"):
                refusals[name].add((row.get("line"), "nested: " + row["refusal"]))
    out = []
    for name, record in names.items():
        blockers = collections.Counter(reason for _, reason in refusals[name])
        out.append({"callee": name, "functions": record["functions"],
                    "alone": record["alone"], "plus_one": record["plus_callee"] + record["plus_leaf"],
                    "parity_rows": seen[name], "body_refusals": dict(blockers.most_common())})
    pathlib.Path(args.out).write_text(json.dumps(out, indent=1, sort_keys=True) + "\n")
    for row in out:
        print(f"{row['functions']:4} {row['alone']} {row['plus_one']} rows {row['parity_rows']:4}  {row['callee']}")
        for reason, count in list(row["body_refusals"].items())[:4]:
            print(f"        {count} x {reason[:110]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
