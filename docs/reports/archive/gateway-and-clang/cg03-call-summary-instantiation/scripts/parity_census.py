#!/usr/bin/env python3
"""CG03-CALL-SUMMARY-INSTANTIATION: census of the `call-instantiation` rows
of a `QCORE_TYPED_PATH_PARITY` stream.

usage: parity_census.py PARITY.jsonl [--json OUT] [--examples N]

Rows are de-duplicated on (function, line, texts, consumer, old/new answer):
the gateway re-parses a body for summary validation, which repeats a row.
Prints, per consumer, the admitted / refused counts and the refusal reasons
(normalised: a callee name in backticks becomes `_`), and the
older-admission disagreements by consumer, use and refusal.
"""

import argparse
import collections
import json
import re


def normalise(reason):
    if reason is None:
        return None
    reason = re.sub(r"`[^`]*`", "`_`", reason)
    reason = re.sub(r"formal \d+", "formal N", reason)
    return reason


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("parity")
    parser.add_argument("--json")
    parser.add_argument("--examples", type=int, default=0)
    args = parser.parse_args()
    seen = set()
    consulted = collections.defaultdict(collections.Counter)
    disagreements = collections.Counter()
    examples = collections.defaultdict(list)
    with open(args.parity) as stream:
        for line in stream:
            if '"call-instantiation"' not in line:
                continue
            row = json.loads(line)
            if row.get("position") != "call-instantiation":
                continue
            key = json.dumps(
                [row.get(k) for k in ("function", "line", "texts", "consumer", "callee",
                                      "new_admitted", "new_refusal")],
                sort_keys=True,
            )
            if key in seen:
                continue
            seen.add(key)
            consumer = row.get("consumer")
            if row.get("old_admitted"):
                disagreements[(consumer, row.get("use"), row.get("new_refusal"))] += 1
                continue
            outcome = "admitted" if row.get("new_admitted") else normalise(row.get("new_refusal"))
            consulted[consumer][outcome] += 1
            if len(examples[(consumer, outcome)]) < args.examples:
                examples[(consumer, outcome)].append(
                    (row.get("function"), " ".join(row.get("texts") or [])[:160])
                )
    record = {
        "consulted": {c: dict(v.most_common()) for c, v in consulted.items()},
        "older_admissions_declined": [
            {"consumer": c, "use": u, "refusal": r, "rows": n}
            for (c, u, r), n in disagreements.most_common()
        ],
    }
    print(json.dumps(record, indent=1))
    for (consumer, outcome), rows in sorted(examples.items(), key=str):
        print(f"== {consumer} / {outcome}")
        for function, text in rows:
            print(f"   {function}: {text}")
    if args.json:
        with open(args.json, "w") as out:
            json.dump(record, out, indent=1)
            out.write("\n")


if __name__ == "__main__":
    main()
